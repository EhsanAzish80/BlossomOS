import QtQuick
import QtQuick.Controls
import "Petal"

Button {
    id: control
    property string symbol: ""
    property url iconSource: ""
    property string description: text
    property bool running: false
    property bool selected: false
    property bool accent: false

    implicitWidth: 52
    implicitHeight: 52
    padding: 0
    Accessible.name: description
    Accessible.description: description
    Accessible.ignored: false

    ToolTip.visible: hovered && description.length > 0
    ToolTip.text: description
    ToolTip.delay: 350

    contentItem: Item {
        Image {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -2
            width: 26
            height: 26
            source: control.iconSource
            sourceSize.width: 52
            sourceSize.height: 52
            fillMode: Image.PreserveAspectFit
            visible: control.iconSource.toString().length > 0
        }
        Text { anchors.centerIn: parent; text: control.symbol; color: Theme.text; font.family: Theme.sans; font.pixelSize: 22; font.weight: Font.Medium; visible: control.iconSource.toString().length === 0 }
        Rectangle {
            visible: control.running || control.selected
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            width: control.selected ? 16 : 4
            height: 4
            radius: 2
            color: control.selected || control.accent ? Theme.blossom : Theme.textSecondary
        }
    }

    background: Rectangle {
        radius: Theme.radiusTile
        color: control.down ? Theme.pressed : control.accent || control.selected ? Theme.blossomTint : control.hovered ? Theme.hover : "transparent"
        border.color: Theme.blossom
        border.width: control.activeFocus ? Theme.focusRing : 0
        Behavior on color { ColorAnimation { duration: Theme.stateMs } }
    }
}
