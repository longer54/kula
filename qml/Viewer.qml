import QtQuick
import QtQuick.Controls
import org.kula

Rectangle {
    id: vw
    property var be
    property int itemId: -1
    property var info: ({})
    property bool showInfo: false
    property real zoom: 1
    property bool isVideo: info.kind === 1
    property bool imgFailed: false
    color: Theme.viewerBg
    readonly property int picStatus: pic.status
    readonly property bool ready: isVideo ? true : pic.status === Image.Ready
    property string errMsg: ""
    signal closeRequested()
    signal step(int delta)
    signal toggleFav()
    signal albumRequested()
    signal deleteRequested()

    function load(id) {
        itemId = id
        info = JSON.parse(be.item_json(id))
        zoom = 1
        imgFailed = false
        flick.contentX = 0; flick.contentY = 0
    }
    function reloadInfo() { info = JSON.parse(be.item_json(itemId)) }
    function zoomBy(f) { zoom = Math.max(1, Math.min(8, zoom * f)) }
    function toggleZoom() { zoom = zoom > 1.05 ? 1 : 2.5 }
    function togglePlay() { if (isVideo && vl.item) vl.item.toggle() }
    function srcUrl() {
        if (!info.id) return ""
        if (isVideo || imgFailed) return be.thumb_url(info.id)
        return be.file_url(info.id)
    }
    function dateNote() {
        var s = info.date_source
        if (s === "nazwa_whatsapp") return Lang.t("Approximate date: day received in WhatsApp (from file name)")
        if (s === "nazwa_ms") return Lang.t("Approximate date: file save time (timestamp from name)")
        if (s === "brak") return Lang.t("No date in the file, JSON or name")
        return ""
    }

    Flickable {
        id: flick
        anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom
        anchors.right: panel.left
        clip: true
        contentWidth: Math.max(width, width * vw.zoom)
        contentHeight: Math.max(height, height * vw.zoom)
        boundsBehavior: Flickable.StopAtBounds
        interactive: vw.zoom > 1
        Image {
            id: pic
            visible: !vw.isVideo || status === Image.Ready
            width: flick.contentWidth; height: flick.contentHeight
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            autoTransform: true
            cache: false
            source: vw.srcUrl()
            sourceSize.height: vw.isVideo ? 0 : 3200
            onStatusChanged: {
                if (status === Image.Error) {
                    console.log("VIEWER image error: " + source)
                    if (!vw.imgFailed && !vw.isVideo) vw.imgFailed = true
                }
            }
        }
        Loader {
            id: vl
            anchors.fill: parent
            active: vw.isVideo
            source: "VideoPane.qml"
            onLoaded: item.source = vw.be.file_url(vw.itemId)
            Connections { target: vw; function onItemIdChanged() { if (vl.item) vl.item.source = vw.be.file_url(vw.itemId) } }
        }
        WheelHandler {
            onWheel: (e) => vw.zoomBy(e.angleDelta.y > 0 ? 1.15 : 1 / 1.15)
        }
        TapHandler { onDoubleTapped: vw.toggleZoom() }
    }
    Column {
        anchors.centerIn: flick
        spacing: 12
        visible: !vw.isVideo && pic.status !== Image.Ready
        Rectangle {
            id: spin
            anchors.horizontalCenter: parent.horizontalCenter
            width: 34; height: 34; radius: 17; color: "transparent"
            border.width: 3; border.color: Theme.border
            visible: pic.status === Image.Loading || pic.status === Image.Null
            Rectangle { width: 10; height: 10; radius: 5; color: Theme.accent; x: 12; y: -3 }
            RotationAnimator on rotation { from: 0; to: 360; duration: 900; loops: Animation.Infinite; running: spin.visible }
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: pic.status === Image.Error || vw.imgFailed && pic.status === Image.Error ? Lang.t("This photo cannot be displayed (unsupported format or damaged file)") : Lang.t("Loading…")
            color: Theme.textMuted; font.pixelSize: 15
        }
    }

    // top bar
    Rectangle {
        id: top
        anchors.left: parent.left; anchors.right: panel.left; anchors.top: parent.top
        height: 52
        gradient: Gradient { GradientStop { position: 0; color: Theme.barBg } GradientStop { position: 1; color: "transparent" } }
        Row {
            anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter; anchors.leftMargin: 10; spacing: 6
            Btn { text: Lang.t("←  Back"); flat: true; onClicked: vw.closeRequested() }
        }
        Row {
            anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; anchors.rightMargin: 10; spacing: 4
            IconBtn { icon: vw.info.fav ? "favon" : "fav"; role: vw.info.fav ? "g" : "d"; tip: vw.info.fav ? Lang.t("Remove from favorites (F)") : Lang.t("Add to favorites (F)"); onClicked: vw.toggleFav() }
            Btn { text: Lang.t("＋ Album"); flat: true; onClicked: vw.albumRequested() }
            IconBtn { icon: "trash"; tip: Lang.t("Move to trash (Delete)"); onClicked: vw.deleteRequested() }
            IconBtn { icon: "info"; role: vw.showInfo ? "g" : "d"; tip: Lang.t("Photo info (I)"); onClicked: vw.showInfo = !vw.showInfo }
        }
    }
    // arrows
    Rectangle {
        anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter; anchors.leftMargin: 12
        width: 44; height: 44; radius: 22; color: lm.containsMouse ? Theme.scrim : Theme.scrimSoft
        Text { anchors.centerIn: parent; text: "‹"; color: Theme.onMedia; font.pixelSize: 30 }
        MouseArea { id: lm; anchors.fill: parent; hoverEnabled: true; onClicked: vw.step(-1) }
    }
    Rectangle {
        anchors.right: panel.left; anchors.verticalCenter: parent.verticalCenter; anchors.rightMargin: 12
        width: 44; height: 44; radius: 22; color: rm.containsMouse ? Theme.scrim : Theme.scrimSoft
        Text { anchors.centerIn: parent; text: "›"; color: Theme.onMedia; font.pixelSize: 30 }
        MouseArea { id: rm; anchors.fill: parent; hoverEnabled: true; onClicked: vw.step(1) }
    }

    // Info panel
    Rectangle {
        id: panel
        anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom
        width: vw.showInfo ? 340 : 0
        visible: width > 0
        color: Theme.surface
        clip: true
        Behavior on width { NumberAnimation { duration: 120 } }
        Flickable {
            anchors.fill: parent; anchors.margins: 18
            contentHeight: col.implicitHeight
            clip: true
            Column {
                id: col
                width: 304
                spacing: 12
                Text { text: Lang.t("Info"); color: Theme.text; font.pixelSize: 20; font.bold: true }
                TextField {
                    id: desc
                    width: parent.width
                    placeholderText: Lang.t("Add a description")
                    text: vw.info.descr || ""
                    color: Theme.text; placeholderTextColor: Theme.textMuted
                    background: Rectangle { color: Theme.window; radius: 6; border.color: desc.activeFocus ? Theme.accent : Theme.border }
                    onEditingFinished: { vw.be.set_desc(vw.itemId, text); vw.focusOut() }
                }
                Row {
                    spacing: 8
                    Text { text: "📅"; color: Theme.accent; font.pixelSize: 16 }
                    Column {
                        Text { text: vw.info.date || ""; color: Theme.text; font.pixelSize: 14 }
                        Text { text: vw.info.name || ""; color: Theme.textMuted; font.pixelSize: 12 }
                    }
                    Text {
                        visible: vw.info.date_quality === "approx" || vw.info.date_quality === "none"
                        text: "≈"; color: Theme.accent; font.pixelSize: 18; font.bold: true
                        ToolTip.visible: dm.containsMouse; ToolTip.text: vw.dateNote()
                        MouseArea { id: dm; anchors.fill: parent; hoverEnabled: true }
                    }
                }
                Text { visible: vw.dateNote() !== ""; width: parent.width; wrapMode: Text.WordWrap; text: "≈ " + vw.dateNote(); color: Theme.textSubtle; font.pixelSize: 11 }
                Text {
                    visible: (vw.info.cam || "") !== ""
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "📷  " + (vw.info.cam || "") + ((vw.info.lens || "") !== "" ? "\n      " + vw.info.lens : "")
                    color: Theme.text; font.pixelSize: 13
                }
                Text {
                    visible: (vw.info.fnum || 0) > 0 || (vw.info.iso || 0) > 0
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "ƒ/" + (vw.info.fnum || 0) + "    " + (vw.info.exposure || "") + " s    ISO " + (vw.info.iso || 0) + "    " + (vw.info.focal || 0) + " mm"
                    color: Theme.text; font.pixelSize: 13
                }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "🖼  " + (vw.info.w || 0) + " × " + (vw.info.h || 0) + (vw.info.kind === 1 ? "    " + Math.round(vw.info.dur || 0) + " s" : "    " + (vw.info.mpix || 0) + " Mpix")
                    color: Theme.text; font.pixelSize: 13
                }
                Text {
                    width: parent.width; wrapMode: Text.WrapAnywhere
                    text: "📁  " + (vw.info.rel || "") + "\n      " + Math.round((vw.info.size || 0) / 1048576 * 10) / 10 + " MB"
                    color: Theme.text; font.pixelSize: 12
                }
                Row {
                    spacing: 8
                    Btn { text: Lang.t("Show in folder"); pad: 10; onClicked: vw.be.show_in_folder(vw.itemId) }
                    Btn { text: Lang.t("Copy path"); pad: 10; onClicked: vw.be.copy_text(vw.info.abs || "") }
                }
                Row {
                    visible: vw.info.lat !== null && vw.info.lat !== undefined
                    spacing: 8
                    Text {
                        text: "📍  " + (vw.info.lat !== null && vw.info.lat !== undefined ? Number(vw.info.lat).toFixed(5) + ", " + Number(vw.info.lon).toFixed(5) : "")
                        color: Theme.text; font.pixelSize: 13
                    }
                    Btn { text: Lang.t("Copy"); pad: 8; onClicked: vw.be.copy_text(Number(vw.info.lat).toFixed(6) + ", " + Number(vw.info.lon).toFixed(6)) }
                }
                Text { visible: (vw.info.people || "") !== ""; width: parent.width; wrapMode: Text.WordWrap; text: "👤  " + (vw.info.people || ""); color: Theme.text; font.pixelSize: 13 }
                Text {
                    visible: (vw.info.albums || []).length > 0
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "🗂  " + (vw.info.albums || []).join(", ")
                    color: Theme.text; font.pixelSize: 13
                }
            }
        }
    }
    signal focusOut()
}
