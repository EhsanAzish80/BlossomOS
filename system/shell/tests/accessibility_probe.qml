import QtQuick
import QtQuick.Controls
import QtQuick.Window

ApplicationWindow {
    visible: true
    width: 360
    height: 120
    title: "Blossom accessibility probe"

    Button {
        anchors.centerIn: parent
        text: "Standard Qt accessibility probe"
        Accessible.name: text
        Accessible.description: "Confirm the standard Qt window is published to AT-SPI."
        Accessible.role: Accessible.Button
        Accessible.ignored: false
    }
}
