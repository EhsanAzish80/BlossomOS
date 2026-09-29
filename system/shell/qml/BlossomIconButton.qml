import QtQuick
import QtQuick.Controls

Button {
    id: control
    property string symbol: ""
    property string description: text
    property bool selected: false

    implicitWidth: 40
    implicitHeight: 40
    padding: 0
    Accessible.name: description
    Accessible.description: description

    ToolTip.visible: hovered && description.length > 0
    ToolTip.text: description
    ToolTip.delay: 450

    contentItem: Text {
        text: control.symbol
        color: control.enabled ? "#edf4fb" : "#6f7c90"
        font.pixelSize: 19
        font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }

    background: Rectangle {
        radius: 11
        color: control.down ? "#405069" : control.hovered || control.selected ? "#2b394b" : "transparent"
        border.color: control.activeFocus ? "#8dd7c7" : "transparent"
        border.width: control.activeFocus ? 2 : 0
        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
