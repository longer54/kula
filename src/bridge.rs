//! Rust ↔ QML bridge. The backend is a thin layer over state/scan/thumbs/ai (all local, SQLite).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;
        include!("kula.h");
        fn kula_new_application(args: &[String]) -> UniquePtr<QGuiApplication>;
        fn kula_make_thumb(src: &str, dst: &str, size: i32) -> bool;
        fn kula_probe(src: &str, w: &mut i32, h: &mut i32) -> bool;
        fn kula_make_face(src: &str, dst: &str, x: f64, y: f64, w: f64, h: f64, size: i32) -> bool;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type Backend = super::BackendRust;

        #[qinvokable]
        fn version(self: &Backend) -> QString;
        #[qinvokable]
        fn autotest_dir(self: &Backend) -> QString;
        /// Scan/thumbnail/view state: JSON.
        #[qinvokable]
        fn poll(self: &Backend) -> QString;
        #[qinvokable]
        fn settings_json(self: &Backend) -> QString;
        #[qinvokable]
        fn set_paths(self: &Backend, source: &QString, meta: &QString);
        #[qinvokable]
        fn set_lang(self: &Backend, code: &QString);
        #[qinvokable]
        fn scan_now(self: &Backend);
        #[qinvokable]
        fn setting_get(self: &Backend, key: &QString) -> QString;
        #[qinvokable]
        fn setting_set(self: &Backend, key: &QString, val: &QString);
        #[qinvokable]
        fn set_view(self: &Backend, kind: &QString, arg: &QString);
        #[qinvokable]
        fn view_kind(self: &Backend) -> QString;
        #[qinvokable]
        fn layout(self: &Backend, width: f64, row_h: f64) -> QString;
        #[qinvokable]
        fn thumb_url(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn file_url(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn item_json(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn count(self: &Backend) -> i64;
        #[qinvokable]
        fn index_of(self: &Backend, id: i64) -> i64;
        #[qinvokable]
        fn id_at(self: &Backend, idx: i64) -> i64;
        #[qinvokable]
        fn counts_json(self: &Backend) -> QString;
        // selection
        #[qinvokable]
        fn toggle_sel(self: &Backend, id: i64) -> i64;
        #[qinvokable]
        fn range_sel(self: &Backend, idx: i64) -> i64;
        #[qinvokable]
        fn sel_group(self: &Backend, a: i64, b: i64, on: bool) -> i64;
        #[qinvokable]
        fn group_selected(self: &Backend, a: i64, b: i64) -> bool;
        #[qinvokable]
        fn sel_all(self: &Backend) -> i64;
        #[qinvokable]
        fn clear_sel(self: &Backend);
        #[qinvokable]
        fn is_selected(self: &Backend, id: i64) -> bool;
        #[qinvokable]
        fn sel_count(self: &Backend) -> i64;
        // marks (id = -1 → selection)
        #[qinvokable]
        fn mark(self: &Backend, kind: &QString, id: i64, val: i32) -> i64;
        #[qinvokable]
        fn set_desc(self: &Backend, id: i64, text: &QString);
        // albums
        #[qinvokable]
        fn selftest_scroll(self: &Backend) -> bool;
        #[qinvokable]
        fn selftest_log(self: &Backend, msg: &QString);
        #[qinvokable]
        fn albums_json(self: &Backend) -> QString;
        #[qinvokable]
        fn album_create(self: &Backend, title: &QString) -> i64;
        #[qinvokable]
        fn album_add(self: &Backend, aid: i64, id: i64) -> i64;
        #[qinvokable]
        fn album_remove(self: &Backend, aid: i64, id: i64) -> i64;
        #[qinvokable]
        fn album_rename(self: &Backend, aid: i64, title: &QString);
        #[qinvokable]
        fn album_delete(self: &Backend, aid: i64);
        #[qinvokable]
        fn album_title(self: &Backend, aid: i64) -> QString;
        #[qinvokable]
        fn album_kind(self: &Backend, aid: i64) -> QString;
        // AI
        #[qinvokable]
        fn has_persons(self: &Backend) -> bool;
        #[qinvokable]
        fn persons_json(self: &Backend) -> QString;
        #[qinvokable]
        fn semantic_available(self: &Backend) -> bool;
        #[qinvokable]
        fn person_name_set(self: &Backend, id: i64, name: &QString) -> bool;
        #[qinvokable]
        fn persons_merge(self: &Backend, ids_json: &QString) -> i64;
        #[qinvokable]
        fn copy_text(self: &Backend, text: &QString);
        #[qinvokable]
        fn target_count(self: &Backend, id: i64) -> i64;
        #[qinvokable]
        fn delete_items(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn restore_items(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn purge_items(self: &Backend, id: i64) -> QString;
        #[qinvokable]
        fn show_in_folder(self: &Backend, id: i64) -> bool;
        #[qinvokable]
        fn abs_path(self: &Backend, id: i64) -> QString;
    }
}

use cxx_qt_lib::QString;
use std::sync::atomic::Ordering::Relaxed;

pub use qobject::kula_new_application;

#[derive(Default)]
pub struct BackendRust {}

fn q(s: impl AsRef<str>) -> QString {
    QString::from(s.as_ref())
}

pub fn init() {
    crate::state::init();
    if !std::env::args().any(|a| a == "--selftest-scroll") {
        crate::thumbs::start_workers(10);
    }
}

impl qobject::Backend {
    fn version(&self) -> QString {
        q(env!("CARGO_PKG_VERSION"))
    }
    fn autotest_dir(&self) -> QString {
        q(std::env::args().find_map(|a| a.strip_prefix("--autotest=").map(|s| s.to_string())).unwrap_or_default())
    }
    fn selftest_log(&self, msg: &QString) {
        println!("{}", msg);
    }
    fn selftest_scroll(&self) -> bool {
        std::env::args().any(|a| a == "--selftest-scroll")
    }
    fn poll(&self) -> QString {
        use crate::{scan, thumbs};
        q(serde_json::json!({
            "ver": scan::VIEW_VER.load(Relaxed),
            "phase": scan::PHASE.load(Relaxed),
            "found": scan::FOUND.load(Relaxed), "todo": scan::TODO.load(Relaxed), "done": scan::DONE.load(Relaxed),
            "removed": scan::REMOVED.load(Relaxed), "json_files": scan::JSON_FILES.load(Relaxed), "json_matched": scan::JSON_MATCHED.load(Relaxed),
            "d_exif": scan::D_EXIF.load(Relaxed), "d_json": scan::D_JSON.load(Relaxed), "d_name": scan::D_NAME.load(Relaxed), "d_none": scan::D_MTIME.load(Relaxed),
            "scan_ms": scan::MS.load(Relaxed),
            "th_total": thumbs::TOTAL.load(Relaxed), "th_done": thumbs::DONE.load(Relaxed), "th_failed": thumbs::FAILED.load(Relaxed),
            "ai_busy": crate::state::AI_BUSY.load(Relaxed),
        }).to_string())
    }
    fn settings_json(&self) -> QString {
        let s = crate::state::st();
        let meta = crate::db::resolve_meta(&s.conn);
        q(serde_json::json!({
            "source": crate::db::source().to_string_lossy(),
            "meta": meta.to_string_lossy(),
            "db": crate::db::db_path().to_string_lossy(),
            "cache": crate::db::cache_dir().to_string_lossy(),
            "ai": crate::ai::ai_path().to_string_lossy(),
            "ai_present": crate::ai::ai_path().is_file(),
            "semantic": crate::ai::semantic_available(),
            "version": env!("CARGO_PKG_VERSION"),
        }).to_string())
    }
    fn set_lang(&self, code: &QString) {
        crate::state::PL.store(code.to_string() == "pl", std::sync::atomic::Ordering::Relaxed);
    }
    fn set_paths(&self, source: &QString, meta: &QString) {
        let s = crate::state::st();
        crate::db::set_setting(&s.conn, "source", &source.to_string());
        crate::db::set_setting(&s.conn, "meta", &meta.to_string());
    }
    fn setting_get(&self, key: &QString) -> QString {
        let s = crate::state::st();
        q(&crate::db::get_setting(&s.conn, &key.to_string()).unwrap_or_default())
    }
    fn setting_set(&self, key: &QString, val: &QString) {
        let s = crate::state::st();
        crate::db::set_setting(&s.conn, &key.to_string(), &val.to_string());
    }
    fn scan_now(&self) {
        crate::scan::start_background();
    }
    fn set_view(&self, kind: &QString, arg: &QString) {
        crate::state::set_view(&kind.to_string(), &arg.to_string());
    }
    fn view_kind(&self) -> QString {
        q(crate::state::st().view.0.clone())
    }
    fn layout(&self, width: f64, row_h: f64) -> QString {
        q(crate::state::layout(width, row_h))
    }
    fn thumb_url(&self, id: i64) -> QString {
        q(crate::state::thumb_url(id))
    }
    fn file_url(&self, id: i64) -> QString {
        q(crate::state::file_url_of(id))
    }
    fn item_json(&self, id: i64) -> QString {
        q(crate::state::item_json(id))
    }
    fn count(&self) -> i64 {
        crate::state::count() as i64
    }
    fn index_of(&self, id: i64) -> i64 {
        crate::state::index_of(id)
    }
    fn id_at(&self, idx: i64) -> i64 {
        crate::state::id_at(idx)
    }
    fn counts_json(&self) -> QString {
        q(crate::state::counts_json())
    }
    fn toggle_sel(&self, id: i64) -> i64 {
        crate::state::toggle_sel(id) as i64
    }
    fn range_sel(&self, idx: i64) -> i64 {
        crate::state::range_sel(idx.max(0) as usize) as i64
    }
    fn sel_group(&self, a: i64, b: i64, on: bool) -> i64 {
        crate::state::sel_group(a.max(0) as usize, b.max(0) as usize, on) as i64
    }
    fn group_selected(&self, a: i64, b: i64) -> bool {
        crate::state::group_selected(a.max(0) as usize, b.max(0) as usize)
    }
    fn sel_all(&self) -> i64 {
        crate::state::sel_all() as i64
    }
    fn clear_sel(&self) {
        crate::state::clear_sel()
    }
    fn is_selected(&self, id: i64) -> bool {
        crate::state::is_selected(id)
    }
    fn sel_count(&self) -> i64 {
        crate::state::sel_count() as i64
    }
    fn mark(&self, kind: &QString, id: i64, val: i32) -> i64 {
        crate::state::mark(&kind.to_string(), id, val).0 as i64
    }
    fn set_desc(&self, id: i64, text: &QString) {
        crate::state::set_desc(id, &text.to_string())
    }
    fn albums_json(&self) -> QString {
        q(crate::state::albums_json())
    }
    fn album_create(&self, title: &QString) -> i64 {
        crate::state::album_create(&title.to_string())
    }
    fn album_add(&self, aid: i64, id: i64) -> i64 {
        crate::state::album_add(aid, id) as i64
    }
    fn album_remove(&self, aid: i64, id: i64) -> i64 {
        crate::state::album_remove(aid, id) as i64
    }
    fn album_rename(&self, aid: i64, title: &QString) {
        crate::state::album_rename(aid, &title.to_string())
    }
    fn album_delete(&self, aid: i64) {
        crate::state::album_delete(aid)
    }
    fn album_title(&self, aid: i64) -> QString {
        q(crate::state::album_title(aid))
    }
    fn album_kind(&self, aid: i64) -> QString {
        q(crate::state::album_kind(aid))
    }
    fn has_persons(&self) -> bool {
        crate::ai::has_persons()
    }
    fn persons_json(&self) -> QString {
        q(crate::state::persons_json())
    }
    fn person_name_set(&self, id: i64, name: &QString) -> bool {
        crate::ai::rename_person(id, &name.to_string())
    }
    fn persons_merge(&self, ids_json: &QString) -> i64 {
        let ids: Vec<i64> = serde_json::from_str(&ids_json.to_string()).unwrap_or_default();
        crate::ai::merge_persons(&ids)
    }
    fn semantic_available(&self) -> bool {
        crate::ai::semantic_available()
    }
    fn target_count(&self, id: i64) -> i64 {
        crate::trashing::count_targets(id) as i64
    }
    fn delete_items(&self, id: i64) -> QString {
        q(crate::trashing::delete_to_trash(id))
    }
    fn restore_items(&self, id: i64) -> QString {
        q(crate::trashing::restore(id))
    }
    fn purge_items(&self, id: i64) -> QString {
        q(crate::trashing::purge(id))
    }
    fn show_in_folder(&self, id: i64) -> bool {
        crate::trashing::show_in_folder(id)
    }
    fn abs_path(&self, id: i64) -> QString {
        q(crate::trashing::abs_path(id))
    }
    fn copy_text(&self, text: &QString) {
        // clipboard: wl-copy / xclip if available (no Qt Widgets dependency)
        use std::io::Write;
        for (bin, args) in [("wl-copy", vec![]), ("xclip", vec!["-selection", "clipboard"])] {
            if let Ok(mut c) = std::process::Command::new(bin).args(&args).stdin(std::process::Stdio::piped()).spawn() {
                if let Some(mut si) = c.stdin.take() {
                    let _ = si.write_all(text.to_string().as_bytes());
                }
                let _ = c.wait();
                return;
            }
        }
    }
}
