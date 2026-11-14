import QtQuick
import QtQuick.Controls
import org.kula

Item {
    id: al
    property var be
    property var list: []
    signal openAlbum(int id, string title)
    property string sortBy: "newest"
    function reload() {
        var l = JSON.parse(be.albums_json())
        if (sortBy === "name") l.sort(function(a, b) { return a.title.localeCompare(b.title, "pl") })
        else l.sort(function(a, b) { return b.newest - a.newest })
        list = l
    }
    onSortByChanged: reload()
    function plural(n) { return Lang.cnt(n, "item") }
    Component.onCompleted: reload()

    Row {
        id: head
        x: 24; y: 16; spacing: 12
        Text { text: Lang.t("Albums"); color: Theme.text; font.pixelSize: 24; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
        Btn { text: Lang.t("＋  Create album"); accent: true; onClicked: { newTitle.text = ""; np.open() } }
        Btn { text: Lang.t("Sort: ") + (al.sortBy === "newest" ? Lang.t("newest") : Lang.t("name")); onClicked: al.sortBy = al.sortBy === "newest" ? "name" : "newest" }
    }
    GridView {
        anchors.top: head.bottom; anchors.topMargin: 12
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.leftMargin: 18
        clip: true
        cellWidth: 220; cellHeight: 260
        model: al.list
        ScrollBar.vertical: ScrollBar {}
        delegate: Item {
            id: c
            required property var modelData
            width: 220; height: 260
            property string url: modelData.cover >= 0 ? al.be.thumb_url(modelData.cover) : ""
            Timer { interval: 400; repeat: true; running: c.modelData.cover >= 0 && c.url === ""; onTriggered: c.url = al.be.thumb_url(c.modelData.cover) }
            Rectangle {
                x: 6; y: 6; width: 204; height: 204; radius: 10; color: Theme.surface; clip: true
                border.width: cm.containsMouse ? 2 : 0; border.color: Theme.accent
                Image {
                    anchors.fill: parent
                    source: (c.url !== "" && c.url !== "!") ? c.url : ""
                    fillMode: Image.PreserveAspectCrop; asynchronous: true
                    sourceSize: Qt.size(420, 420)
                }
                Image { anchors.centerIn: parent; width: 44; height: 44; sourceSize: Qt.size(88, 88); visible: c.url === "" || c.url === "!"; source: Theme.icon_("albums", "d") }
                MouseArea { id: cm; anchors.fill: parent; hoverEnabled: true; onClicked: al.openAlbum(c.modelData.id, c.modelData.title) }
            }
            Text { x: 8; y: 216; width: 200; elide: Text.ElideRight; text: c.modelData.title; color: Theme.text; font.pixelSize: 14; font.bold: true }
            Text {
                x: 8; y: 236; width: 180; elide: Text.ElideRight
                text: al.plural(c.modelData.count)
                color: Theme.textMuted; font.pixelSize: 12
            }
            Image {
                visible: c.modelData.kind === "takeout"
                x: 190; y: 237; width: 14; height: 14; sourceSize: Qt.size(28, 28); source: Theme.icon_("takeout", "d")
            }
        }
    }
    Text {
        anchors.centerIn: parent; visible: al.list.length === 0
        text: Lang.t("No albums yet. Create the first one with the button above."); color: Theme.textMuted; font.pixelSize: 16
    }
    Popup {
        id: np
        anchors.centerIn: Overlay.overlay
        modal: true; width: 360; padding: 18
        background: Rectangle { color: Theme.surface; radius: 10; border.color: Theme.accent }
        onOpened: newTitle.forceActiveFocus()
        Column {
            spacing: 12; width: parent.width
            Text { text: Lang.t("New album"); color: Theme.text; font.pixelSize: 18; font.bold: true }
            TextField {
                id: newTitle; width: parent.width; placeholderText: Lang.t("Album title")
                color: Theme.text; placeholderTextColor: Theme.textMuted
                background: Rectangle { color: Theme.window; radius: 6; border.color: newTitle.activeFocus ? Theme.accent : Theme.border }
                onAccepted: okb.clicked()
            }
            Row {
                spacing: 8
                Btn { id: okb; text: Lang.t("Create"); accent: true; onClicked: { var id = al.be.album_create(newTitle.text); np.close(); al.reload(); al.openAlbum(id, al.be.album_title(id)) } }
                Btn { text: Lang.t("Cancel"); onClicked: np.close() }
            }
        }
    }
}
