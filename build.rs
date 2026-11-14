use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    let res: Vec<String> = [16, 32, 48, 64, 128, 256, 512]
        .iter()
        .map(|s| format!("icons/kula-{s}.png"))
        .collect();
    let mut res = res;
    let mut svgs: Vec<String> = std::fs::read_dir("icons").unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".svg")).map(|e| format!("icons/{}", e.file_name().to_string_lossy())).collect();
    svgs.sort();
    res.extend(svgs);
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=cpp/kula.cpp");
    CxxQtBuilder::new_qml_module(QmlModule::new("org.kula").qml_files([
        QmlFile::from("qml/Theme.qml").singleton(true),
        QmlFile::from("qml/Lang.qml").singleton(true),
        "qml/Main.qml".into(),
        "qml/Timeline.qml".into(),
        "qml/Viewer.qml".into(),
        "qml/Albums.qml".into(),
        "qml/Persons.qml".into(),
        "qml/Settings.qml".into(),
        "qml/Btn.qml".into(),
        "qml/IconBtn.qml".into(),
        "qml/Tile.qml".into(),
        "qml/VideoPane.qml".into(),
    ]))
    .qrc_resources(res)
    .qt_module("Quick")
    .files(["src/bridge.rs"])
    .cpp_file("cpp/kula.cpp")
    .include_dir("cpp")
    .build();
}
