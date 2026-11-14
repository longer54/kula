//! Moving to the system trash (freedesktop Trash), restoring, permanent deletion, "Show in folder".
//! Never deletes permanently instead of trashing: if trashing fails, the file stays and the error is returned to the UI.
use crate::{ai, db, state};
use serde_json::json;
use std::path::{Path, PathBuf};

fn targets_abs(id: i64) -> Vec<(i64, String, PathBuf)> {
    let s = state::st();
    let ids: Vec<i64> = if id >= 0 { vec![id] } else { s.items.iter().filter(|i| s.sel.contains(&i.id)).map(|i| i.id).collect() };
    let src = db::source();
    ids.into_iter()
        .filter_map(|i| s.conn.query_row("SELECT rel FROM photos WHERE id=?1", [i], |r| r.get::<_, String>(0)).ok().map(|rel| (i, rel.clone(), src.join(rel))))
        .collect()
}

pub fn count_targets(id: i64) -> usize {
    targets_abs(id).len()
}

fn ai_remove(rels: &[String]) {
    let p = ai::ai_path();
    if !p.is_file() || rels.is_empty() {
        return;
    }
    let Ok(c) = rusqlite::Connection::open(&p) else { return };
    let _ = c.busy_timeout(std::time::Duration::from_secs(8));
    let _ = c.execute_batch("BEGIN");
    for r in rels {
        for t in ["photos", "clip", "faces"] {
            let _ = c.execute(&format!("DELETE FROM {t} WHERE rel_path=?1"), [r]);
        }
    }
    let _ = c.execute_batch("COMMIT");
}

/// Test mode: only when KULA_SELFTEST=1 AND the data directory is /tmp/kula-selftest-*.
/// Then "deletion" means only a row in the database copy — no file operations.
pub fn selftest_mode() -> bool {
    std::env::var("KULA_SELFTEST").map(|v| v == "1").unwrap_or(false) && db::data_dir().to_string_lossy().starts_with("/tmp/kula-selftest-")
}

/// To the system trash. Returns JSON {ok, failed, errors:[...]}.
pub fn delete_to_trash(id: i64) -> String {
    let t = targets_abs(id);
    let mut ok = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for (pid, rel, abs) in &t {
        if selftest_mode() {
            ok.push((*pid, rel.clone()));
            continue;
        }
        if !abs.is_file() {
            errors.push(format!("{rel}: {}", crate::state::tr("plik nie istnieje", "file does not exist")));
            continue;
        }
        match trash::delete(abs) {
            Ok(()) => ok.push((*pid, rel.clone())),
            Err(e) => errors.push(format!("{rel}: {e}")),
        }
    }
    {
        let s = state::st();
        let _ = s.conn.execute_batch("BEGIN");
        for (pid, _) in &ok {
            let _ = s.conn.execute("UPDATE photos SET trashed=2, fav=0 WHERE id=?1", [pid]);
        }
        let _ = s.conn.execute_batch("COMMIT");
    }
    let rels: Vec<String> = ok.iter().map(|x| x.1.clone()).collect();
    {
        let s = state::st();
        for r in &rels {
            let _ = s.conn.execute("DELETE FROM album_items WHERE rel=?1", [r]);
        }
    }
    ai_remove(&rels);
    let pids: Vec<i64> = ok.iter().map(|x| x.0).collect();
    state::remove_ids(&pids);
    json!({"ok":ok.len(),"failed":errors.len(),"errors":errors}).to_string()
}

fn find_trash_items(paths: &[PathBuf]) -> Vec<trash::TrashItem> {
    let Ok(list) = trash::os_limited::list() else { return vec![] };
    let mut out: Vec<trash::TrashItem> = Vec::new();
    for p in paths {
        // newest entry with this original path
        if let Some(it) = list.iter().filter(|i| i.original_path() == *p).max_by_key(|i| i.time_deleted) {
            out.push(it.clone());
        }
    }
    out
}

