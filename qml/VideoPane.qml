import QtQuick
import QtMultimedia
import org.kula

Item {
    id: vp
    property url source
    property bool playing: player.playbackState === MediaPlayer.PlayingState
    function toggle() { if (vp.playing) player.pause(); else player.play() }
    function stop() { player.stop() }
    MediaPlayer {
        id: player
        source: vp.source
        videoOutput: vout
        audioOutput: AudioOutput {}
    }
    VideoOutput { id: vout; anchors.fill: parent }
    Rectangle {
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: 6; color: Theme.scrimSoft
        Rectangle { height: parent.height; color: Theme.accent; width: player.duration > 0 ? parent.width * player.position / player.duration : 0 }
        MouseArea { anchors.fill: parent; anchors.topMargin: -10; onClicked: (m) => player.position = player.duration * m.x / width }
    }
    Text {
        anchors.centerIn: parent
        visible: !vp.playing
        text: "▶"; color: Theme.onMedia; font.pixelSize: 64
    }
    MouseArea { anchors.fill: parent; anchors.bottomMargin: 14; onClicked: vp.toggle() }
}
