//! Thumbnails: thread pool; priority = what the UI asks for; the rest in the background (newest first).
use crate::db;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::Duration;

pub const THUMB_SIZE: i32 = 512;
static PRIO: Mutex<Vec<i64>> = Mutex::new(Vec::new());
static BG: Mutex<VecDeque<i64>> = Mutex::new(VecDeque::new());
static INFLIGHT: Mutex<Option<HashSet<i64>>> = Mutex::new(None);
pub static TOTAL: AtomicUsize = AtomicUsize::new(0);
pub static DONE: AtomicUsize = AtomicUsize::new(0);
pub static FAILED: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static STARTED: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn fnv(rel: &str, mtime: i64, size: i64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in rel.bytes().chain(mtime.to_le_bytes()).chain(size.to_le_bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn thumb_path(rel: &str, mtime: i64, size: i64) -> PathBuf {
    let h = fnv(rel, mtime, size);
    db::cache_dir().join("thumbs").join(format!("{:02x}", h >> 56)).join(format!("{:016x}.jpg", h))
}

/// UI thumbnail request (LIFO — what the user sees right now).
pub fn want(id: i64) {
    let mut p = lock(&PRIO);
    if !p.contains(&id) {
        p.push(id);
        if p.len() > 160 {
            p.remove(0); // drop the oldest requests (already scrolled past)
        }
    }
}

pub fn queue_background(conn: &Connection) {
    let mut v: VecDeque<i64> = VecDeque::new();
    if let Ok(mut st) = conn.prepare("SELECT id, rel, mtime, size FROM photos WHERE thumb!=2 ORDER BY taken DESC") {
        if let Ok(rows) = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))) {
            for (id, rel, mt, sz) in rows.flatten() {
                if !thumb_path(&rel, mt, sz).exists() {
                    v.push_back(id);
                }
            }
        }
    }
    DONE.store(0, Relaxed);
    FAILED.store(0, Relaxed);
    TOTAL.store(v.len(), Relaxed);
    *lock(&BG) = v;
}

fn ffmpeg_thumb(src: &Path, tmp: &Path, video: bool) -> bool {
    let run = |seek: Option<&str>| -> bool {
        let _ = std::fs::remove_file(tmp);
        let mut c = Command::new("ffmpeg");
        c.args(["-nostdin", "-v", "error", "-threads", "1", "-y"]);
        if let Some(s) = seek {
            c.args(["-ss", s]);
        }
        c.arg("-i").arg(src);
        c.args(["-frames:v", "1", "-vf", &format!("scale={0}:{0}:force_original_aspect_ratio=decrease", THUMB_SIZE), "-q:v", "4"]);
        c.arg(tmp).stdout(Stdio::null()).stderr(Stdio::null());
        c.status().map(|s| s.success()).unwrap_or(false) && tmp.metadata().map(|m| m.len() > 0).unwrap_or(false)
    };
    if video {
        run(Some("1")) || run(None)
    } else {
        run(None)
    }
}

pub fn generate(abs: &Path, kind: i64, dst: &Path) -> bool {
    if let Some(d) = dst.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = dst.with_extension("tmp.jpg");
    let mut ok = false;
    if kind == 0 {
        ok = crate::bridge::qobject::kula_make_thumb(&abs.to_string_lossy(), &tmp.to_string_lossy(), THUMB_SIZE);
    }
    if !ok {
        ok = ffmpeg_thumb(abs, &tmp, kind == 1);
    }
    if ok {
        ok = std::fs::rename(&tmp, dst).is_ok();
    }
    if !ok {
        let _ = std::fs::remove_file(&tmp);
    }
    ok
}

fn work_one(conn: &Connection, id: i64, src: &Path) {
    let row = conn.query_row("SELECT rel, mtime, size, kind FROM photos WHERE id=?1", [id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
    });
    let Ok((rel, mt, sz, kind)) = row else { return };
    let dst = thumb_path(&rel, mt, sz);
    if dst.exists() {
        return;
    }
    if generate(&src.join(&rel), kind, &dst) {
        let _ = conn.execute("UPDATE photos SET thumb=1 WHERE id=?1", [id]);
    } else {
        FAILED.fetch_add(1, Relaxed);
        let _ = conn.execute("UPDATE photos SET thumb=2 WHERE id=?1", [id]);
    }
}

/// Number of thumbnail threads: 1/4 of cores, 1..=3 — so the computer doesn't lag.
fn worker_count(_requested: usize) -> usize {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    std::env::var("KULA_THUMB_WORKERS").ok().and_then(|s| s.parse().ok()).unwrap_or((cores / 4).clamp(1, 3))
}

pub fn start_workers(n: usize) {
    if STARTED.swap(true, Relaxed) {
        return;
    }
    let n = worker_count(n);
    *lock(&INFLIGHT) = Some(HashSet::new());
    for _ in 0..n.max(1) {
        std::thread::spawn(|| {
            // Linux: per-thread niceness (ffmpeg processes inherit it too)
            unsafe {
                libc::setpriority(libc::PRIO_PROCESS, 0, 15);
            }
            let conn = db::open();
            loop {
                let next = { lock(&PRIO).pop() }.map(|i| (i, false)).or_else(|| lock(&BG).pop_front().map(|i| (i, true)));
                let Some((id, bg)) = next else {
                    std::thread::sleep(Duration::from_millis(120));
                    continue;
                };
                ACTIVE.fetch_add(1, Relaxed);
                let fresh = lock(&INFLIGHT).as_mut().map(|s| s.insert(id)).unwrap_or(true);
                if fresh {
                    let src = db::source();
                    work_one(&conn, id, &src);
                    if let Some(s) = lock(&INFLIGHT).as_mut() {
                        s.remove(&id);
                    }
                }
                if bg {
                    DONE.fetch_add(1, Relaxed);
                }
                ACTIVE.fetch_sub(1, Relaxed);
            }
        });
    }
}

pub fn idle() -> bool {
    ACTIVE.load(Relaxed) == 0 && lock(&PRIO).is_empty() && lock(&BG).is_empty()
}

pub fn thumb_exists(rel: &str, mtime: i64, size: i64) -> Option<PathBuf> {
    let p = thumb_path(rel, mtime, size);
    p.exists().then_some(p)
}
