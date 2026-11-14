import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import org.kula

Flickable {
    id: st
    property var be
    property var s: ({})
    property var status: ({})
    signal rescan()
    contentHeight: col.implicitHeight + 60
    clip: true
    function reload() { s = JSON.parse(be.settings_json()); srcF.text = s.source; metaF.text = s.meta }
    Component.onCompleted: reload()
    ScrollBar.vertical: ScrollBar {}

    Column {
        id: col
        x: 24; y: 16; width: Math.min(760, st.width - 48); spacing: 14
        Text { text: Lang.t("Settings"); color: Theme.text; font.pixelSize: 24; font.bold: true }
        Text { text: Lang.t("Theme"); color: Theme.text; font.pixelSize: 16; font.bold: true }
        Row {
            spacing: 8
            Repeater {
                model: [{ k: "light", t: Lang.t("Light") }, { k: "dark", t: Lang.t("Dark") }, { k: "system", t: Lang.t("System") }]
                delegate: Rectangle {
                    required property var modelData
                    property bool on: Theme.mode === modelData.k
                    width: 110; height: 34; radius: Theme.radius
                    color: on ? Theme.selected : (tm.containsMouse ? Theme.hover : Theme.surface)
                    border.width: 1; border.color: on ? Theme.accent : Theme.border
                    Text { anchors.centerIn: parent; text: modelData.t; color: on ? Theme.accent : Theme.text; font.pixelSize: 14 }
                    MouseArea { id: tm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        onClicked: { Theme.mode = modelData.k; st.be.setting_set("theme", modelData.k) } }
                }
            }
        }
        Text { text: Lang.t("Language"); color: Theme.text; font.pixelSize: 16; font.bold: true }
        Row {
            spacing: 8
            Repeater {
                model: [{ k: "auto", t: Lang.t("Automatic") }, { k: "en", t: "English" }, { k: "pl", t: "Polski" }]
                delegate: Rectangle {
                    required property var modelData
                    property bool on: Lang.mode === modelData.k
                    width: 110; height: 34; radius: Theme.radius
                    color: on ? Theme.selected : (lm.containsMouse ? Theme.hover : Theme.surface)
                    border.width: 1; border.color: on ? Theme.accent : Theme.border
                    Text { anchors.centerIn: parent; text: modelData.t; color: on ? Theme.accent : Theme.text; font.pixelSize: 14 }
                    MouseArea { id: lm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        onClicked: { Lang.mode = modelData.k; st.be.setting_set("lang", modelData.k) } }
                }
            }
        }
        Rectangle { width: parent.width; height: 1; color: Theme.border }
        Text { text: Lang.t("Photo folder (read-only — Kula never changes anything in it)"); color: Theme.textMuted; font.pixelSize: 13 }
        Row {
            width: parent.width; spacing: 8
            TextField {
                id: srcF; width: parent.width - 150; color: Theme.text; selectByMouse: true
                background: Rectangle { color: Theme.surface; radius: 6; border.color: srcF.activeFocus ? Theme.accent : Theme.border }
            }
            Btn { text: Lang.t("Photo folder…"); onClicked: srcDialog.open() }
        }
        FolderDialog {
            id: srcDialog; title: Lang.t("Photo folder")
            onAccepted: { srcF.text = decodeURIComponent(selectedFolder.toString().replace("file://", "")); st.be.set_paths(srcF.text, metaF.text); st.rescan() }
        }
        Text { text: Lang.t("Takeout metadata folder (JSON files; empty = none)"); color: Theme.textMuted; font.pixelSize: 13 }
        TextField {
            id: metaF; width: parent.width; color: Theme.text; selectByMouse: true
            background: Rectangle { color: Theme.surface; radius: 6; border.color: metaF.activeFocus ? Theme.accent : Theme.border }
        }
        Row {
            spacing: 10
            Btn { text: Lang.t("Save and scan"); accent: true; onClicked: { st.be.set_paths(srcF.text, metaF.text); st.rescan() } }
            Btn { text: Lang.t("Scan now"); onClicked: st.rescan() }
        }
        Rectangle { width: parent.width; height: 1; color: Theme.border }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: Theme.text; font.pixelSize: 13; lineHeight: 1.35
            text: {
                var p = st.status, r = []
                r.push(Lang.t("Scan: ") + (p.phase === 0 ? Lang.t("idle") : ["", Lang.t("loading JSON"), Lang.t("browsing folders"), Lang.t("reading metadata"), Lang.t("cleaning up")][p.phase] || ""))
                r.push(Lang.t("Files found: ") + Theme.fmt(p.found || 0) + Lang.t("   to read: ") + Theme.fmt(p.todo || 0) + Lang.t("   processed: ") + Theme.fmt(p.done || 0))
                r.push(Lang.t("JSON files: ") + (p.json_files || 0) + Lang.t("   matched to photos: ") + (p.json_matched || 0))
                r.push(Lang.t("Date source (last scan): EXIF ") + (p.d_exif || 0) + ", JSON " + (p.d_json || 0) + Lang.t(", name ") + (p.d_name || 0) + Lang.t(", none ") + (p.d_none || 0))
                r.push(Lang.t("Thumbnails: ") + Theme.fmt(p.th_done || 0) + " / " + Theme.fmt(p.th_total || 0) + Lang.t("   errors: ") + (p.th_failed || 0))
                return r.join("\n")
            }
        }
        Rectangle { width: parent.width; height: 1; color: Theme.border }
        Text {
            width: parent.width; wrapMode: Text.WrapAnywhere; color: Theme.textMuted; font.pixelSize: 12; lineHeight: 1.35
            text: Lang.t("Index: ") + (st.s.db || "") + "\n" + Lang.t("Thumbnails: ") + (st.s.cache || "")
                  + (st.s.ai_present || st.s.semantic ? "\n" + Lang.t("AI data: ") + (st.s.ai || "") + (st.s.ai_present ? Lang.t(" (present)") : Lang.t(" (missing)")) + "\n" + Lang.t("Content search: ") + (st.s.semantic ? Lang.t("available") : Lang.t("unavailable")) : "")
                  + "\n" + Lang.t("Version: ") + (st.s.version || "")
        }
        Rectangle { width: parent.width; height: 1; color: Theme.border }
        Text { text: Lang.t("Keyboard shortcuts"); color: Theme.text; font.pixelSize: 16; font.bold: true }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: 13; lineHeight: 1.4
            text: Lang.t("/  search      ← →  previous / next      z  + −  zoom      i  Info      Space  video\n")
                + Lang.t("x  select (under cursor)      Shift+click  range      Ctrl+A  all      f  favorite      Shift+A  archive\n")
                + Lang.t("#  or  Delete  to trash      Ctrl+wheel  grid size      F11  full screen      Esc  back / deselect")
        }
    }
}
