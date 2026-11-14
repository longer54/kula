//! Incremental scan: photo directory (read-only) + optional Takeout metadata directory (JSON).
//! Date order: EXIF DateTimeOriginal → JSON photoTakenTime → date from filename → mtime.
use crate::db;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering::Relaxed};
use walkdir::WalkDir;

pub static VIEW_VER: AtomicU64 = AtomicU64::new(1);
pub static PHASE: AtomicUsize = AtomicUsize::new(0); // 0 idle 1 JSON 2 review 3 read 4 end
pub static FOUND: AtomicUsize = AtomicUsize::new(0);
pub static TODO: AtomicUsize = AtomicUsize::new(0);
pub static DONE: AtomicUsize = AtomicUsize::new(0);
pub static REMOVED: AtomicUsize = AtomicUsize::new(0);
pub static JSON_FILES: AtomicUsize = AtomicUsize::new(0);
pub static JSON_MATCHED: AtomicUsize = AtomicUsize::new(0);
pub static D_EXIF: AtomicUsize = AtomicUsize::new(0);
pub static D_JSON: AtomicUsize = AtomicUsize::new(0);
pub static D_NAME: AtomicUsize = AtomicUsize::new(0);
pub static D_MTIME: AtomicUsize = AtomicUsize::new(0);
pub static MS: AtomicI64 = AtomicI64::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

const IMG: &[&str] = &["jpg", "jpeg", "png", "webp", "heic", "heif", "gif", "avif", "bmp", "tif", "tiff", "nef", "cr2", "cr3", "arw", "dng", "raf", "orf", "rw2", "jxl"];
const VID: &[&str] = &["mp4", "mov", "mkv", "avi", "3gp", "m4v", "webm", "mpg", "mpeg", "mts", "m2ts", "wmv", "mpeg4"];

pub fn running() -> bool {
    RUNNING.load(Relaxed)
}

pub fn start_background() {
    if RUNNING.swap(true, Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        run();
        RUNNING.store(false, Relaxed);
    });
}

#[derive(Default, Clone)]
pub struct Meta {
    pub taken: i64,
    pub w: i32,
    pub h: i32,
    pub dur: f64,
    pub cam: String,
    pub lens: String,
    pub fnum: f64,
    pub exposure: String,
    pub iso: i64,
    pub focal: f64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub descr: String,
    pub people: String,
    pub fav: bool,
    pub archived: bool,
    pub trashed: bool,
    pub takeout: bool,
    pub date_source: &'static str,
    pub date_quality: &'static str,
}

struct Entry {
    rel: String,
    abs: PathBuf,
    folder: String,
    name: String,
    size: i64,
    mtime: i64,
    kind: i64,
}

/// Epoch seconds → "wall time" (local clock stored as UTC).
pub fn wall(ts: i64) -> i64 {
    Local.timestamp_opt(ts, 0).single().map(|d| d.naive_local().and_utc().timestamp()).unwrap_or(ts)
}

// ---------------------------------------------------------------- date from filename
fn all_digits(b: &[u8]) -> bool {
    !b.is_empty() && b.iter().all(|c| c.is_ascii_digit())
}
fn num(b: &[u8]) -> i64 {
    std::str::from_utf8(b).ok().and_then(|s| s.parse().ok()).unwrap_or(-1)
}

fn time_after(b: &[u8], i: usize) -> (u32, u32, u32) {
    // after the date: optional separator (_ - . space T) and HHMMSS or HH-MM-SS / HH.MM.SS
    let mut j = i;
    if j < b.len() && matches!(b[j], b'_' | b'-' | b' ' | b'T' | b'.') {
        j += 1;
    } else {
        return (12, 0, 0);
    }
    if j + 6 <= b.len() && all_digits(&b[j..j + 6]) && !(j + 6 < b.len() && b[j + 6].is_ascii_digit()) {
        let (h, m, s) = (num(&b[j..j + 2]), num(&b[j + 2..j + 4]), num(&b[j + 4..j + 6]));
        if h < 24 && m < 60 && s < 60 {
            return (h as u32, m as u32, s as u32);
        }
    }
    if j + 8 <= b.len() && all_digits(&b[j..j + 2]) && matches!(b[j + 2], b'-' | b'.' | b'_' | b':') && all_digits(&b[j + 3..j + 5]) && matches!(b[j + 5], b'-' | b'.' | b'_' | b':') && all_digits(&b[j + 6..j + 8]) {
        let (h, m, s) = (num(&b[j..j + 2]), num(&b[j + 3..j + 5]), num(&b[j + 6..j + 8]));
        if h < 24 && m < 60 && s < 60 {
            return (h as u32, m as u32, s as u32);
        }
    }
    (12, 0, 0)
}

