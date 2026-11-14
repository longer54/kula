import QtQuick
import QtQuick.Controls
import org.kula

Item {
    id: tlr
    property var be
    property int zoomLevel: 3
    readonly property var rowHs: [90, 130, 165, 200, 260]
    property var rows: []
    property var marks: []
    property var rowY: []
    property real padL: 20
    property int vFirst: -1
    property int vLast: -1
    readonly property real maxY: Math.max(0, total - lv.height)
    readonly property alias view: lv
    readonly property real scrollY: lv.contentY
    property var scrubMarks: []
    property real total: 0
    property int itemCount: 0
    property int hoverId: -1
    property int hoverIdx: -1
    property int selVer: 0
    property int selCount: 0
    property bool selecting: selCount > 0
    property string emptyText: Lang.t("No photos in this view.")
    property bool scanning: false
    signal openItem(int id, int idx)
    signal focusRequested()
    signal contextRequested(int id)

    function forceFocus() { focusRequested() }
    function bump() { selCount = be.sel_count(); selVer++ }

    function findRow(y) {
        var n = rowY.length
        if (n === 0) return -1
        var lo = 0, hi = n - 1
        while (lo < hi) {
            var mid = (lo + hi + 1) >> 1
            if (rowY[mid] <= y) lo = mid; else hi = mid - 1
        }
        return lo
    }
    // virtualization: only rows in the viewport (+buffer); ListModel changed incrementally
    function syncVis() {
        var n = rows.length
        if (n === 0 || lv.height <= 0) { vis.clear(); vFirst = -1; vLast = -1; return }
        var buf = 700
        var f = findRow(lv.contentY - buf), l = findRow(lv.contentY + lv.height + buf)
        if (f < 0) f = 0
        if (l < f) l = f
        if (vFirst < 0 || f > vLast || l < vFirst) {
            vis.clear()
            for (var i = f; i <= l; i++) vis.append({ ri: i })
            vFirst = f; vLast = l
            return
        }
        while (vFirst > f) { vFirst--; vis.insert(0, { ri: vFirst }) }
        while (vLast < l) { vLast++; vis.append({ ri: vLast }) }
        while (vFirst < f) { vis.remove(0); vFirst++ }
        while (vLast > l) { vis.remove(vis.count - 1); vLast-- }
    }
    function clampY(y) { return Math.max(0, Math.min(y, Math.max(0, total - lv.height))) }
    function setY(y) { lv.contentY = clampY(y); syncVis() }

    // first visible element (id, index, row offset) — for anchoring after changes
    function anchorInfo() {
        var ri = findRow(lv.contentY)
        if (ri < 0) return null
        for (var k = ri; k < rows.length; k++) {
            if (rows[k].t === "r") {
                var ids = []
                for (var j = k; j < rows.length && ids.length < 80; j++) {
                    if (rows[j].t === "r") for (var q = 0; q < rows[j].it.length; q++) ids.push(rows[j].it[q][0])
                }
                return { ids: ids, first: rows[k].it[0][0], off: lv.contentY - rowY[k] }
            }
        }
        return null
    }
    function firstVisible() {
        var ri = findRow(lv.contentY)
        for (var k = Math.max(0, ri); k < rows.length; k++)
            if (rows[k].t === "r" && rowY[k] + rows[k].h > lv.contentY) return { id: rows[k].it[0][0], idx: rows[k].it[0][2], row: k, y: rowY[k] }
        return null
    }
    function relayout(resetTop) {
        if (!be || lv.width < 120) return
        var a = (resetTop === true) ? null : anchorInfo()
        var y0 = lv.contentY
        var o = JSON.parse(be.layout(lv.width - padL - 10, rowHs[zoomLevel]))
        var ys = [], acc = 0
        for (var i = 0; i < o.rows.length; i++) { ys.push(acc); acc += o.rows[i].h }
        rowY = ys; rows = o.rows; marks = o.marks; total = o.total; itemCount = o.count
        lv.contentHeight = o.total
        var ny = 0
        if (resetTop === true) ny = 0
        else if (a) {
            var want = {}
            for (var q = 0; q < a.ids.length; q++) want[a.ids[q]] = q
            var best = -1, bestRow = -1
            for (var r = 0; r < rows.length && bestRow < 0; r++) {
                if (rows[r].t !== "r") continue
                for (var k = 0; k < rows[r].it.length; k++) {
                    var qi = want[rows[r].it[k][0]]
                    if (qi !== undefined && (best < 0 || qi < best)) best = qi
                }
                if (best >= 0) bestRow = r
            }
            if (bestRow >= 0) ny = rowY[bestRow] + (rows[bestRow].it[0][0] === a.first ? a.off : Math.min(0, a.off))
            else ny = y0
        } else ny = y0
        lv.contentY = clampY(ny)
        buildScrub()
        bump()
        syncVis()
    }
    function buildScrub() {
        var out = [], last = -100, th = scrubThumbH, h = Math.max(1, scrub.height - th)
        var mx = Math.max(1, total - lv.height)
        for (var i = 0; i < marks.length; i++) {
            var f = Math.min(1, marks[i][0] / mx)
            var p = th / 2 + f * h
            if (p - last >= 24 && p <= scrub.height - 12) { out.push({ y: p, t: marks[i][1] }); last = p }
        }
        scrubMarks = out
    }
    property real scrubThumbH: 28
    function scrollToIndex(idx) {
        for (var i = 0; i < rows.length; i++) {
            var r = rows[i]
            if (r.t === "r") for (var k = 0; k < r.it.length; k++) if (r.it[k][2] === idx) { setY(rowY[i] - 60); return }
        }
    }
    function scrubTo(f) { sm.go(scrubThumbH / 2 + f * (scrub.height - scrubThumbH)) }
    function visibleTiles() {
        var out = []
        function walk(it) {
            for (var i = 0; i < it.children.length; i++) {
                var c = it.children[i]
                if (c.pid !== undefined && c.url !== undefined) out.push(c)
                walk(c)
            }
        }
        walk(lv.contentItem)
        return out
    }
    function toTop() { setY(0) }
    function toBottom() { setY(total) }
    function pageBy(dir) { setY(lv.contentY + dir * Math.max(100, lv.height - 80)) }
    function labelAt(cy) {
        var l = ""
        for (var i = 0; i < marks.length; i++) { if (marks[i][0] <= cy + 4) l = marks[i][1]; else break }
        return l
    }
    onWidthChanged: debounce.restart()
    onZoomLevelChanged: relayout()
    Timer { id: debounce; interval: 120; onTriggered: tlr.relayout() }
    ListModel { id: vis }

    Flickable {
        id: lv
        anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; anchors.right: scrub.left
        anchors.rightMargin: 12
        clip: true
        contentWidth: width
        contentHeight: tlr.total
        flickableDirection: Flickable.VerticalFlick
        boundsBehavior: Flickable.StopAtBounds
        flickDeceleration: 2600
        maximumFlickVelocity: 9000
        onContentYChanged: tlr.syncVis()
        onHeightChanged: { tlr.syncVis(); tlr.buildScrub() }
        Repeater {
            model: vis
        delegate: Item {
            id: rowItem
            required property int ri
            property var modelData: ri < tlr.rows.length ? tlr.rows[ri] : ({ t: "s", h: 0, it: [] })
            x: tlr.padL
            y: ri < tlr.rowY.length ? tlr.rowY[ri] : 0
            width: lv.width - tlr.padL
            height: modelData.h
            Text {
                visible: rowItem.modelData.t === "h"
                x: (hdrHover.hovered || tlr.selecting) && !tlr.listOnly ? 32 : 0
                Behavior on x { NumberAnimation { duration: 110 } }
                anchors.verticalCenter: parent.verticalCenter
                text: rowItem.modelData.title || ""
                color: Theme.text
                font.pixelSize: 16
                font.bold: true
            }
            Rectangle {
                visible: rowItem.modelData.t === "h" && !tlr.listOnly && (hdrHover.hovered || on || tlr.selecting)
                x: 0; anchors.verticalCenter: parent.verticalCenter
                width: 22; height: 22; radius: 11
                property bool on: { tlr.selVer; return rowItem.modelData.t === "h" && tlr.be.group_selected(rowItem.modelData.a, rowItem.modelData.b) }
                color: on ? Theme.accent : "transparent"
                border.width: 2; border.color: on ? Theme.accent : Theme.textMuted
                Text { anchors.centerIn: parent; text: "✓"; visible: parent.on; color: Theme.accentText; font.pixelSize: 14; font.bold: true }
                MouseArea {
                    anchors.fill: parent
                    onClicked: { tlr.be.sel_group(rowItem.modelData.a, rowItem.modelData.b, !parent.on); tlr.bump() }
                }
            }
            HoverHandler { id: hdrHover; enabled: rowItem.modelData.t === "h" }
            Row {
                x: 0
                spacing: 4
                Repeater {
                    model: rowItem.modelData.t === "r" ? rowItem.modelData.it : []
                    delegate: Tile {
                        required property var modelData
                        tl: tlr
                        d: modelData
                        th: rowItem.modelData.h - 4
                    }
                }
            }
        }
        }
        WheelHandler {
            acceptedModifiers: Qt.ControlModifier
            onWheel: (e) => { tlr.zoomLevel = Math.max(0, Math.min(tlr.rowHs.length - 1, tlr.zoomLevel + (e.angleDelta.y > 0 ? 1 : -1))) }
        }
    }
    property bool listOnly: false

    // date slider: handle position continuous and monotonic, f = contentY / (total - height); slider bottom = end of list
    Item {
        id: scrub
        anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom
        width: 84
        onHeightChanged: tlr.buildScrub()
        readonly property real fr: tlr.maxY > 0 ? Math.max(0, Math.min(1, lv.contentY / tlr.maxY)) : 0
        readonly property real thumbY: tlr.scrubThumbH / 2 + fr * (scrub.height - tlr.scrubThumbH)
        Repeater {
            model: tlr.scrubMarks
            delegate: Text {
                required property var modelData
                y: modelData.y - height / 2
                anchors.right: parent.right; anchors.rightMargin: 20
                text: modelData.t
                color: sm.containsMouse || sm.pressed ? Theme.text : Theme.textSubtle
                font.pixelSize: 13
                font.bold: sm.pressed && tlr.labelAt(lv.contentY) === modelData.t
            }
        }
        Rectangle {
            id: track
            visible: tlr.total > lv.height
            width: 2; radius: 1
            anchors.right: parent.right; anchors.rightMargin: 9
            anchors.top: parent.top; anchors.bottom: parent.bottom
            color: Theme.border
        }
        Rectangle {
            id: thumb
            visible: tlr.total > lv.height
            width: 8; radius: 4
            anchors.right: parent.right; anchors.rightMargin: 6
            color: Theme.accent
            height: tlr.scrubThumbH
            y: scrub.thumbY - height / 2
        }
        MouseArea {
            id: sm
            anchors.fill: parent
            hoverEnabled: true
            function go(my) {
                var th = tlr.scrubThumbH
                var f = Math.max(0, Math.min(1, (my - th / 2) / Math.max(1, scrub.height - th)))
                tlr.setY(f * tlr.maxY)
            }
            onPressed: (m) => go(m.y)
            onPositionChanged: (m) => { if (pressed) go(m.y) }
        }
        Rectangle {
            visible: sm.pressed
            anchors.right: parent.left; anchors.rightMargin: 6
            y: Math.max(0, Math.min(scrub.height - height, scrub.thumbY - height / 2))
            height: 32; width: bt.implicitWidth + 24; radius: 16
            color: Theme.accent
            Text { id: bt; anchors.centerIn: parent; text: tlr.labelAt(lv.contentY); color: Theme.accentText; font.bold: true; font.pixelSize: 14 }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: tlr.itemCount === 0
        text: tlr.scanning ? Lang.t("Scanning library…") : tlr.emptyText
        color: Theme.textMuted
        font.pixelSize: 17
    }
}
