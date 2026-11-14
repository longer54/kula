import QtQuick
import QtQuick.Controls
import org.kula

// Icon button (24 px) with a tooltip.
Rectangle {
    id: b
    property string icon: ""
    property string role: "d"
    property string tip: ""
    property int iconSize: 24
    signal clicked()
    implicitWidth: 40; implicitHeight: 40
    radius: 20
    color: ma.containsMouse ? Theme.navSelected : "transparent"
    Image {
        anchors.centerIn: parent
        width: b.iconSize; height: b.iconSize
        sourceSize: Qt.size(b.iconSize * 2, b.iconSize * 2)
        source: Theme.icon_(b.icon, b.role)
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
    ToolTip.visible: ma.containsMouse && b.tip !== ""; ToolTip.delay: 400; ToolTip.text: b.tip
}
