import QtQuick
import QtQuick.Controls
import "Petal"

Button {
    id: control
    property string symbol: ""
    property url iconSource: ""
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

    contentItem: Item {
        Image {
            anchors.centerIn: parent
            width: 20
            height: 20
            source: control.iconSource
            sourceSize.width: 40
            sourceSize.height: 40
            fillMode: Image.PreserveAspectFit
            opacity: control.enabled ? 1 : 0.4
            visible: control.iconSource.toString().length > 0
        }
        Text {
            anchors.centerIn: parent
            text: control.symbol
            color: control.enabled ? Theme.text : Theme.textDisabled
            font.family: Theme.sans
            font.pixelSize: 17
            font.weight: Font.Medium
            visible: control.iconSource.toString().length === 0
        }
    }

    background: Rectangle {
        radius: 8
        color: control.down ? Theme.pressed : control.selected ? Theme.blossomTint : control.hovered ? Theme.hover : "transparent"
        border.color: Theme.blossom
        border.width: control.activeFocus ? Theme.focusRing : 0
        Behavior on color { ColorAnimation { duration: Theme.stateMs } }
    }
}
