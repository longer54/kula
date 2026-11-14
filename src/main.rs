//! Kula — a local photo app. Originals read-only; everything else in SQLite.
mod ai;
mod bridge;
mod db;
mod scan;
mod state;
mod thumbs;
mod trashing;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("kula {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Kula {}\n\nUsage: kula [--scan-only] [--autotest=KATALOG] [--selftest-scroll] [--version]\n\n  --scan-only       scan + thumbnails without a window, prints statistics\n  --autotest=DIR    test script (offscreen): UI screenshots into DIR, then exit\n\nEnvironment: KULA_SRC (photos), KULA_TAKEOUT (optional: Takeout JSON), KULA_DATA, KULA_CACHE.", env!("CARGO_PKG_VERSION"));
        return;
    }
    bridge::init();
    if let Some(i) = args.iter().position(|a| a == "--selftest-showfolder") {
        let arg = args.get(i + 1).cloned().unwrap_or_default();
        std::process::exit(if trashing::selftest_showfolder(&arg) { 0 } else { 1 });
    }
    if args.iter().any(|a| a == "--selftest-trash") {
        std::process::exit(if trashing::selftest() { 0 } else { 1 });
    }
    if args.iter().any(|a| a == "--scan-only") {
        scan_only();
        return;
    }
    let selftest = args.iter().any(|a| a == "--selftest-scroll");
    if selftest {
        if !trashing::selftest_mode() {
            eprintln!("selftest-scroll requires KULA_SELFTEST=1 and KULA_DATA=/tmp/kula-selftest-* (database copy)");
            std::process::exit(2);
        }
    } else {
        scan::start_background();
    }
    let mut app = bridge::kula_new_application(&args);
    if let Some(mut a) = app.as_mut() {
        a.as_mut().set_application_name(&QString::from("Kula"));
        a.as_mut().set_application_display_name(&QString::from("Kula"));
        QGuiApplication::set_desktop_file_name(&QString::from("kula"));
    }
    let mut qml = QQmlApplicationEngine::new();
    if let Some(qml) = qml.as_mut() {
        qml.load(&QUrl::from("qrc:/qt/qml/org/kula/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        let code = app.exec();
        if selftest {
            std::process::exit(code);
        }
    }
}

fn scan_only() {
    use std::sync::atomic::Ordering::Relaxed;
    let src = db::source();
    println!("source: {}", src.display());
    let t0 = std::time::Instant::now();
    scan::run();
    println!("scan: {} ms", t0.elapsed().as_millis());
    println!("{}", scan::stats_line());
    let t1 = std::time::Instant::now();
    let total = thumbs::TOTAL.load(Relaxed);
    println!("thumbnails to build: {total}");
    let limit: Option<usize> = std::env::var("KULA_THUMB_LIMIT").ok().and_then(|s| s.parse().ok());
    if limit == Some(0) {
        return;
    }
    thumbs::start_workers(12);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let done = thumbs::DONE.load(Relaxed);
        if thumbs::idle() || limit.map(|l| done >= l).unwrap_or(false) {
            break;
        }
    }
    println!("thumbnails: done={} failed={} time={} ms", thumbs::DONE.load(Relaxed), thumbs::FAILED.load(Relaxed), t1.elapsed().as_millis());
}
