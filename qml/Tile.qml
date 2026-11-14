import QtQuick
import org.kula

Rectangle {
    id: t
    required property var tl
    required property var d      // [id, width, index, kind, time, favorite]
    required property real th
    property int pid: d[0]
    property string url: ""
    property bool sel: { tl.selVer; return tl.be.is_selected(pid) }
    width: d[1]
    height: th
    color: Theme.window
    clip: true
    property real shimmer: 0
    NumberAnimation on shimmer { from: 0; to: 1; duration: 1500; loops: Animation.Infinite; running: img.status !== Image.Ready && t.url !== "!" }

    function dur(s) {
        var n = Math.round(s), m = Math.floor(n / 60), x = n % 60
        return m + ":" + (x < 10 ? "0" : "") + x
    }
    Component.onCompleted: url = tl.be.thumb_url(pid)
    onPidChanged: url = tl.be.thumb_url(pid)
    Timer {
        interval: 350; repeat: true; running: t.url === ""
        onTriggered: t.url = tl.be.thumb_url(t.pid)
    }
    Image {
        id: img
        anchors.fill: parent
        anchors.margins: t.sel ? 10 : 0
        source: (t.url !== "" && t.url !== "!") ? t.url : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }
        sourceSize: Qt.size(t.width * 2, t.height * 2)
        Behavior on anchors.margins { NumberAnimation { duration: 90 } }
    }
    Rectangle {
        // subtle "breathing" while the thumbnail loads
        anchors.fill: parent
        visible: img.status !== Image.Ready && t.url !== "!"
        color: Theme.navSelected
        opacity: 0.10 + 0.22 * (0.5 - 0.5 * Math.cos(t.shimmer * 6.2832))
    }
    Text {
        anchors.centerIn: parent
        visible: t.url === "!" || false
        text: t.d[3] === 1 ? "▶" : "▣"
        color: Theme.textMuted
        font.pixelSize: 26
    }
    Rectangle {
        visible: t.d[3] === 1
        anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 6
        radius: 9; height: 18; width: dtxt.implicitWidth + 12
        color: Theme.scrim
        Text { id: dtxt; anchors.centerIn: parent; text: "▶ " + t.dur(t.d[4]); color: Theme.onMedia; font.pixelSize: 11 }
    }
    Text {
        visible: t.d[5] === 1
        anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.margins: 6
        text: "★"; color: Theme.accent; font.pixelSize: 18
        style: Text.Outline; styleColor: Theme.mediaShade
    }
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.width: t.sel ? 2 : 0
        border.color: Theme.accent
    }
    HoverHandler { id: hh; onHoveredChanged: { if (hovered) { tl.hoverId = t.pid; tl.hoverIdx = t.d[2] } } }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: (m) => {
            if (m.button === Qt.RightButton) { tl.forceFocus(); tl.contextRequested(t.pid); return }
            tl.forceFocus()
            if (m.modifiers & Qt.ShiftModifier) { tl.be.range_sel(t.d[2]); tl.bump() }
            else if (tl.selecting) { tl.be.toggle_sel(t.pid); tl.bump() }
            else tl.openItem(t.pid, t.d[2])
        }
    }
    Rectangle {
        visible: hh.hovered || tl.selecting || t.sel
        x: 6; y: 6; width: 24; height: 24; radius: 12
        color: t.sel ? Theme.accent : Theme.scrimSoft
        border.width: 2; border.color: t.sel ? Theme.accent : Theme.onMedia
        Text { anchors.centerIn: parent; text: "✓"; visible: t.sel; color: Theme.accentText; font.pixelSize: 15; font.bold: true }
        MouseArea { anchors.fill: parent; onClicked: { tl.be.toggle_sel(t.pid); tl.bump() } }
    }
}
