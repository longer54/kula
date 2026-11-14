import QtQuick
import org.kula

Rectangle {
    id: b
    property string text: ""
    property bool accent: false
    property bool flat: false
    property int pad: 14
    signal clicked()
    implicitWidth: lbl.implicitWidth + pad * 2
    implicitHeight: 34
    radius: Theme.radius
    color: accent ? (ma.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (ma.containsMouse ? Theme.navSelected : (flat ? "transparent" : Theme.surface))
    border.width: accent || flat ? 0 : 1
    border.color: Theme.border
    Text {
        id: lbl
        anchors.centerIn: parent
        text: b.text
        color: b.accent ? Theme.accentText : Theme.text
        font.pixelSize: 14
        font.bold: b.accent
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
}
