import QtQuick
import QtQuick.Controls

Button {
    id: control
    property string symbol: ""
    property url iconSource: ""
    property string description: text
    property bool running: false
    property bool selected: false

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
        Text { anchors.centerIn: parent; text: control.symbol; color: "#f4f7fb"; font.pixelSize: 24; font.weight: Font.DemiBold; visible: control.iconSource.toString().length === 0 }
        Rectangle {
            visible: control.running || control.selected
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            width: control.selected ? 18 : 6
            height: 3
            radius: 2
            color: "#79d4c5"
        }
    }

    background: Rectangle {
        radius: 14
        color: control.down ? "#3a4a61" : control.hovered || control.selected ? "#2a384b" : "transparent"
        border.color: control.activeFocus ? "#8dd7c7" : "transparent"
        border.width: control.activeFocus ? 2 : 0
        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