/// Returns (time, source): "name" (IMG_YYYYMMDD_HHMMSS, Screenshot_…, signal-YYYY-MM-DD-HHMMSS, YYYY-MM-DD),
/// "name_whatsapp" (IMG-YYYYMMDD-WA…, day only = date received), "name_ms" (13 digits = Unix ms, file save date).
pub fn date_from_name(name: &str) -> Option<(i64, &'static str)> {
    let b = name.as_bytes();
    let n = b.len();
    let ok_year = |y: i64| (1990..=2100).contains(&y);
    let mk = |y: i64, mo: i64, d: i64, t: (u32, u32, u32)| -> Option<i64> {
        NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)?.and_hms_opt(t.0, t.1, t.2).map(|dt| dt.and_utc().timestamp())
    };
    let lower = name.to_lowercase();
    let whatsapp = lower.contains("-wa") && (lower.starts_with("img-") || lower.starts_with("vid-") || lower.starts_with("ptt-") || lower.starts_with("aud-") || lower.starts_with("doc-"));
    let mut i = 0;
    while i + 8 <= n {
        if all_digits(&b[i..i + 8]) && (i == 0 || !b[i - 1].is_ascii_digit()) && (i + 8 == n || !b[i + 8].is_ascii_digit()) {
            let (y, mo, d) = (num(&b[i..i + 4]), num(&b[i + 4..i + 6]), num(&b[i + 6..i + 8]));
            if ok_year(y) {
                if let Some(t) = mk(y, mo, d, time_after(b, i + 8)) {
                    return Some((t, if whatsapp { "nazwa_whatsapp" } else { "nazwa" }));
                }
            }
        }
        if i + 10 <= n && all_digits(&b[i..i + 4]) && matches!(b[i + 4], b'-' | b'_' | b'.') && all_digits(&b[i + 5..i + 7]) && b[i + 7] == b[i + 4] && all_digits(&b[i + 8..i + 10]) && (i == 0 || !b[i - 1].is_ascii_digit()) {
            let (y, mo, d) = (num(&b[i..i + 4]), num(&b[i + 5..i + 7]), num(&b[i + 8..i + 10]));
            if ok_year(y) {
                if let Some(t) = mk(y, mo, d, time_after(b, i + 10)) {
                    return Some((t, "nazwa"));
                }
            }
        }
        i += 1;
    }
    let mut i = 0;
    while i + 13 <= n {
        if b[i] == b'1' && all_digits(&b[i..i + 13]) && (i == 0 || !b[i - 1].is_ascii_digit()) && (i + 13 == n || !b[i + 13].is_ascii_digit()) {
            let ms = num(&b[i..i + 13]);
            if (631_152_000_000..4_102_444_800_000).contains(&ms) {
                return Some((wall(ms / 1000), "nazwa_ms"));
            }
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------- Takeout JSON
#[derive(Clone)]
struct J {
    n: u32,
    folder: String,
    taken: Option<i64>,
    descr: String,
    lat: Option<f64>,
    lon: Option<f64>,
    people: String,
    fav: bool,
    arch: bool,
    trash: bool,
}

struct JIndex {
    by_title: HashMap<String, Vec<J>>,
    album_titles: HashMap<String, String>, // folder name → album title (from metadane.json)
    sig: String,
}

fn trailing_n(s: &str) -> (&str, u32) {
    // "xxx(12)" → ("xxx", 12)
    if s.ends_with(')') {
        if let Some(p) = s.rfind('(') {
            if let Ok(n) = s[p + 1..s.len() - 1].parse::<u32>() {
                return (&s[..p], n);
            }
        }
    }
    (s, 0)
}

fn parse_json(path: &Path) -> Option<(String, J)> {
    let txt = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&txt).ok()?;
    let fname = path.file_name()?.to_string_lossy().to_string();
    let stem = fname.strip_suffix(".json").unwrap_or(&fname);
    let (_, n) = trailing_n(stem);
    let title = v.get("title").and_then(|t| t.as_str()).map(|s| s.to_string()).unwrap_or_else(|| {
        let (st, _) = trailing_n(stem);
        match st.find(".supplemental-metadat") {
            Some(p) => st[..p].to_string(),
            None => st.to_string(),
        }
    });
    let folder = path.parent()?.file_name()?.to_string_lossy().to_string();
    let ts = |k: &str| -> Option<i64> {
        v.get(k)?.get("timestamp")?.as_str()?.parse::<i64>().ok().filter(|t| *t > 0)
    };
    let taken = ts("photoTakenTime").or_else(|| ts("creationTime")).map(wall);
    let geo = |k: &str| -> Option<(f64, f64)> {
        let g = v.get(k)?;
        let (la, lo) = (g.get("latitude")?.as_f64()?, g.get("longitude")?.as_f64()?);
        (la != 0.0 || lo != 0.0).then_some((la, lo))
    };
    let g = geo("geoData").or_else(|| geo("geoDataExif"));
    let people = v
        .get("people")
        .and_then(|p| p.as_array())
        .map(|a| a.iter().filter_map(|x| x.get("name").and_then(|n| n.as_str())).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let flag = |k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    Some((
        title,
        J {
            n,
            folder,
            taken,
            descr: v.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
            lat: g.map(|x| x.0),
            lon: g.map(|x| x.1),
            people,
            fav: flag("favorited"),
            arch: flag("archived"),
            trash: flag("trashed"),
        },
    ))
}

fn build_jindex(dir: &Path) -> JIndex {
    let mut idx = JIndex { by_title: HashMap::new(), album_titles: HashMap::new(), sig: String::new() };
    if dir.as_os_str().is_empty() || !dir.is_dir() {
        return idx;
    }
    let mut files: Vec<PathBuf> = Vec::new();
    let mut max_m = 0i64;
    for e in WalkDir::new(dir).into_iter().flatten() {
        if e.file_type().is_file() && e.path().extension().map(|x| x.eq_ignore_ascii_case("json")).unwrap_or(false) {
            if let Ok(m) = e.metadata() {
                max_m = max_m.max(m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0));
            }
            files.push(e.into_path());
        }
    }
    idx.sig = format!("{}|{}|{}", dir.display(), files.len(), max_m);
    JSON_FILES.store(files.len(), Relaxed);
    let threads = 8;
    let chunk = (files.len() / threads).max(1) + 1;
    let results: Vec<Vec<(String, J)>> = std::thread::scope(|s| {
        let hs: Vec<_> = files
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    let mut out = Vec::new();
                    for p in c {
                        let nm = p.file_name().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                        if nm == "metadane.json" || nm == "metadata.json" {
                            out.push((format!("\u{1}{}", p.display()), J { n: 0, folder: String::new(), taken: None, descr: String::new(), lat: None, lon: None, people: String::new(), fav: false, arch: false, trash: false }));
                            continue;
                        }
                        if let Some(x) = parse_json(p) {
                            out.push(x);
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    for (t, j) in results.into_iter().flatten() {
        if let Some(p) = t.strip_prefix('\u{1}') {
            let pb = PathBuf::from(p);
            if let (Some(folder), Ok(txt)) = (pb.parent().and_then(|x| x.file_name()), std::fs::read_to_string(&pb)) {
                if let Some(title) = serde_json::from_str::<serde_json::Value>(&txt).ok().and_then(|v| v.get("title").and_then(|x| x.as_str()).map(|s| s.to_string())) {
                    if !title.is_empty() {
                        idx.album_titles.insert(folder.to_string_lossy().to_string(), title);
                    }
                }
            }
            continue;
        }
        idx.by_title.entry(t.to_lowercase()).or_default().push(j);
    }
    idx
}

fn pick<'a>(v: &'a [J], n: u32, folder: &str) -> Option<&'a J> {
    let same: Vec<&J> = v.iter().filter(|j| j.n == n).collect();
    let pool: Vec<&J> = if same.is_empty() { v.iter().collect() } else { same };
    pool.iter().find(|j| j.folder == folder).copied().or_else(|| pool.first().copied())
}

/// Matching by filename (title in JSON), handling suffixes (n), "-edited/-edytowane".
fn lookup<'a>(idx: &'a JIndex, name: &str, parent: &str) -> Option<&'a J> {
    if idx.by_title.is_empty() {
        return None;
    }
    let lname = name.to_lowercase();
    let (stem, ext) = match lname.rfind('.') {
        Some(p) => (&lname[..p], &lname[p..]),
        None => (&lname[..], ""),
    };
    let parent_l = parent;
    if let Some(v) = idx.by_title.get(&lname) {
        if let Some(j) = pick(v, 0, parent_l) {
            return Some(j);
        }
    }
    let (bs, n) = trailing_n(stem);
    if n > 0 {
        if let Some(v) = idx.by_title.get(&format!("{bs}{ext}")) {
            if let Some(j) = pick(v, n, parent_l) {
                return Some(j);
            }
        }
    }
    for suf in ["-edited", "-edytowane", "-bearbeitet"] {
        if let Some(b2) = bs.strip_suffix(suf) {
            if let Some(v) = idx.by_title.get(&format!("{b2}{ext}")) {
                if let Some(j) = pick(v, 0, parent_l) {
                    return Some(j);
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------- EXIF / video
fn ascii_of(ex: &exif::Exif, t: exif::Tag) -> Option<String> {
    match &ex.get_field(t, exif::In::PRIMARY)?.value {
        exif::Value::Ascii(v) => {
            let s = v.iter().map(|b| String::from_utf8_lossy(b).trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string()).collect::<Vec<_>>().join(" ");
            (!s.is_empty()).then_some(s)
        }
        _ => None,
    }
}
fn rat_of(ex: &exif::Exif, t: exif::Tag) -> Option<(f64, u32, u32)> {
    match &ex.get_field(t, exif::In::PRIMARY)?.value {
        exif::Value::Rational(v) if !v.is_empty() && v[0].denom != 0 => Some((v[0].to_f64(), v[0].num, v[0].denom)),
        _ => None,
    }
}
fn gps(ex: &exif::Exif, t: exif::Tag, r: exif::Tag) -> Option<f64> {
    let f = ex.get_field(t, exif::In::PRIMARY)?;
    let v = match &f.value {
        exif::Value::Rational(v) if v.len() >= 3 && v.iter().all(|x| x.denom != 0) => v.iter().map(|x| x.to_f64()).collect::<Vec<_>>(),
        _ => return None,
    };
    let mut d = v[0] + v[1] / 60.0 + v[2] / 3600.0;
    if let Some(rf) = ascii_of(ex, r) {
        if rf.starts_with('S') || rf.starts_with('W') {
            d = -d;
        }
    }
    Some(d)
}

fn read_exif(path: &Path, m: &mut Meta) -> bool {
    let Ok(f) = std::fs::File::open(path) else { return false };
    let mut br = std::io::BufReader::new(f);
    let Ok(ex) = exif::Reader::new().read_from_container(&mut br) else { return false };
    use exif::Tag;
    let dt = ascii_of(&ex, Tag::DateTimeOriginal).or_else(|| ascii_of(&ex, Tag::DateTimeDigitized)).or_else(|| ascii_of(&ex, Tag::DateTime));
    let mut got_date = false;
    if let Some(s) = dt {
        if let Ok(n) = NaiveDateTime::parse_from_str(s.trim(), "%Y:%m:%d %H:%M:%S") {
            if n.year() >= 1990 {
                m.taken = n.and_utc().timestamp();
                got_date = true;
            }
        }
    }
    let make = ascii_of(&ex, Tag::Make).unwrap_or_default();
    let model = ascii_of(&ex, Tag::Model).unwrap_or_default();
    m.cam = if !model.is_empty() && !make.is_empty() && !model.to_lowercase().starts_with(&make.to_lowercase()) { format!("{make} {model}") } else if !model.is_empty() { model } else { make };
    m.lens = ascii_of(&ex, Tag::LensModel).unwrap_or_default();
    if let Some((v, _, _)) = rat_of(&ex, Tag::FNumber) {
        m.fnum = v;
    }
    if let Some((v, num, den)) = rat_of(&ex, Tag::ExposureTime) {
        m.exposure = if v >= 1.0 { format!("{}", (v * 10.0).round() / 10.0) } else if num == 1 { format!("1/{den}") } else { format!("1/{}", (1.0 / v).round() as i64) };
    }
    if let Some(f) = ex.get_field(Tag::PhotographicSensitivity, exif::In::PRIMARY) {
        if let Some(i) = f.value.get_uint(0) {
            m.iso = i as i64;
        }
    }
    if let Some((v, _, _)) = rat_of(&ex, Tag::FocalLength) {
        m.focal = v;
    }
    if let (Some(la), Some(lo)) = (gps(&ex, Tag::GPSLatitude, Tag::GPSLatitudeRef), gps(&ex, Tag::GPSLongitude, Tag::GPSLongitudeRef)) {
        if la != 0.0 || lo != 0.0 {
            m.lat = Some(la);
            m.lon = Some(lo);
        }
    }
    got_date
}

fn probe_video(path: &Path, m: &mut Meta) {
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height:stream_side_data=rotation:format=duration", "-of", "json"])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .output();
    let Ok(o) = out else { return };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) else { return };
    if let Some(s) = v.get("streams").and_then(|s| s.get(0)) {
        let (mut w, mut h) = (s.get("width").and_then(|x| x.as_i64()).unwrap_or(0), s.get("height").and_then(|x| x.as_i64()).unwrap_or(0));
        let rot = s.get("side_data_list").and_then(|l| l.get(0)).and_then(|d| d.get("rotation")).and_then(|r| r.as_f64()).unwrap_or(0.0).abs() as i64;
        if rot == 90 || rot == 270 {
            std::mem::swap(&mut w, &mut h);
        }
        m.w = w as i32;
        m.h = h as i32;
    }
    m.dur = v.get("format").and_then(|f| f.get("duration")).and_then(|d| d.as_str()).and_then(|d| d.parse::<f64>().ok()).unwrap_or(0.0);
}

fn process(e: &Entry, jx: &JIndex) -> Meta {
    let mut m = Meta::default();
    let mut exif_date = false;
    if e.kind == 0 {
        exif_date = read_exif(&e.abs, &mut m);
        let (mut w, mut h) = (0i32, 0i32);
        if crate::bridge::qobject::kula_probe(&e.abs.to_string_lossy(), &mut w, &mut h) {
            m.w = w;
            m.h = h;
        }
    } else {
        probe_video(&e.abs, &mut m);
    }
    let parent = e.folder.rsplit('/').next().unwrap_or("");
    let j = lookup(jx, &e.name, parent);
    if let Some(j) = j {
        JSON_MATCHED.fetch_add(1, Relaxed);
        m.takeout = true;
        m.descr = j.descr.clone();
        m.people = j.people.clone();
        m.fav = j.fav;
        m.archived = j.arch;
        m.trashed = j.trash;
        if m.lat.is_none() {
            m.lat = j.lat;
            m.lon = j.lon;
        }
    }
    if exif_date && m.taken > 0 {
        D_EXIF.fetch_add(1, Relaxed);
        m.date_source = "exif";
        m.date_quality = "pewna";
    } else if let Some(t) = j.and_then(|j| j.taken) {
        m.taken = t;
        D_JSON.fetch_add(1, Relaxed);
        m.date_source = "json";
        m.date_quality = "pewna";
    } else if let Some((t, src)) = date_from_name(&e.name) {
        m.taken = t;
        D_NAME.fetch_add(1, Relaxed);
        m.date_source = src;
        m.date_quality = if src == "nazwa" { "pewna" } else { "przybliżona" };
    } else {
        // mtime is useless (copying) — "No date"
        m.taken = 0;
        D_MTIME.fetch_add(1, Relaxed);
        m.date_source = "brak";
        m.date_quality = "brak";
    }
    if m.w <= 0 || m.h <= 0 {
        m.w = 1600;
        m.h = 1200;
    }
    m
}

// ---------------------------------------------------------------- scan
fn is_year_folder(s: &str) -> bool {
    let l = s.to_lowercase();
    let tail = |p: &str| l.strip_prefix(p).map(|r| r.trim().len() == 4 && r.trim().chars().all(|c| c.is_ascii_digit())).unwrap_or(false);
    tail("zdjęcia z roku") || tail("photos from")
}

fn walk(src: &Path) -> Vec<Entry> {
    let mut v = Vec::new();
    let it = WalkDir::new(src).follow_links(false).into_iter().filter_entry(|e| {
        // hidden directories/files (phone thumbnail caches, .stversions, Syncthing's .stfolder) are skipped on purpose
        e.depth() == 0 || { let n = e.file_name().to_string_lossy(); !n.starts_with('.') && !n.starts_with("~syncthing~") }
    });
    for e in it.flatten() {
        if !e.file_type().is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        let kind = if IMG.contains(&ext.as_str()) {
            0
        } else if VID.contains(&ext.as_str()) {
            1
        } else {
            continue;
        };
        let Ok(md) = e.metadata() else { continue };
        let rel = match e.path().strip_prefix(src) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };
        let folder = rel.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
        let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
        v.push(Entry { rel, abs: e.into_path(), folder, name, size: md.len() as i64, mtime, kind });
    }
    FOUND.store(v.len(), Relaxed);
    v
}

fn album_of(rel: &str) -> Option<String> {
    let comps: Vec<&str> = rel.split('/').collect();
    for (i, c) in comps.iter().enumerate() {
        let l = c.to_lowercase();
        if (l == "zdjęcia google" || l == "google photos") && i + 2 < comps.len() {
            let sub = comps[i + 1];
            let sl = sub.to_lowercase();
            if is_year_folder(sub) || sl == "kosz" || sl == "trash" || sl == "folder zamknięty" || sl == "locked folder" {
                return None;
            }
            return Some(sub.to_string());
        }
    }
    None
}

fn special_folder(rel: &str) -> (bool, bool) {
    // (trash, closed)
    let comps: Vec<String> = rel.split('/').map(|s| s.to_lowercase()).collect();
    (comps.iter().any(|c| c == "kosz" || c == "trash"), comps.iter().any(|c| c == "folder zamknięty" || c == "locked folder"))
}

pub fn run() {
    let t0 = std::time::Instant::now();
    for a in [&FOUND, &TODO, &DONE, &REMOVED, &JSON_FILES, &JSON_MATCHED, &D_EXIF, &D_JSON, &D_NAME, &D_MTIME] {
        a.store(0, Relaxed);
    }
    let conn = db::open();
    let src = db::resolve_source(&conn);
    *db::SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = Some(src.clone());
    if !src.is_dir() {
        eprintln!("kula: source directory does not exist: {}", src.display());
        PHASE.store(0, Relaxed);
        return;
    }
    PHASE.store(1, Relaxed);
    let metadir = db::resolve_meta(&conn);
    let jx = build_jindex(&metadir);
    let forced = db::get_setting(&conn, "meta_sig").unwrap_or_default() != jx.sig;

    PHASE.store(2, Relaxed);
    let entries = walk(&src);

    let mut existing: HashMap<String, (i64, i64)> = HashMap::new();
    if let Ok(mut st) = conn.prepare("SELECT rel,size,mtime FROM photos WHERE trashed<>2") {
        if let Ok(rows) = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))) {
            for (rel, s, m) in rows.flatten() {
                existing.insert(rel, (s, m));
            }
        }
    }
    let todo: Vec<&Entry> = entries.iter().filter(|e| forced || existing.get(&e.rel).map(|x| *x != (e.size, e.mtime)).unwrap_or(true)).collect();
    TODO.store(todo.len(), Relaxed);
    PHASE.store(3, Relaxed);

    let now = db::now();
    let (tx, rx) = std::sync::mpsc::sync_channel::<(usize, Meta)>(512);
    let next = AtomicUsize::new(0);
    let nthreads = 12;
    std::thread::scope(|s| {
        for _ in 0..nthreads {
            let tx = tx.clone();
            let (todo, next, jx) = (&todo, &next, &jx);
            s.spawn(move || loop {
                let i = next.fetch_add(1, Relaxed);
                if i >= todo.len() {
                    break;
                }
                let m = process(todo[i], jx);
                if tx.send((i, m)).is_err() {
                    break;
                }
            });
        }
        drop(tx);
        let conn2 = db::open();
        let _ = conn2.execute_batch("BEGIN");
        let mut n = 0usize;
        for (i, m) in rx {
            let e = todo[i];
            let (kosz, zam) = special_folder(&e.rel);
            {
                let r = conn2.execute(
                    "INSERT INTO photos(rel,folder,name,size,mtime,kind,taken,w,h,dur,cam,lens,fnum,exposure,iso,focal,lat,lon,descr,people,fav,archived,trashed,added,takeout,date_source,date_quality)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27)
                     ON CONFLICT(rel) DO UPDATE SET folder=?2,name=?3,size=?4,mtime=?5,kind=?6,taken=?7,w=?8,h=?9,dur=?10,cam=?11,lens=?12,fnum=?13,exposure=?14,iso=?15,focal=?16,lat=?17,lon=?18,people=?20,takeout=?25,date_source=?26,date_quality=?27,thumb=0,
                     descr=CASE WHEN photos.descr='' THEN ?19 ELSE photos.descr END",
                    params![e.rel, e.folder, e.name, e.size, e.mtime, e.kind, m.taken, m.w, m.h, m.dur, m.cam, m.lens, m.fnum, m.exposure, m.iso, m.focal, m.lat, m.lon, m.descr, m.people, (m.fav) as i32, (m.archived || zam) as i32, (m.trashed || kosz) as i32, now, m.takeout as i32, m.date_source, m.date_quality],
                );
                if let Err(er) = r {
                    eprintln!("kula: zapis {}: {er}", e.rel);
                }
            }
            n += 1;
            DONE.store(n, Relaxed);
            if n % 400 == 0 {
                let _ = conn2.execute_batch("COMMIT; BEGIN");
                VIEW_VER.fetch_add(1, Relaxed);
            }
        }
        let _ = conn2.execute_batch("COMMIT");
    });

    PHASE.store(4, Relaxed);
    // removal of vanished files — only when the scan looks reliable (e.g. NFS not unmounted)
    let present: HashSet<&str> = entries.iter().map(|e| e.rel.as_str()).collect();
    let gone: Vec<&String> = existing.keys().filter(|r| !present.contains(r.as_str())).collect();
    if !gone.is_empty() && entries.len() * 2 >= existing.len() {
        let _ = conn.execute_batch("BEGIN");
        for r in &gone {
            let _ = conn.execute("DELETE FROM photos WHERE rel=?1", [r]);
        }
        let _ = conn.execute_batch("COMMIT");
        REMOVED.store(gone.len(), Relaxed);
    } else if !gone.is_empty() {
        eprintln!("kula: skipped removing {} vanished entries (scan unreliable: {} of {})", gone.len(), entries.len(), existing.len());
    }
    db::set_setting(&conn, "meta_sig", &jx.sig);
    rebuild_takeout_albums(&conn, &entries, &jx);
    MS.store(t0.elapsed().as_millis() as i64, Relaxed);
    VIEW_VER.fetch_add(1, Relaxed);
    crate::thumbs::queue_background(&conn);
    PHASE.store(0, Relaxed);
}

fn rebuild_takeout_albums(conn: &Connection, entries: &[Entry], jx: &JIndex) {
    let mut albums: HashMap<String, Vec<&str>> = HashMap::new();
    for e in entries {
        if let Some(a) = album_of(&e.rel) {
            let title = jx.album_titles.get(&a).cloned().unwrap_or(a);
            albums.entry(title).or_default().push(&e.rel);
        }
    }
    let _ = conn.execute_batch("BEGIN");
    let mut keep: Vec<i64> = Vec::new();
    for (title, rels) in &albums {
        let id: Option<i64> = conn.query_row("SELECT id FROM albums WHERE kind='takeout' AND title=?1", [title], |r| r.get(0)).ok();
        let id = match id {
            Some(i) => i,
            None => {
                let _ = conn.execute("INSERT INTO albums(title,kind,cover_rel,created) VALUES(?1,'takeout',?2,?3)", params![title, rels[0], db::now()]);
                conn.last_insert_rowid()
            }
        };
        keep.push(id);
        let _ = conn.execute("DELETE FROM album_items WHERE album_id=?1", [id]);
        for r in rels {
            let _ = conn.execute("INSERT OR IGNORE INTO album_items(album_id,rel) VALUES(?1,?2)", params![id, r]);
        }
    }
    let ids: Vec<i64> = conn.prepare("SELECT id FROM albums WHERE kind='takeout'").and_then(|mut s| s.query_map([], |r| r.get(0)).map(|x| x.flatten().collect())).unwrap_or_default();
    for id in ids {
        if !keep.contains(&id) {
            let _ = conn.execute("DELETE FROM album_items WHERE album_id=?1", [id]);
            let _ = conn.execute("DELETE FROM albums WHERE id=?1", [id]);
        }
    }
    let _ = conn.execute_batch("COMMIT");
}

#[allow(dead_code)]
pub fn stats_line() -> String {
    format!(
        "found={} to_read={} json={} matched={} date(exif/json/name/none)={}/{}/{}/{} removed={} ms={}",
        FOUND.load(Relaxed), TODO.load(Relaxed), JSON_FILES.load(Relaxed), JSON_MATCHED.load(Relaxed), D_EXIF.load(Relaxed), D_JSON.load(Relaxed), D_NAME.load(Relaxed), D_MTIME.load(Relaxed), REMOVED.load(Relaxed), MS.load(Relaxed)
    )
}