/// Restore from the system trash to the original location.
pub fn restore(id: i64) -> String {
    let t: Vec<_> = targets_abs(id);
    let paths: Vec<PathBuf> = t.iter().map(|x| x.2.clone()).collect();
    let items = find_trash_items(&paths);
    let mut errors: Vec<String> = Vec::new();
    let mut restored = 0;
    let mut done_ids: Vec<i64> = Vec::new();
    for it in items {
        let orig = it.original_path();
        match trash::os_limited::restore_all([it]) {
            Ok(()) => {
                restored += 1;
                if let Some(x) = t.iter().find(|x| x.2 == orig) {
                    done_ids.push(x.0);
                }
                if let Ok(rel) = orig.strip_prefix(db::source()) {
                    let s = state::st();
                    let _ = s.conn.execute("UPDATE photos SET trashed=0 WHERE rel=?1", [rel.to_string_lossy().to_string()]);
                }
            }
            Err(e) => errors.push(format!("{}: {e}", orig.display())),
        }
    }
    if restored < t.len() && errors.is_empty() {
        errors.push(crate::state::tr("Nie znaleziono części plików w koszu systemowym", "Some files were not found in the system trash"));
    }
    state::remove_ids(&done_ids);
    json!({"ok":restored,"failed":t.len()-restored,"errors":errors}).to_string()
}

/// Permanent deletion — only of items already in the system trash.
pub fn purge(id: i64) -> String {
    let t: Vec<_> = {
        let all = targets_abs(id);
        let s = state::st();
        all.into_iter().filter(|x| s.conn.query_row("SELECT trashed FROM photos WHERE id=?1", [x.0], |r| r.get::<_, i64>(0)).unwrap_or(0) == 2).collect()
    };
    let paths: Vec<PathBuf> = t.iter().map(|x| x.2.clone()).collect();
    let items = find_trash_items(&paths);
    let n = items.len();
    let mut errors: Vec<String> = Vec::new();
    let mut purged_paths: Vec<PathBuf> = Vec::new();
    for it in items {
        let p = it.original_path();
        match trash::os_limited::purge_all([it]) {
            Ok(()) => purged_paths.push(p),
            Err(e) => errors.push(format!("{}: {e}", p.display())),
        }
    }
    {
        let s = state::st();
        for (pid, rel, abs) in &t {
            // the row is also deleted when the file is no longer in the trash (e.g. emptied manually)
            if purged_paths.contains(abs) {
                let _ = s.conn.execute("DELETE FROM photos WHERE id=?1", [pid]);
                let _ = s.conn.execute("DELETE FROM album_items WHERE rel=?1", [rel]);
            }
        }
    }
    let pids: Vec<i64> = t.iter().filter(|x| purged_paths.contains(&x.2)).map(|x| x.0).collect();
    state::remove_ids(&pids);
    let _ = n;
    json!({"ok":purged_paths.len(),"failed":errors.len(),"errors":errors}).to_string()
}

fn uri_of(p: &Path) -> String {
    let mut s = String::from("file://");
    for b in p.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{:02X}", b));
        }
    }
    s
}

fn klog(msg: &str) {
    use std::io::Write;
    let p = db::cache_dir().join("kula.log");
    let _ = std::fs::create_dir_all(db::cache_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "{msg}");
    }
}

