import QtQuick
import QtQuick.Controls
import org.kula

Item {
    id: ps
    property var be
    property var all: []
    property var list: []
    property bool showAll: false
    property var picked: ({})
    property int pickedCount: 0
    property int hiddenCount: 0
    signal openPerson(int id, string name)
    signal toast(string t)

    function reload() {
        all = JSON.parse(be.persons_json())
        applyFilter()
        retry.running = all.some(function(p) { return p.face === "" })
    }
    function applyFilter() {
        var out = [], hid = 0
        for (var i = 0; i < all.length; i++) {
            var p = all[i]
            if (showAll || p.count >= 3 || p.name !== "" || picked[p.id]) out.push(p)
            else hid++
        }
        list = out; hiddenCount = hid
    }
    onShowAllChanged: applyFilter()
    function toggle(id) {
        var m = Object.assign({}, picked)
        if (m[id]) delete m[id]; else m[id] = true
        picked = m
        pickedCount = Object.keys(m).length
    }
    function clearPick() { picked = ({}); pickedCount = 0 }
    function doMerge() {
        var ids = Object.keys(picked).map(function(k) { return parseInt(k) })
        ids.sort(function(a, b) {
            var pa = all.find(function(p) { return p.id === a }), pb = all.find(function(p) { return p.id === b })
            return (pb ? pb.count : 0) - (pa ? pa.count : 0)
        })
        var moved = be.persons_merge(JSON.stringify(ids))
        clearPick()
        reload()
        ps.toast(Lang.f("Merged %1 people (%2 faces)", ids.length, moved))
    }
    function plural(n) { return Lang.noun(n, "photo") }
    Component.onCompleted: reload()
    onVisibleChanged: if (visible) reload()
    Timer { id: retry; interval: 900; repeat: true; onTriggered: ps.reload() }

    Item {
        id: head
        x: 24; y: 10; height: 40; width: parent.width - 48
        Text { id: ttl; anchors.verticalCenter: parent.verticalCenter; text: Lang.t("People"); color: Theme.text; font.pixelSize: 24; font.bold: true }
        Text {
            anchors.left: ttl.right; anchors.leftMargin: 14; anchors.verticalCenter: parent.verticalCenter
            visible: ps.pickedCount === 0
            text: ps.list.length + (ps.hiddenCount > 0 ? " z " + ps.all.length : "")
            color: Theme.textMuted; font.pixelSize: 13
        }
        Row {
            anchors.left: ttl.right; anchors.leftMargin: 20; anchors.verticalCenter: parent.verticalCenter; spacing: 8
            visible: ps.pickedCount > 0
            Text { anchors.verticalCenter: parent.verticalCenter; text: Lang.t("Selected: ") + ps.pickedCount; color: Theme.accent; font.pixelSize: 15; font.bold: true; rightPadding: 6 }
            Btn { text: Lang.t("Merge"); accent: true; visible: ps.pickedCount >= 2; onClicked: ps.doMerge() }
            Btn { text: Lang.t("Cancel"); flat: true; onClicked: ps.clearPick() }
        }
        Btn {
            anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
            visible: ps.hiddenCount > 0 || ps.showAll
            text: ps.showAll ? Lang.t("Hide rare people") : Lang.f("Show all (hidden: %1)", ps.hiddenCount)
            onClicked: ps.showAll = !ps.showAll
        }
    }
    Text {
        id: hint
        anchors.left: head.left; anchors.top: head.bottom
        text: Lang.t("Select several people with the circle and click “Merge”. Click a name to rename it.")
        color: Theme.textMuted; font.pixelSize: 12
    }
    GridView {
        id: gv
        anchors.top: hint.bottom; anchors.topMargin: 10
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.leftMargin: 24; clip: true
        cellWidth: 170; cellHeight: 214
        model: ps.list
        ScrollBar.vertical: ScrollBar {}
        delegate: Item {
            id: c
            required property var modelData
            property bool sel: ps.picked[modelData.id] === true
            width: 170; height: 214
            Item {
                id: circ
                x: 10; y: 6; width: 150; height: 150
                Rectangle { anchors.fill: parent; radius: 75; color: Theme.surface; border.width: c.sel ? 3 : (pm.containsMouse ? 2 : 0); border.color: Theme.accent }
                Image {
                    anchors.fill: parent; anchors.margins: c.sel ? 6 : 0
                    source: c.modelData.face !== "" ? c.modelData.face : ""
                    sourceSize: Qt.size(300, 300)
                    asynchronous: true
                    fillMode: Image.PreserveAspectCrop
                    opacity: status === Image.Ready ? 1 : 0
                    Behavior on opacity { NumberAnimation { duration: Theme.normal } }
                    Behavior on anchors.margins { NumberAnimation { duration: Theme.fast } }
                }
                MouseArea {
                    id: pm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                    onClicked: (m) => {
                        if (ps.pickedCount > 0 || (m.modifiers & Qt.ControlModifier)) ps.toggle(c.modelData.id)
                        else ps.openPerson(c.modelData.id, c.modelData.name)
                    }
                }
                Rectangle {
                    visible: pm.containsMouse || ps.pickedCount > 0
                    x: 6; y: 6; width: 26; height: 26; radius: 13
                    color: c.sel ? Theme.accent : Theme.scrimSoft; border.width: 2; border.color: c.sel ? Theme.accent : Theme.onMedia
                    Image { anchors.centerIn: parent; width: 16; height: 16; sourceSize: Qt.size(32, 32); visible: c.sel; source: Theme.icon_("check", "k") }
                    MouseArea { anchors.fill: parent; onClicked: ps.toggle(c.modelData.id) }
                }
            }
            Item {
                x: 6; y: 160; width: 158; height: 22
                Text {
                    visible: !ed.visible
                    anchors.centerIn: parent; width: parent.width - 16; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                    text: c.modelData.name !== "" ? c.modelData.name : Lang.t("Unnamed")
                    color: c.modelData.name !== "" ? Theme.text : Theme.textMuted; font.pixelSize: 14; font.bold: c.modelData.name !== ""
                }
                Image {
                    visible: !ed.visible && nmm.containsMouse
                    anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                    width: 14; height: 14; sourceSize: Qt.size(28, 28); source: Theme.icon_("edit", "d")
                }
                MouseArea { id: nmm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.IBeamCursor; onClicked: { ed.text = c.modelData.name; ed.visible = true; ed.forceActiveFocus(); ed.selectAll() } }
                TextField {
                    id: ed
                    visible: false
                    anchors.fill: parent; padding: 2; font.pixelSize: 13; horizontalAlignment: Text.AlignHCenter
                    color: Theme.text; selectByMouse: true
                    background: Rectangle { color: Theme.surface; radius: 4; border.width: 1; border.color: Theme.accent }
                    onAccepted: { ps.be.person_name_set(c.modelData.id, text); visible = false; ps.reload() }
                    Keys.onEscapePressed: visible = false
                    onActiveFocusChanged: if (!activeFocus) visible = false
                }
            }
            Text { x: 6; y: 184; width: 158; horizontalAlignment: Text.AlignHCenter; text: Theme.fmt(c.modelData.count) + " " + ps.plural(c.modelData.count); color: Theme.textMuted; font.pixelSize: 12 }
        }
    }
    Text { anchors.centerIn: parent; visible: ps.all.length === 0; text: Lang.t("No recognized people (ai.sqlite)."); color: Theme.textMuted; font.pixelSize: 16 }
}
