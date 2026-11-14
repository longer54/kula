//! SQLite index (local in ~/.local/share/kula) and paths.
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub static SOURCE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match std::env::var(var) {
        Ok(d) if d.starts_with('/') => PathBuf::from(d),
        _ => home().join(fallback),
    }
}

pub fn data_dir() -> PathBuf {
    match std::env::var("KULA_DATA") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => xdg("XDG_DATA_HOME", ".local/share").join("kula"),
    }
}

pub fn cache_dir() -> PathBuf {
    match std::env::var("KULA_CACHE") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => xdg("XDG_CACHE_HOME", ".cache").join("kula"),
    }
}

pub fn db_path() -> PathBuf {
    data_dir().join("kula.sqlite")
}

pub fn open() -> Connection {
    let _ = std::fs::create_dir_all(data_dir());
    let conn = Connection::open(db_path()).expect("cannot open the SQLite index");
    let _ = conn.busy_timeout(std::time::Duration::from_secs(20));
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    conn.execute_batch(SCHEMA).expect("SQLite schema");
    conn
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS photos(
  id INTEGER PRIMARY KEY, rel TEXT NOT NULL UNIQUE, folder TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '',
  size INTEGER NOT NULL DEFAULT 0, mtime INTEGER NOT NULL DEFAULT 0, kind INTEGER NOT NULL DEFAULT 0,
  taken INTEGER NOT NULL DEFAULT 0, w INTEGER NOT NULL DEFAULT 0, h INTEGER NOT NULL DEFAULT 0, dur REAL NOT NULL DEFAULT 0,
  cam TEXT NOT NULL DEFAULT '', lens TEXT NOT NULL DEFAULT '', fnum REAL NOT NULL DEFAULT 0, exposure TEXT NOT NULL DEFAULT '',
  iso INTEGER NOT NULL DEFAULT 0, focal REAL NOT NULL DEFAULT 0, lat REAL, lon REAL,
  descr TEXT NOT NULL DEFAULT '', people TEXT NOT NULL DEFAULT '', fav INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, trashed INTEGER NOT NULL DEFAULT 0,
  date_source TEXT NOT NULL DEFAULT '', date_quality TEXT NOT NULL DEFAULT '', added INTEGER NOT NULL DEFAULT 0, thumb INTEGER NOT NULL DEFAULT 0, takeout INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS photos_taken ON photos(taken);
CREATE INDEX IF NOT EXISTS photos_added ON photos(added);
CREATE TABLE IF NOT EXISTS albums(id INTEGER PRIMARY KEY, title TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'local', cover_rel TEXT, created INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS album_items(album_id INTEGER NOT NULL, rel TEXT NOT NULL, PRIMARY KEY(album_id, rel));
CREATE INDEX IF NOT EXISTS album_items_rel ON album_items(rel);
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT);
";

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get::<_, String>(0)).ok()
}

pub fn set_setting(conn: &Connection, key: &str, val: &str) {
    let _ = conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", [key, val]);
}

/// Default photo directory: XDG Pictures (xdg-user-dir PICTURES), fallback ~/Pictures.
pub fn default_pictures() -> PathBuf {
    let h = home();
    if let Ok(o) = std::process::Command::new("xdg-user-dir").arg("PICTURES").output() {
        let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if o.status.success() && !t.is_empty() && PathBuf::from(&t) != h {
            return PathBuf::from(t);
        }
    }
    h.join("Pictures")
}

fn env_path(names: &[&str]) -> Option<PathBuf> {
    for n in names {
        if let Ok(s) = std::env::var(n) {
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    None
}

/// Source directory: KULA_SRC > "source" setting > XDG Pictures.
pub fn resolve_source(conn: &Connection) -> PathBuf {
    if let Some(p) = env_path(&["KULA_SRC"]) {
        return p;
    }
    if let Some(s) = get_setting(conn, "source") {
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    default_pictures()
}

/// Optional Takeout metadata directory (JSON files only): KULA_TAKEOUT (or KULA_META) > "meta" setting; none = skip.
pub fn resolve_meta(conn: &Connection) -> PathBuf {
    if let Some(p) = env_path(&["KULA_TAKEOUT", "KULA_META"]) {
        return p;
    }
    if let Some(s) = get_setting(conn, "meta") {
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    PathBuf::new()
}

pub fn source() -> PathBuf {
    SOURCE.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(default_pictures)
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Path encoding to file:// (no dependencies).
pub fn file_url(p: &std::path::Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut s = String::from("file://");
    for &b in p.as_os_str().as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => s.push(b as char),
            _ => s.push_str(&format!("%{:02X}", b)),
        }
    }
    s
}
