import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Label {
    required property string label
    required property string value

    Layout.fillWidth: true
    color: "#dbe5f5"
    elide: Text.ElideMiddle
    textFormat: Text.PlainText
    text: label + ":  " + value
    Accessible.name: label
    Accessible.description: value
}
