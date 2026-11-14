pragma Singleton
import QtQuick

// All interface colors. Roles as in Kinto. Mode: "dark" | "light" | "system".
QtObject {
    id: t
    property string mode: "dark"
    readonly property bool dark: mode === "system" ? Qt.styleHints.colorScheme === Qt.ColorScheme.Dark : mode === "dark"

    readonly property color window: dark ? "#1d1d22" : "#ffffff"
    readonly property color list: window
    readonly property color sidebar: dark ? "#232429" : "#eceef2"
    readonly property color surface: dark ? "#2b2d33" : "#f4f5f8"
    readonly property color hover: dark ? "#2f3138" : "#f2f3f6"
    readonly property color selected: dark ? "#2c3a5c" : "#e3ebfd"
    readonly property color navSelected: dark ? "#34363d" : "#d9dde6"
    readonly property color border: dark ? "#34363d" : "#ebecef"
    readonly property color section: dark ? "#202126" : "#fafbfc"
    readonly property color text: dark ? "#eceef2" : "#1d1d22"
    readonly property color textStrong: text
    readonly property color textMuted: dark ? "#9a9da6" : "#6b6f7a"
    readonly property color textSubtle: dark ? "#a3a6b0" : "#5b5f6b"
    readonly property color icon: dark ? "#a3a6b0" : "#5b5f6b"
    readonly property color accent: dark ? "#8fb4ff" : "#2f6bed"
    readonly property color accentText: dark ? "#1d1d22" : "#ffffff"
    readonly property color danger: dark ? "#e86a6a" : "#d64545"
    readonly property color toast: dark ? "#3a3c44" : "#2b2d33"
    readonly property color toastText: "#ffffff"
    readonly property color toastAction: "#8fb4ff"
    readonly property color viewerBg: dark ? "#141418" : "#f4f5f8"
    readonly property color scrim: "#aa000000"
    readonly property color scrimSoft: "#55000000"
    readonly property color scrimStrong: "#cc000000"
    readonly property color onMedia: "#ffffff"
    readonly property color mediaShade: "#99000000"
    readonly property color barBg: dark ? "#ee2b2d33" : "#eef4f5f8"

    readonly property int fast: 110
    readonly property int normal: 170
    readonly property int easing: Easing.OutCubic
    readonly property int radius: 8

    // numbers with thousands separator (non-breaking space): 8935 -> "8 935"
    function fmt(n) {
        var v = Math.round(Number(n) || 0), neg = v < 0
        var t = String(Math.abs(v)).replace(/\B(?=(\d{3})+(?!\d))/g, "\u00a0")
        return (neg ? "-" : "") + t
    }

    // line icon in the role color: d = default, g = accent, t = text, k = text on accent
    function icon_(name, role) {
        return "qrc:/qt/qml/org/kula/icons/ic-" + name + "-" + role + "-" + (dark ? "dark" : "light") + ".svg"
    }
}
