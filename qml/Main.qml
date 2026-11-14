import QtQuick
import QtQuick.Controls
import QtQuick.Window
import org.kula

Window {
    id: win
    width: 1360; height: 860; minimumWidth: 760; minimumHeight: 520
    visible: true
    title: "Kula"
    color: Theme.window
    flags: Qt.Window | Qt.FramelessWindowHint

    Backend { id: be }
    Rectangle { anchors.fill: parent; color: Theme.window; z: -1 }

    property string page: "grid"        // grid | albums | persons | settings
    property string viewKind: "timeline"
    property string viewArg: ""
    property string viewTitle: Lang.t("Photos")
    property string prevKind: "timeline"
    property string prevTitle: Lang.t("Photos")
    property int lastVer: -1
    property var status: ({})
    property var counts: ({})
    property bool hasPersons: false
    property bool semantic: false
    property bool semMode: false
    property string toast: ""
    property int pendingAlbumTarget: -1
    property bool sideCollapsed: false
    function toggleSide() { sideCollapsed = !sideCollapsed; be.setting_set("side_collapsed", sideCollapsed ? "1" : "0") }

    function openView(kind, arg, title) {
        viewKind = kind; viewArg = arg || ""; viewTitle = title
        be.set_view(kind, viewArg)
        page = "grid"
        grid.relayout(true)
    }
    function goSide(key) {
        searchField.text = ""
        if (key === "albums" || key === "persons" || key === "settings") {
            page = key
            if (key === "settings") settings.reload()
            return
        }
        var titles = { timeline: Lang.t("Photos"), fav: Lang.t("Favorites"), video: Lang.t("Videos"), recent: Lang.t("Recently added"), archive: Lang.t("Archive"), trash: Lang.t("Trash"), nodate: Lang.t("No date") }
        openView(key, "", titles[key])
    }
    function loadLang() { var m = be.setting_get("lang"); Lang.mode = (m === "pl" || m === "en") ? m : "auto"; be.set_lang(Lang.pl ? "pl" : "en") }
    Connections { target: Lang; function onPlChanged() { be.set_lang(Lang.pl ? "pl" : "en"); win.refreshAll() } }
    function loadTheme() { var m = be.setting_get("theme"); Theme.mode = (m === "light" || m === "system") ? m : "dark" }
    Component.onCompleted: { loadTheme(); loadLang(); sideCollapsed = be.setting_get("side_collapsed") === "1" }
    function say(t) { toast = t; toastTimer.restart() }
    function refreshAll() {
        counts = JSON.parse(be.counts_json())
        hasPersons = be.has_persons()
        semantic = be.semantic_available()
        grid.relayout()
        if (page === "albums") albumsPage.reload()
    }
    function viewerOpen() { return viewer.visible }
    function openViewer(id) { viewer.load(id); viewer.visible = true; root.forceActiveFocus() }
    function closeViewer() { viewer.visible = false; if (viewer.isVideo && viewer.itemId >= 0) { } ; root.forceActiveFocus() }
    function stepViewer(d) {
        var idx = be.index_of(viewer.itemId)
        if (idx < 0) return
        var n = be.id_at(idx + d)
        if (n >= 0) viewer.load(n)
    }
    function doMark(kind, val) {
        var inV = viewerOpen()
        var idx = inV ? be.index_of(viewer.itemId) : -1
        var n = be.mark(kind, inV ? viewer.itemId : -1, val)
        if (n === 0) { say(Lang.t("Select photos first")); return }
        grid.relayout()
        counts = JSON.parse(be.counts_json())
        if (kind === "fav") say(val === 0 ? Lang.t("Removed from favorites") : Lang.t("Favorites changed: ") + n)
        else if (kind === "archived") say(Lang.t("Archived: ") + n)
        else say(Lang.t("Done: ") + n)
        if (inV) {
            var cnt = be.count()
            if (cnt === 0) { closeViewer(); return }
            var cur = be.index_of(viewer.itemId)
            if (cur < 0) viewer.load(be.id_at(Math.min(idx, cnt - 1)))
            else viewer.reloadInfo()
        }
    }
    // ---- deletion (always after confirmation, with file count) ----
    property string confirmMode: ""
    property int confirmId: -1
    property int confirmN: 0
    function currentTarget() { return viewerOpen() ? viewer.itemId : -1 }
    function askDelete() {
        var id = currentTarget()
        var n = be.target_count(id)
        if (n === 0) { say(Lang.t("Select photos first")); return }
        confirmId = id; confirmN = n
        confirmMode = viewKind === "trash" ? "purge" : "trash"
        confirmPop.open()
    }
    function doRestore() {
        var n = be.target_count(-1)
        if (n === 0) { say(Lang.t("Select photos first")); return }
        report(JSON.parse(be.restore_items(-1)), Lang.t("Restored"))
    }
    function report(r, verb) {
        refreshAll()
        if (r.failed > 0) { errText = r.errors.join("\n"); errPop.open() }
        else say(verb + ": " + r.ok)
    }
    property string errText: ""
    function runConfirmed() {
        var inV = viewerOpen(), idx = inV ? be.index_of(viewer.itemId) : -1
        var r = confirmMode === "purge" ? JSON.parse(be.purge_items(confirmId)) : JSON.parse(be.delete_items(confirmId))
        confirmPop.close()
        report(r, confirmMode === "purge" ? Lang.t("Deleted permanently") : Lang.t("Moved to trash"))
        if (inV) {
            var cnt = be.count()
            if (cnt === 0) { closeViewer(); return }
            var cur = be.index_of(viewer.itemId)
            if (cur < 0) viewer.load(be.id_at(Math.min(idx, cnt - 1)))
        }
    }
    function revealCurrent() {
        var id = viewerOpen() ? viewer.itemId : (grid.selCount === 1 ? -1 : grid.hoverId)
        if (id < 0 && grid.selCount !== 1) { say(Lang.t("Select a single photo")); return }
        if (id < 0) id = -1
        be.show_in_folder(id < 0 ? be.id_at(-1) : id); say(Lang.t("Opening folder…"))
    }
    function askAlbum() { albumPicker.refresh(); albumPicker.open() }
    function runSearch() {
        var t = searchField.text.trim()
        if (t === "") { if (viewKind === "search" || viewKind === "ai") openView(prevKind, "", prevTitle); return }
        if (viewKind !== "search" && viewKind !== "ai") { prevKind = viewKind; prevTitle = viewTitle }
        if (semMode) openView("ai", t, Lang.t("Content: ") + t)
        else openView("search", t, Lang.t("Results: ") + t)
    }

    Item {
        id: root
        anchors.fill: parent
        focus: true
        Component.onCompleted: forceActiveFocus()

        Keys.onPressed: (e) => {
            var ctrl = e.modifiers & Qt.ControlModifier
            var shift = e.modifiers & Qt.ShiftModifier
            if (e.key === Qt.Key_Escape) {
                if (helpPop.opened) helpPop.close()
                else if (viewerOpen()) closeViewer()
                else if (grid.selCount > 0) { be.clear_sel(); grid.bump() }
                else if (viewKind === "search" || viewKind === "ai") { searchField.text = ""; openView(prevKind, "", prevTitle) }
                else if (page !== "grid") page = "grid"
                e.accepted = true; return
            }
            if (e.text === "/" ) { searchField.forceActiveFocus(); searchField.selectAll(); e.accepted = true; return }
            if (e.key === Qt.Key_F11) { win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen; e.accepted = true; return }
            if (e.key === Qt.Key_B && ctrl) { toggleSide(); e.accepted = true; return }
            if (e.key === Qt.Key_O && ctrl && shift) { revealCurrent(); e.accepted = true; return }
            if (e.key === Qt.Key_F1 || e.text === "?") { helpPop.open(); e.accepted = true; return }
            if (viewerOpen()) {
                if (e.key === Qt.Key_Left) { stepViewer(-1); e.accepted = true }
                else if (e.key === Qt.Key_Right) { stepViewer(1); e.accepted = true }
                else if (e.key === Qt.Key_Z) { viewer.toggleZoom(); e.accepted = true }
                else if (e.key === Qt.Key_Plus || e.key === Qt.Key_Equal) { viewer.zoomBy(1.25); e.accepted = true }
                else if (e.key === Qt.Key_Minus) { viewer.zoomBy(0.8); e.accepted = true }
                else if (e.key === Qt.Key_0) { viewer.zoom = 1; e.accepted = true }
                else if (e.key === Qt.Key_I) { viewer.showInfo = !viewer.showInfo; e.accepted = true }
                else if (e.key === Qt.Key_Space) { viewer.togglePlay(); e.accepted = true }
                else if (e.key === Qt.Key_F && !ctrl) { doMark("fav", -1); e.accepted = true }
                else if (e.key === Qt.Key_A && shift) { doMark("archived", -1); e.accepted = true }
                else if (e.key === Qt.Key_Delete || e.text === "#") { askDelete(); e.accepted = true }
                return
            }
            if (page !== "grid") return
            if (e.key === Qt.Key_PageDown) { grid.pageBy(1); e.accepted = true; return }
            if (e.key === Qt.Key_PageUp) { grid.pageBy(-1); e.accepted = true; return }
            if (e.key === Qt.Key_Home) { grid.toTop(); e.accepted = true; return }
            if (e.key === Qt.Key_End) { grid.toBottom(); e.accepted = true; return }
            if (e.key === Qt.Key_X && !ctrl) {
                if (grid.hoverId >= 0) { be.toggle_sel(grid.hoverId); grid.bump() }
                e.accepted = true
            } else if (e.key === Qt.Key_A && ctrl) { be.sel_all(); grid.bump(); e.accepted = true }
            else if (e.key === Qt.Key_F && !ctrl) { doMark("fav", -1); e.accepted = true }
            else if (e.key === Qt.Key_A && shift) { doMark("archived", -1); e.accepted = true }
            else if (e.key === Qt.Key_Delete || e.text === "#") { askDelete(); e.accepted = true }
            else if (e.key === Qt.Key_Plus || e.key === Qt.Key_Equal) { grid.zoomLevel = Math.min(4, grid.zoomLevel + 1); e.accepted = true }
            else if (e.key === Qt.Key_Minus) { grid.zoomLevel = Math.max(0, grid.zoomLevel - 1); e.accepted = true }
        }

        // ---------- title bar ----------
        Rectangle {
            id: titleBar
            anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
            height: 52
            color: Theme.sidebar
            DragHandler { onActiveChanged: if (active) win.startSystemMove(); target: null }
            TapHandler { acceptedButtons: Qt.LeftButton; onDoubleTapped: win.visibility = win.visibility === Window.Maximized ? Window.Windowed : Window.Maximized }
            Row {
                anchors.left: parent.left; anchors.leftMargin: 8; anchors.verticalCenter: parent.verticalCenter; spacing: 8
                Rectangle {
                    id: burger
                    width: 36; height: 36; radius: 18; anchors.verticalCenter: parent.verticalCenter
                    color: bm.containsMouse ? Theme.navSelected : "transparent"
                    Image { anchors.centerIn: parent; width: 22; height: 22; sourceSize: Qt.size(44, 44); source: Theme.icon_("menu", "d") }
                    MouseArea { id: bm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { win.toggleSide(); root.forceActiveFocus() } }
                    ToolTip.visible: bm.containsMouse; ToolTip.delay: 500
                    ToolTip.text: win.sideCollapsed ? Lang.t("Show side panel (Ctrl+B)") : Lang.t("Hide side panel (Ctrl+B)")
                }
                Image { source: "qrc:/qt/qml/org/kula/icons/kula-32.png"; width: 28; height: 28; sourceSize: Qt.size(56, 56); anchors.verticalCenter: parent.verticalCenter }
                Text { text: "Kula"; color: Theme.accent; font.pixelSize: 20; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
            }
            Rectangle {
                id: searchBox
                anchors.centerIn: parent
                width: Math.min(560, parent.width - 420); height: 36; radius: 18
                color: Theme.surface; border.width: searchField.activeFocus ? 1 : 0; border.color: Theme.accent
                Image { x: 14; anchors.verticalCenter: parent.verticalCenter; width: 18; height: 18; sourceSize: Qt.size(36, 36); source: Theme.icon_("search", "d") }
                TextField {
                    id: searchField
                    anchors.left: parent.left; anchors.leftMargin: 38
                    anchors.right: slashBadge.left; anchors.rightMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    placeholderText: semMode ? Lang.t("Describe what you are looking for (e.g. beach at sunset)") : Lang.t("Search photos")
                    placeholderTextColor: Theme.textMuted
                    color: Theme.text; font.pixelSize: 14
                    selectByMouse: true
                    background: Item {}
                    onTextChanged: if (!semMode) searchTimer.restart()
                    onAccepted: { searchTimer.stop(); win.runSearch(); root.forceActiveFocus() }
                    Keys.onEscapePressed: { text = ""; root.forceActiveFocus() }
                }
                Rectangle {
                    id: slashBadge
                    anchors.right: chip.left; anchors.rightMargin: 8; anchors.verticalCenter: parent.verticalCenter
                    width: 20; height: 20; radius: 5; color: "transparent"; border.width: 1; border.color: Theme.border
                    visible: searchField.text === "" && !searchField.activeFocus
                    Text { anchors.centerIn: parent; text: "/"; color: Theme.textMuted; font.pixelSize: 12 }
                }
                Rectangle {
                    id: chip
                    visible: win.semantic
                    anchors.right: parent.right; anchors.rightMargin: 8; anchors.verticalCenter: parent.verticalCenter
                    width: visible ? chipRow.width + 20 : 0; height: 26; radius: 13
                    color: win.semMode ? Theme.accent : "transparent"; border.width: 1; border.color: win.semMode ? Theme.accent : Theme.border
                    Row {
                        id: chipRow; anchors.centerIn: parent; spacing: 6
                        Image { width: 14; height: 14; anchors.verticalCenter: parent.verticalCenter; sourceSize: Qt.size(28, 28); source: Theme.icon_("ai", win.semMode ? "k" : "d") }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: Lang.t("Search by content (AI)"); color: win.semMode ? Theme.accentText : Theme.textMuted; font.pixelSize: 12 }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { win.semMode = !win.semMode; if (searchField.text !== "") win.runSearch() } }
                }
            }
            Timer { id: searchTimer; interval: 280; onTriggered: win.runSearch() }
            Row {
                anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom
                Repeater {
                    model: [{ k: "min" }, { k: "max" }, { k: "close" }]
                    delegate: Rectangle {
                        required property var modelData
                        width: 50; height: parent.height
                        color: wm.containsMouse ? (modelData.k === "close" ? Theme.danger : Theme.navSelected) : "transparent"
                        Image { anchors.centerIn: parent; width: 18; height: 18; sourceSize: Qt.size(36, 36); source: Theme.icon_(modelData.k, "t") }
                        MouseArea {
                            id: wm; anchors.fill: parent; hoverEnabled: true
                            onClicked: {
                                if (modelData.k === "min") win.showMinimized()
                                else if (modelData.k === "max") win.visibility = win.visibility === Window.Maximized ? Window.Windowed : Window.Maximized
                                else Qt.quit()
                            }
                        }
                    }
                }
            }
        }

        // ---------- side panel ----------
        Rectangle {
            id: side
            anchors.left: parent.left; anchors.top: titleBar.bottom; anchors.bottom: parent.bottom
            width: win.sideCollapsed ? 56 : 210; color: Theme.sidebar
            clip: true
            Behavior on width { NumberAnimation { duration: Theme.normal; easing.type: Theme.easing } }
            Column {
                anchors.fill: parent; anchors.topMargin: 8; spacing: 2
                Repeater {
                    model: [
                        { k: "timeline", t: Lang.t("Photos"), g: "photos", c: "timeline" },
                        { k: "albums", t: Lang.t("Albums"), g: "albums", c: "" },
                        { k: "persons", t: Lang.t("People"), g: "persons", c: "", ai: true },
                        { k: "fav", t: Lang.t("Favorites"), g: "fav", c: "fav" },
                        { k: "video", t: Lang.t("Videos"), g: "video", c: "video" },
                        { k: "recent", t: Lang.t("Recently added"), g: "recent", c: "" },
                        { k: "archive", t: Lang.t("Archive"), g: "archive", c: "archive" },
                        { k: "nodate", t: Lang.t("No date"), g: "nodate", c: "nodate" },
                        { k: "trash", t: Lang.t("Trash"), g: "trash", c: "trash" }
                    ]
                    delegate: Rectangle {
                        id: si
                        required property var modelData
                        property bool cur: (modelData.k === win.page) || (win.page === "grid" && modelData.k === win.viewKind) || (win.page === "grid" && win.viewKind === "album" && modelData.k === "albums") || (win.page === "grid" && win.viewKind === "person" && modelData.k === "persons")
                        visible: !modelData.ai || win.hasPersons
                        height: visible ? 40 : 0
                        anchors.left: parent.left; anchors.right: parent.right; anchors.leftMargin: 8; anchors.rightMargin: 8
                        radius: 20
                        color: cur ? Theme.navSelected : (sm2.containsMouse ? Theme.surface : "transparent")
                        Image { x: win.sideCollapsed ? (parent.width - 20) / 2 : 16; anchors.verticalCenter: parent.verticalCenter; width: 20; height: 20; sourceSize: Qt.size(40, 40); source: Theme.icon_(si.modelData.g, si.cur ? "g" : "d") }
                        Text { x: 46; anchors.verticalCenter: parent.verticalCenter; text: si.modelData.t; visible: !win.sideCollapsed; color: si.cur ? Theme.accent : Theme.text; font.pixelSize: 14; font.bold: si.cur }
                        Text {
                            anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter
                            visible: !win.sideCollapsed && si.modelData.c !== "" && (win.counts[si.modelData.c] || 0) > 0
                            text: Theme.fmt(win.counts[si.modelData.c] || 0)
                            color: Theme.textMuted; font.pixelSize: 11
                        }
                        MouseArea { id: sm2; anchors.fill: parent; hoverEnabled: true; onClicked: { win.goSide(si.modelData.k); root.forceActiveFocus() } }
                        ToolTip.visible: win.sideCollapsed && sm2.containsMouse; ToolTip.delay: 300; ToolTip.text: si.modelData.t
                    }
                }
            }
        }

        Rectangle { anchors.left: side.right; anchors.top: titleBar.bottom; anchors.bottom: parent.bottom; width: 1; color: Theme.border; z: 5 }
        Rectangle {
            id: setItem
            anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.leftMargin: 8; anchors.bottomMargin: 10
            width: side.width - 16; height: 40; radius: 20
            property bool cur: win.page === "settings"
            color: cur ? Theme.navSelected : (sm3.containsMouse ? Theme.surface : "transparent")
            Image { x: win.sideCollapsed ? (parent.width - 20) / 2 : 16; anchors.verticalCenter: parent.verticalCenter; width: 20; height: 20; sourceSize: Qt.size(40, 40); source: Theme.icon_("settings", setItem.cur ? "g" : "d") }
            Text { x: 46; anchors.verticalCenter: parent.verticalCenter; text: Lang.t("Settings"); visible: !win.sideCollapsed; color: setItem.cur ? Theme.accent : Theme.text; font.pixelSize: 14; font.bold: setItem.cur }
            MouseArea { id: sm3; anchors.fill: parent; hoverEnabled: true; onClicked: { win.goSide("settings"); root.forceActiveFocus() } }
            ToolTip.visible: win.sideCollapsed && sm3.containsMouse; ToolTip.delay: 300; ToolTip.text: Lang.t("Settings")
        }

        // ---------- scan / thumbnail progress bar ----------
        Rectangle {
            id: progBar
            property bool scanningNow: (win.status.phase || 0) > 0
            property bool thumbsNow: !scanningNow && (win.status.th_total || 0) > (win.status.th_done || 0) + (win.status.th_failed || 0)
            visible: scanningNow || thumbsNow
            z: 30
            anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.rightMargin: 84; anchors.bottomMargin: 14
            height: 30; radius: 15; width: pbRow.width + 28
            color: Theme.barBg; border.width: 1; border.color: Theme.border
            Row {
                id: pbRow; anchors.centerIn: parent; spacing: 10
                Rectangle {
                    id: pbSpin; width: 12; height: 12; radius: 6; anchors.verticalCenter: parent.verticalCenter
                    color: "transparent"; border.width: 2; border.color: Theme.accent
                    SequentialAnimation on opacity { loops: Animation.Infinite; running: progBar.visible; NumberAnimation { to: 0.25; duration: 700 } NumberAnimation { to: 1; duration: 700 } }
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter; color: Theme.text; font.pixelSize: 12
                    text: progBar.scanningNow ? (Lang.t("Scanning: ") + Theme.fmt(win.status.done || 0) + " / " + Theme.fmt(win.status.todo || 0))
                                              : (Lang.t("Thumbnails: ") + Theme.fmt(win.status.th_done || 0) + " / " + Theme.fmt(win.status.th_total || 0))
                }
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter; width: 90; height: 4; radius: 2; color: Theme.border
                    Rectangle {
                        height: parent.height; radius: 2; color: Theme.accent
                        width: parent.width * Math.min(1, progBar.scanningNow ? (win.status.done || 0) / Math.max(1, win.status.todo || 1) : (win.status.th_done || 0) / Math.max(1, win.status.th_total || 1))
                    }
                }
            }
        }

        // ---------- content ----------
        Item {
            id: content
            anchors.left: side.right; anchors.right: parent.right; anchors.top: titleBar.bottom; anchors.bottom: parent.bottom

            Item {
                anchors.fill: parent
                visible: win.page === "grid"
                Item {
                    id: gridHead
                    anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
                    height: 46
                    Text { x: 20; anchors.verticalCenter: parent.verticalCenter; visible: grid.selCount === 0; text: win.viewTitle; color: Theme.text; font.pixelSize: 20; font.bold: true }
                    Text {
                        anchors.left: parent.left; anchors.leftMargin: 20 + 0; anchors.verticalCenter: parent.verticalCenter
                        visible: false; text: ""
                    }
                    Text {
                        anchors.right: parent.right; anchors.rightMargin: 64; anchors.verticalCenter: parent.verticalCenter
                        visible: grid.selCount === 0
                        color: Theme.textMuted; font.pixelSize: 12
                        text: Theme.fmt(grid.itemCount) + " " + Lang.t("items")
                    }
                    Row {
                        visible: grid.selCount > 0
                        anchors.left: parent.left; anchors.leftMargin: 14; anchors.verticalCenter: parent.verticalCenter; spacing: 8
                        Btn { text: "✕"; flat: true; pad: 10; onClicked: { be.clear_sel(); grid.bump() } }
                        Text { text: Lang.t("Selected: ") + Theme.fmt(grid.selCount); color: Theme.accent; font.pixelSize: 16; font.bold: true; anchors.verticalCenter: parent.verticalCenter; rightPadding: 14 }
                        Btn { text: Lang.t("＋ Album"); onClicked: win.askAlbum() }
                        Btn { text: Lang.t("★ Favorite"); onClicked: win.doMark("fav", -1) }
                        Btn { text: win.viewKind === "archive" ? Lang.t("Restore from archive") : Lang.t("Archive|action"); onClicked: win.doMark("archived", -1) }
                        Btn { visible: win.viewKind !== "trash"; text: Lang.t("Delete…"); onClicked: win.askDelete() }
                        Btn { visible: win.viewKind === "trash"; text: Lang.t("Restore"); onClicked: win.doRestore() }
                        Btn { visible: win.viewKind === "trash"; text: Lang.t("Delete permanently…"); onClicked: win.askDelete() }
                        Btn { visible: win.viewKind !== "trash" && grid.selCount === 1; text: Lang.t("Show in folder"); onClicked: win.revealCurrent() }
                        Btn { visible: win.viewKind === "album" && win.viewArg !== ""; text: Lang.t("Remove from album"); onClicked: { be.album_remove(parseInt(win.viewArg), -1); win.refreshAll() } }
                    }
                    Btn {
                        visible: win.viewKind === "album" && grid.selCount === 0
                        anchors.right: parent.right; anchors.rightMargin: 200; anchors.verticalCenter: parent.verticalCenter
                        text: Lang.t("Rename"); flat: true
                        onClicked: { if (be.album_kind(parseInt(win.viewArg)) === "local") { renameField.text = win.viewTitle; renamePop.open() } else win.say(Lang.t("Takeout album — name is read-only")) }
                    }
                }
                Timeline {
                    id: grid
                    be: be
                    anchors.left: parent.left; anchors.right: parent.right; anchors.top: gridHead.bottom; anchors.bottom: parent.bottom
                    scanning: win.status.phase > 0
                    emptyText: win.viewKind === "search" ? Lang.t("No results for this query.") : (win.viewKind === "ai" ? (win.status.ai_busy ? Lang.t("Searching by content…") : Lang.t("No results.")) : Lang.t("No photos in this view."))
                    onOpenItem: (id, idx) => win.openViewer(id)
                    onContextRequested: (id) => { if (!be.is_selected(id)) { be.clear_sel(); be.toggle_sel(id); grid.bump() } ctxMenu.pid = id; ctxMenu.popup() }
                    onFocusRequested: root.forceActiveFocus()
                }
            }
            Albums {
                id: albumsPage
                anchors.fill: parent; visible: win.page === "albums"; be: be
                onOpenAlbum: (id, title) => win.openView("album", String(id), title)
            }
            Persons {
                id: personsPage
                anchors.fill: parent; visible: win.page === "persons"; be: be
                onToast: (t) => win.say(t)
                onOpenPerson: (id, name) => win.openView("person", String(id), name !== "" ? name : Lang.t("Person"))
            }
            Settings {
                id: settings
                anchors.fill: parent; visible: win.page === "settings"; be: be; status: win.status
                onRescan: { be.scan_now(); win.say(Lang.t("Scan started")) }
            }
        }

        // ---------- viewer ----------
        Viewer {
            id: viewer
            visible: false
            anchors.fill: parent
            be: be
            z: 50
            onCloseRequested: win.closeViewer()
            onStep: (d) => win.stepViewer(d)
            onToggleFav: win.doMark("fav", -1)
            onAlbumRequested: win.askAlbum()
            onDeleteRequested: win.askDelete()
            onFocusOut: root.forceActiveFocus()
        }

        // ---------- toast ----------
        Rectangle {
            visible: win.toast !== ""
            z: 100
            anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; anchors.bottomMargin: 28
            height: 38; radius: 19; width: tt.implicitWidth + 36
            color: Theme.toast
            Text { id: tt; anchors.centerIn: parent; text: win.toast; color: Theme.toastText; font.pixelSize: 14 }
        }
        Timer { id: toastTimer; interval: 2600; onTriggered: win.toast = "" }

        // ---------- resize handles ----------
        MouseArea { z: 200; anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 4; cursorShape: Qt.SizeHorCursor; onPressed: win.startSystemResize(Qt.LeftEdge) }
        MouseArea { z: 200; anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 4; cursorShape: Qt.SizeHorCursor; onPressed: win.startSystemResize(Qt.RightEdge) }
        MouseArea { z: 200; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: 4; cursorShape: Qt.SizeVerCursor; onPressed: win.startSystemResize(Qt.BottomEdge) }
        MouseArea { z: 200; anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; height: 3; cursorShape: Qt.SizeVerCursor; onPressed: win.startSystemResize(Qt.TopEdge) }
        MouseArea { z: 201; anchors.right: parent.right; anchors.bottom: parent.bottom; width: 12; height: 12; cursorShape: Qt.SizeFDiagCursor; onPressed: win.startSystemResize(Qt.RightEdge | Qt.BottomEdge) }

        // ---------- popups ----------
        Popup {
            id: albumPicker
            parent: Overlay.overlay
            anchors.centerIn: Overlay.overlay
            modal: true; width: 380; height: 440; padding: 16
            property var list: []
            function refresh() { var a = JSON.parse(be.albums_json()); list = a.filter(function(x) { return x.kind === "local" }) }
            background: Rectangle { color: Theme.surface; radius: 10; border.color: Theme.accent }
            onClosed: root.forceActiveFocus()
            Column {
                spacing: 10; anchors.fill: parent
                Text { text: Lang.t("Add to album"); color: Theme.text; font.pixelSize: 18; font.bold: true }
                Row {
                    spacing: 6
                    TextField {
                        id: apNew; width: 250; placeholderText: Lang.t("New album…"); color: Theme.text; placeholderTextColor: Theme.textMuted
                        background: Rectangle { color: Theme.window; radius: 6; border.color: apNew.activeFocus ? Theme.accent : Theme.border }
                        onAccepted: apCreate.clicked()
                    }
                    Btn { id: apCreate; text: Lang.t("Create"); accent: true; onClicked: {
                        if (apNew.text.trim() === "") return
                        var id = be.album_create(apNew.text); apNew.text = ""; win.addToAlbum(id) } }
                }
                ListView {
                    width: parent.width; height: parent.height - 100; clip: true
                    model: albumPicker.list
                    delegate: Rectangle {
                        required property var modelData
                        width: ListView.view.width; height: 38; radius: 6
                        color: am.containsMouse ? Theme.navSelected : "transparent"
                        Text { x: 10; anchors.verticalCenter: parent.verticalCenter; text: modelData.title + "  (" + modelData.count + ")"; color: Theme.text; font.pixelSize: 14 }
                        MouseArea { id: am; anchors.fill: parent; hoverEnabled: true; onClicked: win.addToAlbum(modelData.id) }
                    }
                }
            }
        }
        Popup {
            id: renamePop
            parent: Overlay.overlay
            anchors.centerIn: Overlay.overlay
            modal: true; width: 360; padding: 16
            background: Rectangle { color: Theme.surface; radius: 10; border.color: Theme.accent }
            onOpened: renameField.forceActiveFocus()
            onClosed: root.forceActiveFocus()
            Column {
                spacing: 10; width: parent.width
                TextField {
                    id: renameField; width: parent.width; color: Theme.text
                    background: Rectangle { color: Theme.window; radius: 6; border.color: Theme.accent }
                    onAccepted: { be.album_rename(parseInt(win.viewArg), text); win.viewTitle = be.album_title(parseInt(win.viewArg)); renamePop.close() }
                }
                Text { text: Lang.t("Enter — save"); color: Theme.textMuted; font.pixelSize: 12 }
            }
        }
        Popup {
            id: confirmPop
            parent: Overlay.overlay
            anchors.centerIn: Overlay.overlay
            modal: true; width: 440; padding: 20
            background: Rectangle { color: Theme.surface; radius: 10; border.color: win.confirmMode === "purge" ? Theme.danger : Theme.accent }
            onClosed: root.forceActiveFocus()
            Column {
                spacing: 14; width: parent.width
                Text {
                    width: parent.width; wrapMode: Text.WordWrap; color: Theme.text; font.pixelSize: 17; font.bold: true
                    text: win.confirmMode === "purge"
                          ? Lang.f("Permanently delete %1?", Lang.cnt(win.confirmN, "file"))
                          : Lang.f("Move %1 to trash?", Lang.cnt(win.confirmN, "file"))
                }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: 13
                    text: win.confirmMode === "purge" ? Lang.t("This cannot be undone. The files will disappear from the system trash and from disk.")
                                                         : Lang.t("The files will go to the system trash (you can restore them from the “Trash” view).")
                }
                Row {
                    spacing: 10
                    Btn { text: win.confirmMode === "purge" ? Lang.t("Delete permanently") : Lang.t("Move to trash"); accent: true; onClicked: win.runConfirmed() }
                    Btn { text: Lang.t("Cancel"); onClicked: confirmPop.close() }
                }
            }
        }
        Popup {
            id: errPop
            parent: Overlay.overlay
            anchors.centerIn: Overlay.overlay
            modal: true; width: 520; padding: 20
            background: Rectangle { color: Theme.surface; radius: 10; border.color: Theme.danger }
            onClosed: root.forceActiveFocus()
            Column {
                spacing: 12; width: parent.width
                Text { text: Lang.t("The operation failed"); color: Theme.danger; font.pixelSize: 18; font.bold: true }
                Text { width: parent.width; wrapMode: Text.WrapAnywhere; color: Theme.text; font.pixelSize: 12; text: win.errText + Lang.t("\n\nFiles affected by the error were left untouched (nothing was deleted permanently).") }
                Btn { text: "OK"; accent: true; onClicked: errPop.close() }
            }
        }
        Menu {
            id: ctxMenu
            property int pid: -1
            MenuItem { text: Lang.t("Show in folder"); onTriggered: { be.show_in_folder(ctxMenu.pid); win.say(Lang.t("Opening folder…")) } }
            MenuItem { text: Lang.t("Copy path"); onTriggered: { be.copy_text(be.abs_path(ctxMenu.pid)); win.say(Lang.t("Path copied")) } }
            MenuSeparator {}
            MenuItem { text: win.viewKind === "trash" ? Lang.t("Delete permanently…") : Lang.t("Delete…"); onTriggered: win.askDelete() }
            MenuItem { visible: win.viewKind === "trash"; height: visible ? implicitHeight : 0; text: Lang.t("Restore"); onTriggered: win.doRestore() }
        }
        Popup {
            id: helpPop
            parent: Overlay.overlay
            anchors.centerIn: Overlay.overlay
            modal: true; width: 560; padding: 20
            background: Rectangle { color: Theme.surface; radius: 10; border.color: Theme.accent }
            onClosed: root.forceActiveFocus()
            Column {
                spacing: 8; width: parent.width
                Text { text: Lang.t("Keyboard shortcuts"); color: Theme.accent; font.pixelSize: 20; font.bold: true }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap; color: Theme.text; font.pixelSize: 14; lineHeight: 1.5
                    text: Lang.t("/   search\n←  →   previous / next (viewer)\nz   + −   0   zoom\ni   Info panel\nSpace   video: play / pause\nx   select under cursor     Shift+click   range     Ctrl+A   all\nf   favorite     Shift+A   archive     #  or  Delete   to trash\nCtrl+wheel   grid size     F11   full screen     Esc   back")
                }
            }
        }
    }

    function addToAlbum(aid) {
        var inV = viewerOpen()
        var n = be.album_add(aid, inV ? viewer.itemId : -1)
        albumPicker.close()
        say(Lang.f("Added to album “%1”: %2", be.album_title(aid), n))
        if (inV) viewer.reloadInfo()
        if (page === "albums") albumsPage.reload()
    }

    // ---------- state polling ----------
    Timer {
        interval: 500; repeat: true; running: true; triggeredOnStart: true
        onTriggered: {
            win.status = JSON.parse(be.poll())
            if (win.status.ver !== win.lastVer) {
                win.lastVer = win.status.ver
                be.set_view(win.viewKind, win.viewArg)
                win.refreshAll()
            }
        }
    }
    onWidthChanged: grid.relayout()

    // ---------- automated test (--autotest=DIRECTORY) ----------
    property string atDir: be.autotest_dir()
    property int atStep: 0
    function shot(name, next) {
        win.contentItem.grabToImage(function(r) {
            var ok = r.saveToFile(win.atDir + "/" + name + ".png")
            console.log("AUTOTEST screenshot " + name + " " + (ok ? "ok" : "ERROR"))
            next()
        })
    }
    Timer {
        id: atTimer
        interval: 600; repeat: true; running: win.atDir !== "" && !win.selfScroll
        property int waited: 0
        onTriggered: {
            waited++
            var ready = win.status.phase === 0 && win.lastVer > 1 && grid.itemCount > 0
            if (win.atStep === 0 && (ready || waited > 120)) {
                win.atStep = 1
                console.log("AUTOTEST timeline: items=" + grid.itemCount + " rows=" + grid.rows.length + " height=" + Math.round(grid.total))
                // give thumbnails a moment
                atWait.start()
            }
        }
    }
    // steps: list of functions run in sequence; each step waits for a condition (max 15 s)
    property var atSteps: []
    property int atIdx: 0
    property int atTicks: 0
    function atRun() {
        atSteps = [
            { n: "01-timeline", pre: function() { grid.zoomLevel = 3 }, cond: function() { return (win.status.th_done || 0) > 200 || win.status.th_total === win.status.th_done }, wait: 8000 },
            { n: "02-timeline-larger-zoom", pre: function() { grid.zoomLevel = 4 } },
            { n: "03-viewer", pre: function() { grid.zoomLevel = 3; win.openViewer(be.id_at(0)) }, cond: function() { return viewer.ready }, extra: 400 },
            { n: "04-viewer-info", pre: function() { viewer.showInfo = true }, cond: function() { return viewer.ready }, extra: 700 },
            { n: "05-albums", pre: function() { viewer.showInfo = false; win.closeViewer(); win.goSide("albums") }, extra: 1500 },
            { n: "06-people", pre: function() { win.goSide("persons") }, extra: 3500 },
            { n: "07-settings", pre: function() { win.goSide("settings") }, extra: 500 },
            { n: "08-search", pre: function() { win.goSide("timeline"); searchField.text = "2019"; win.runSearch() }, extra: 1500 },
            { n: "09-search-content", pre: function() { win.goSide("timeline"); win.semMode = true; searchField.text = "wedding"; win.runSearch() }, cond: function() { return !win.status.ai_busy && win.atTicks > 8 }, extra: 4000 }
        ]
        atIdx = 0
        atNext()
    }
    function atNext() {
        if (atIdx >= atSteps.length) { console.log("AUTOTEST DONE ok"); Qt.quit(); return }
        var st = atSteps[atIdx]
        if (st.pre) st.pre()
        atTicks = 0
        atPoll.st = st
        atPoll.restart()
    }
    Timer {
        id: atPoll
        interval: 250; repeat: true
        property var st
        onTriggered: {
            win.atTicks++
            var ok = !st.cond || st.cond()
            if (!ok && win.atTicks < 80) return
            if (!ok) console.log("AUTOTEST warning: step condition " + st.n + " not met")
            stop()
            atExtra.interval = st.extra || (st.wait || 300)
            atExtra.start()
        }
    }
    Timer {
        id: atExtra
        onTriggered: {
            var st = win.atSteps[win.atIdx]
            win.shot(st.n, function() { win.atIdx++; win.atNext() })
        }
    }
    Timer {
        id: atWait
        interval: 5000
        onTriggered: win.atRun()
    }

    // ---------- scroll and deletion test (--selftest-scroll, only KULA_SELFTEST=1 + copy of the database in /tmp) ----------
    property bool selfScroll: be.selftest_scroll()
    property int stFail: 0
    property var stQueue: []
    property int stI: 0
    function stLog(m) { be.selftest_log("SELFTEST " + m) }
    function stCheck(ok, m) { if (!ok) stFail++; stLog((ok ? "OK    " : "ERROR  ") + m) }
    function stNext() {
        if (stI >= stQueue.length) { stLog("DONE: failures=" + stFail); Qt.exit(stFail > 0 ? 1 : 0); return }
        var st = stQueue[stI++]
        st.f()
        stStep.interval = st.w || 300
        stStep.restart()
    }
    Timer { id: stStep; onTriggered: win.stNext() }
    function stShot(name) { shot(name, function() { }) }
    function stRun() {
        var H0 = 0, fv0 = null, y0 = 0, cnt0 = 0, hs = []
        stQueue = [
          { f: function() {
                grid.zoomLevel = 3
                stLog("items=" + grid.itemCount + " rows=" + grid.rows.length + " total=" + Math.round(grid.total) + " window=" + Math.round(grid.view.height) + " maxY=" + Math.round(grid.maxY))
            }, w: 700 },
          { f: function() {   // 1. continuity of contentHeight from top to bottom and slider monotonicity
                var prev = -1, mono = true, hmin = 1e18, hmax = -1
                for (var i = 0; i <= 400; i++) {
                    grid.scrubTo(i / 400)
                    var cy = grid.view.contentY, ch = grid.view.contentHeight
                    hmin = Math.min(hmin, ch); hmax = Math.max(hmax, ch)
                    if (cy < prev - 0.001) mono = false
                    prev = cy
                }
                stCheck(hmax - hmin < 0.5, "contentHeight constant while scrolling 0→100%: min=" + hmin + " max=" + hmax + " (total=" + grid.total + ")")
                stCheck(mono, "scrubber → contentY monotonic (401 steps)")
                var pts = [0, 0.5, 1.0]
                for (var k = 0; k < pts.length; k++) {
                    grid.scrubTo(pts[k])
                    var fvv = grid.firstVisible()
                    stLog("scrubber " + Math.round(pts[k] * 100) + "%: contentY=" + Math.round(grid.view.contentY) + " of maxY=" + Math.round(grid.maxY) + ", contentHeight=" + Math.round(grid.view.contentHeight) + ", first visible idx=" + (fvv ? fvv.idx : -1) + ", etykieta=" + grid.labelAt(grid.view.contentY))
                }
                stCheck(Math.abs(grid.view.contentY - grid.maxY) < 0.5, "100%: contentY == contentHeight - height")
                var lr = -1
                for (var r = grid.rows.length - 1; r >= 0; r--) if (grid.rows[r].t === "r") { lr = r; break }
                var lastIdx = grid.rows[lr].it[grid.rows[lr].it.length - 1][2]
                stCheck(lastIdx === be.count() - 1, "last row holds the oldest item (idx " + lastIdx + " z " + (be.count() - 1) + ")")
                var bottom = grid.rowY[lr] + grid.rows[lr].h
                stCheck(grid.rowY[lr] >= grid.view.contentY && bottom <= grid.view.contentY + grid.view.height + 0.5, "100%: last row fully inside the window (row bottom " + Math.round(bottom) + " ≤ " + Math.round(grid.view.contentY + grid.view.height) + ")")
                stCheck(grid.vLast >= lr, "100%: last row delegate created (vLast=" + grid.vLast + " ≥ " + lr + ")")
                grid.scrubTo(0)
                stCheck(grid.view.contentY === 0 && grid.firstVisible().idx === 0, "0%: list start")
            }, w: 300 },
          { f: function() {   // keyboard / API: End, Home, PgDn until the end
                grid.toBottom(); stCheck(Math.abs(grid.view.contentY - grid.maxY) < 0.5, "End → list end")
                grid.toTop(); stCheck(grid.view.contentY === 0, "Home → start")
                var n = 0
                while (grid.view.contentY < grid.maxY - 0.5 && n < 5000) { grid.pageBy(1); n++ }
                stCheck(Math.abs(grid.view.contentY - grid.maxY) < 0.5, "PgDn × " + n + " reaches the end (contentY=" + Math.round(grid.view.contentY) + ")")
                var m = 0
                while (grid.view.contentY > 0.5 && m < 5000) { grid.pageBy(-1); m++ }
                stCheck(grid.view.contentY === 0, "PgUp × " + m + " returns to start")
            }, w: 200 },
          { f: function() { grid.scrubTo(0.4) }, w: 400 },
          { f: function() { stShot("s-01-panel-expanded") }, w: 800 },
          { f: function() { grid.scrubTo(1.0) }, w: 500 },
          { f: function() { stShot("s-02-scrubber-at-bottom") }, w: 800 },
          { f: function() { grid.scrubTo(0.4); win.toggleSide() }, w: 700 },
          { f: function() { stCheck(Math.abs(side.width - 56) < 1, "panel collapsed: width " + side.width + " px, grid " + Math.round(grid.width) + " px, rows " + grid.rows.length); stShot("s-03-panel-collapsed") }, w: 800 },
          { f: function() { grid.scrubTo(1.0) }, w: 500 },
          { f: function() { stShot("s-04-schowany-scrubber-at-bottom"); win.toggleSide() }, w: 700 },
          { f: function() { stCheck(Math.abs(side.width - 210) < 1, "panel expanded: width " + side.width + " px"); grid.scrubTo(0.5) }, w: 500 },
          // 2. deletion: element below the first visible one
          { f: function() {
                y0 = grid.view.contentY; fv0 = grid.firstVisible(); cnt0 = be.count()
                var vr = -1, c = 0
                for (var r = fv0.row + 1; r < grid.rows.length; r++) if (grid.rows[r].t === "r") { c++; if (c === 3) { vr = r; break } }
                var ids = grid.rows[vr].it.map(function(x) { return x[0] })
                be.clear_sel(); for (var i = 0; i < ids.length; i++) be.toggle_sel(ids[i]); grid.bump()
                stLog("deleting " + ids.length + " photos from row " + vr + " (below first visible id=" + fv0.id + "), contentY before=" + Math.round(y0) + ", items=" + cnt0)
                win.confirmMode = "trash"; win.confirmId = -1; win.runConfirmed()
            }, w: 700 },
          { f: function() {
                var fv1 = grid.firstVisible(), y1 = grid.view.contentY
                stCheck(fv1 && fv1.id === fv0.id, "after deletion first visible id unchanged (" + fv0.id + " → " + (fv1 ? fv1.id : -1) + ")")
                stCheck(Math.abs(y1 - y0) <= 300, "scroll position before/after: " + Math.round(y0) + " → " + Math.round(y1) + " (difference " + Math.round(Math.abs(y1 - y0)) + " ≤ row height)")
                stCheck(be.count() < cnt0 && grid.selCount === 0, "items " + cnt0 + " → " + be.count() + ", selection cleared (" + grid.selCount + ")")
                stCheck(grid.itemCount === be.count(), "grid model matches backend (" + grid.itemCount + ")")
                var tl = grid.visibleTiles(), bad = 0, chk = 0
                for (var i = 0; i < tl.length; i++) {
                    var u = be.thumb_url(tl[i].pid)
                    if (tl[i].url !== "" && u !== "") { chk++; if (tl[i].url !== u) bad++ }
                    if (tl[i].pid !== tl[i].d[0]) bad++
                }
                stCheck(bad === 0, "thumbnails after deletion: checked " + chk + " z " + tl.length + " tiles, mismatched " + bad)
            }, w: 300 },
          // 3. deleting only the first visible element (the anchor disappears)
          { f: function() {
                y0 = grid.view.contentY; fv0 = grid.firstVisible(); cnt0 = be.count()
                be.clear_sel(); be.toggle_sel(fv0.id); grid.bump()
                var nextId = be.id_at(fv0.idx + 1)
                stLog("deleting pierwszy widoczny id=" + fv0.id + " (next id=" + nextId + "), contentY before=" + Math.round(y0))
                win.confirmMode = "trash"; win.confirmId = -1; win.runConfirmed()
                win.stNextId = nextId
            }, w: 700 },
          { f: function() {
                var fv1 = grid.firstVisible(), y1 = grid.view.contentY
                stCheck(fv1 && fv1.id !== fv0.id, "deleted first visible item gone from view (id " + (fv1 ? fv1.id : -1) + ")")
                stCheck(Math.abs(y1 - y0) <= 300, "position after deleting anchor: " + Math.round(y0) + " → " + Math.round(y1) + " (difference " + Math.round(Math.abs(y1 - y0)) + ")")
                stCheck(be.count() === cnt0 - 1, "items " + cnt0 + " → " + be.count())
            }, w: 300 },
          // 4. favorites and archiving do not change position
          { f: function() {
                y0 = grid.view.contentY; fv0 = grid.firstVisible()
                be.clear_sel(); be.toggle_sel(grid.rows[grid.findRow(y0 + 300)].t === "r" ? grid.rows[grid.findRow(y0 + 300)].it[0][0] : fv0.id); grid.bump()
                win.doMark("fav", 1)
            }, w: 600 },
          { f: function() {
                stCheck(Math.abs(grid.view.contentY - y0) < 0.5 && grid.firstVisible().id === fv0.id, "favorite: position unchanged (" + Math.round(y0) + " → " + Math.round(grid.view.contentY) + ")")
                cnt0 = be.count()
                be.clear_sel(); be.toggle_sel(grid.rows[grid.findRow(y0 + 300)].t === "r" ? grid.rows[grid.findRow(y0 + 300)].it[0][0] : fv0.id); grid.bump()
                win.doMark("archived", 1)
            }, w: 600 },
          { f: function() {
                stCheck(be.count() === cnt0 - 1, "archiving: item gone from timeline (" + cnt0 + " → " + be.count() + ")")
                stCheck(Math.abs(grid.view.contentY - y0) <= 300 && grid.firstVisible().id === fv0.id, "archiving: position (" + Math.round(y0) + " → " + Math.round(grid.view.contentY) + "), first visible id unchanged")
            }, w: 300 },
          { f: function() { win.openViewer(be.id_at(0)) }, w: 1200 },
          { f: function() { stShot("s-05-viewer-icons") }, w: 800 },
          { f: function() { win.closeViewer() }, w: 200 }
        ]
        stI = 0
        stNext()
    }
    property int stNextId: -1
    Timer {
        id: stStart
        interval: 500; repeat: true; running: win.selfScroll
        property int n: 0
        onTriggered: {
            n++
            if (grid.itemCount > 0 && grid.total > 0 && grid.view.height > 0) { stop(); win.stRun() }
            else if (n > 60) { stop(); stLog("ERROR no data in the database copy"); Qt.exit(1) }
        }
    }
}
