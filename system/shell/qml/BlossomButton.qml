import QtQuick
import QtQuick.Controls

Button {
    id: control
    property bool primary: false
    property bool destructive: false

    implicitHeight: 42
    leftPadding: 18
    rightPadding: 18
    topPadding: 10
    bottomPadding: 10
    font.pixelSize: 14
    font.weight: Font.DemiBold

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? "#f4f7fb" : "#6f7c90"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: 12
        color: !control.enabled ? "#17202c"
             : control.down ? (control.destructive ? "#8f3434" : control.primary ? "#247c72" : "#344155")
             : control.hovered ? (control.destructive ? "#713030" : control.primary ? "#2f9488" : "#2b3748")
             : control.destructive ? "#59292d" : control.primary ? "#267f76" : "#202b3a"
        border.color: control.activeFocus ? "#8dd7c7" : control.primary ? "#3ba598" : "#3c4a5e"
        border.width: control.activeFocus ? 2 : 1

        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
