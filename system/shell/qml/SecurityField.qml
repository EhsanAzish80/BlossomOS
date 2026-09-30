import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Label {
    required property string label
    required property string value

    Layout.fillWidth: true
    color: "#dbe5f5"
    wrapMode: Text.WrapAnywhere
    textFormat: Text.PlainText
    text: label + ":  " + value
    Accessible.role: Accessible.StaticText
    Accessible.name: label
    Accessible.description: value
    Accessible.ignored: false
}