///Runs the command without blocking; waits at most `secs`, logs stderr. true = exit code 0.
fn run_timeout(cmd: &str, args: &[String], secs: u64) -> bool {
    use std::process::Stdio;
    let mut ch = match std::process::Command::new(cmd).args(args).stdout(Stdio::null()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => {
            klog(&format!("show_in_folder: {cmd} failed to start: {e}"));
            return false;
        }
    };
    let t0 = std::time::Instant::now();
    loop {
        match ch.try_wait() {
            Ok(Some(st)) => {
                if !st.success() {
                    let mut err = String::new();
                    if let Some(mut e) = ch.stderr.take() {
                        use std::io::Read;
                        let _ = e.read_to_string(&mut err);
                    }
                    klog(&format!("show_in_folder: {cmd} {args:?} -> {st}: {}", err.trim()));
                }
                return st.success();
            }
            Ok(None) => {
                if t0.elapsed().as_secs() >= secs {
                    let _ = ch.kill();
                    klog(&format!("show_in_folder: {cmd} timed out after {secs} s"));
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

fn gdbus_fm(method: &str, uri: &str) -> bool {
    let args: Vec<String> = ["call", "--session", "--timeout", "5", "--dest", "org.freedesktop.FileManager1", "--object-path", "/org/freedesktop/FileManager1", "--method"]
        .iter().map(|s| s.to_string()).chain([format!("org.freedesktop.FileManager1.{method}"), format!("['{uri}']"), String::new()]).collect();
    run_timeout("gdbus", &args, 7)
}

/// Synchronous: opens the file manager. COSMIC: ShowFolders(directory) (ShowItems with a file gives cosmic-files the file's URI).
pub fn show_folder_sync(abs: &Path) -> bool {
    let parent = abs.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let puri = uri_of(&parent);
    let cosmic = ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"].iter().any(|k| std::env::var(k).map(|d| d.to_uppercase().contains("COSMIC")).unwrap_or(false));
    klog(&format!("show_in_folder: {} cosmic={cosmic} dir={puri}", abs.display()));
    if cosmic {
        if gdbus_fm("ShowFolders", &puri) {
            return true;
        }
        if run_timeout("cosmic-files", &[parent.to_string_lossy().to_string()], 1) {
            return true;
        }
        // cosmic-files keeps running as a window: timeout = launched
        return std::process::Command::new("cosmic-files").arg(&parent).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok();
    }
    if gdbus_fm("ShowItems", &uri_of(abs)) || gdbus_fm("ShowFolders", &puri) {
        return true;
    }
    std::process::Command::new("xdg-open").arg(&parent).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok()
}

/// Does not block the UI: work happens in a thread, result goes to the log.
pub fn show_in_folder(id: i64) -> bool {
    let Some((_, _, abs)) = targets_abs(id.max(0)).into_iter().next() else { return false };
    std::thread::spawn(move || {
        let ok = show_folder_sync(&abs);
        klog(&format!("show_in_folder: result {ok}"));
    });
    true
}

/// `kula --selftest-showfolder <relative or absolute path>`
pub fn selftest_showfolder(arg: &str) -> bool {
    let p = std::path::PathBuf::from(arg);
    let abs = if p.is_absolute() { p } else { db::source().join(arg) };
    println!("selftest-showfolder: {} -> URI katalogu {}", abs.display(), uri_of(abs.parent().unwrap_or(Path::new("/"))));
    let ok = show_folder_sync(&abs);
    println!("selftest-showfolder: result {ok}");
    ok
}

pub fn abs_path(id: i64) -> String {
    targets_abs(id).into_iter().next().map(|x| x.2.to_string_lossy().to_string()).unwrap_or_default()
}

/// Test on temporary files (never on real photos): `kula --selftest-trash`.
pub fn selftest() -> bool {
    let src = db::source();
    println!("selftest: test source {}", src.display());
    let fail = |m: &str| -> bool {
        println!("selftest ERROR: {m}");
        false
    };
    if !src.to_string_lossy().contains("kula-selftest") {
        return fail("refused: source is not a test directory (KULA_SRC must contain \"kula-selftest\")");
    }
    crate::scan::run();
    {
        let s = state::st();
        let _ = &s;
    }
    state::set_view("timeline", "");
    let n = state::count();
    println!("selftest: zindeksowano {n}");
    if n < 2 {
        return fail("too few test files");
    }
    let id0 = state::id_at(0);
    let abs0 = PathBuf::from(abs_path(id0));
    let r = delete_to_trash(id0);
    println!("selftest: kosz → {r}");
    if abs0.exists() {
        return fail("file still exists after moving to trash (trash failed, file untouched — correct error behaviour, but the test failed)");
    }
    if state::count() != n - 1 {
        return fail("view not updated");
    }
    state::set_view("trash", "");
    if state::count() != 1 {
        return fail("Trash view does not show the deleted item");
    }
    let r = restore(state::id_at(0));
    println!("selftest: przywracanie → {r}");
    if !abs0.exists() {
        return fail("restore did not bring the file back");
    }
    state::set_view("timeline", "");
    let id1 = state::id_at(0);
    let abs1 = PathBuf::from(abs_path(id1));
    let _ = delete_to_trash(id1);
    state::set_view("trash", "");
    let r = purge(state::id_at(0));
    println!("selftest: permanent delete → {r}");
    if abs1.exists() || state::count() != 0 {
        return fail("purge failed");
    }
    println!("selftest OK");
    true
}
