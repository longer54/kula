//! Interface to AI results (produced by a separate process): ai.sqlite file (read-only) and the `kula-ai` command.
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::process::Command;

pub fn ai_path() -> PathBuf {
    crate::db::data_dir().join("ai.sqlite")
}

pub fn open_ro() -> Option<Connection> {
    let p = ai_path();
    if !p.is_file() {
        return None;
    }
    Connection::open_with_flags(&p, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()
}

pub fn has_persons() -> bool {
    open_ro().map(|c| c.query_row("SELECT COUNT(*) FROM persons", [], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0).unwrap_or(false)
}

pub struct Person {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub cover_rel: String,
    pub bx: [f64; 4],
}

pub fn persons() -> Vec<Person> {
    let Some(c) = open_ro() else { return vec![] };
    let mut out = Vec::new();
    let sql = "SELECT p.id, COALESCE(p.name,''), (SELECT COUNT(DISTINCT rel_path) FROM faces f WHERE f.person_id=p.id), COALESCE(p.cover_face_id,0)
               FROM persons p WHERE COALESCE(p.hidden,0)=0 ORDER BY 3 DESC";
    let Ok(mut st) = c.prepare(sql) else { return out };
    let rows: Vec<(i64, String, i64, i64)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map(|x| x.flatten().collect()).unwrap_or_default();
    for (id, name, count, cover) in rows {
        if count == 0 {
            continue;
        }
        let face = c
            .query_row("SELECT rel_path,x,y,w,h FROM faces WHERE id=?1", [cover], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, f64>(3)?, r.get::<_, f64>(4)?)))
            .or_else(|_| c.query_row("SELECT rel_path,x,y,w,h FROM faces WHERE person_id=?1 LIMIT 1", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, f64>(3)?, r.get::<_, f64>(4)?))));
        if let Ok((rel, x, y, fw, fh)) = face {
            out.push(Person { id, name, count, cover_rel: rel, bx: [x, y, fw, fh] });
        }
    }
    out
}

pub fn person_rels(pid: i64) -> Vec<String> {
    let Some(c) = open_ro() else { return vec![] };
    let Ok(mut st) = c.prepare("SELECT DISTINCT rel_path FROM faces WHERE person_id=?1") else { return vec![] };
    st.query_map([pid], |r| r.get::<_, String>(0)).map(|x| x.flatten().collect()).unwrap_or_default()
}

fn which(cmd: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(h).join(".local/bin"));
    }
    dirs.into_iter().map(|d| d.join(cmd)).find(|p| p.is_file())
}

pub fn semantic_available() -> bool {
    which("kula-ai").is_some()
}

/// HOOK for semantic search: `kula-ai szukaj "text" --top 200` → JSON list of rel_path.
pub fn semantic_search(q: &str) -> Vec<String> {
    let Some(bin) = which("kula-ai") else { return vec![] };
    let Ok(out) = Command::new(bin).args(["szukaj", q, "--top", "200"]).output() else { return vec![] };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else { return vec![] };
    let arr = match &v {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(o) => o.get("results").or_else(|| o.get("wyniki")).and_then(|x| x.as_array()).cloned().unwrap_or_default(),
        _ => vec![],
    };
    arr.iter()
        .filter_map(|x| match x {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => o.get("rel_path").and_then(|s| s.as_str()).map(|s| s.to_string()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------- faces (round crops) and person editing
static FACE_Q: std::sync::Mutex<Vec<(PathBuf, PathBuf, [f64; 4])>> = std::sync::Mutex::new(Vec::new());
static FACE_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn face_dst(pid: i64, rel: &str, bx: [f64; 4]) -> PathBuf {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in rel.bytes().chain(bx[0].to_le_bytes()).chain(bx[1].to_le_bytes()).chain(bx[2].to_le_bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    crate::db::cache_dir().join("faces").join(format!("{}_{:08x}.png", pid, h as u32))
}

/// URL of a ready round face crop or "" (then it is generated in the background; the UI polls again).
pub fn face_url(pid: i64, rel: &str, bx: [f64; 4]) -> String {
    let dst = face_dst(pid, rel, bx);
    if dst.exists() {
        return crate::db::file_url(&dst);
    }
    let src = crate::db::source().join(rel);
    {
        let mut q = FACE_Q.lock().unwrap_or_else(|e| e.into_inner());
        if !q.iter().any(|x| x.1 == dst) {
            q.push((src, dst, bx));
        }
    }
    if !FACE_STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        for _ in 0..3 {
            std::thread::spawn(|| loop {
                let job = FACE_Q.lock().unwrap_or_else(|e| e.into_inner()).pop();
                match job {
                    Some((src, dst, bx)) => {
                        if let Some(d) = dst.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let tmp = dst.with_extension("tmp.png");
                        if crate::bridge::qobject::kula_make_face(&src.to_string_lossy(), &tmp.to_string_lossy(), bx[0], bx[1], bx[2], bx[3], 256) {
                            let _ = std::fs::rename(&tmp, &dst);
                        } else {
                            // not possible: save a marker so we don't keep retrying
                            let _ = std::fs::write(&dst, b"");
                        }
                    }
                    None => std::thread::sleep(Duration::from_millis(100)),
                }
            });
        }
    }
    String::new()
}

use std::time::Duration;

fn open_rw() -> Option<Connection> {
    let p = ai_path();
    if !p.is_file() {
        return None;
    }
    let bak = p.with_extension("sqlite.bak-kula");
    if !bak.exists() {
        let _ = std::fs::copy(&p, &bak);
    }
    let c = Connection::open(&p).ok()?;
    let _ = c.busy_timeout(Duration::from_secs(20));
    Some(c)
}

pub fn rename_person(pid: i64, name: &str) -> bool {
    let Some(c) = open_rw() else { return false };
    c.execute("UPDATE persons SET name=?1 WHERE id=?2", rusqlite::params![name.trim(), pid]).is_ok()
}

/// Merge persons: all faces go to the first one in the list (the largest by input order).
pub fn merge_persons(ids: &[i64]) -> i64 {
    if ids.len() < 2 {
        return 0;
    }
    let Some(c) = open_rw() else { return 0 };
    let target = ids[0];
    let mut moved = 0;
    let _ = c.execute_batch("BEGIN");
    // name: keep the first non-empty
    let mut name: String = c.query_row("SELECT COALESCE(name,'') FROM persons WHERE id=?1", [target], |r| r.get(0)).unwrap_or_default();
    for &o in &ids[1..] {
        if o == target {
            continue;
        }
        if name.is_empty() {
            name = c.query_row("SELECT COALESCE(name,'') FROM persons WHERE id=?1", [o], |r| r.get(0)).unwrap_or_default();
        }
        moved += c.execute("UPDATE faces SET person_id=?1 WHERE person_id=?2", rusqlite::params![target, o]).unwrap_or(0) as i64;
        let _ = c.execute("DELETE FROM persons WHERE id=?1", [o]);
    }
    let _ = c.execute("UPDATE persons SET name=?1 WHERE id=?2", rusqlite::params![name, target]);
    let _ = c.execute_batch("COMMIT");
    moved
}
