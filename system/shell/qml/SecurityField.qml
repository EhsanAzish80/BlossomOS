import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Petal"

// One service-authored preview field: sans caption over its value. The
// accessible node keeps the exact label as its name and the raw value as
// its description; the visible text may only be regrouped for reading.
Label {
    id: field
    required property string label
    required property string value
    property string shown: value
    property bool mono: false

    Layout.fillWidth: true
    topPadding: caption.implicitHeight + 3
    color: Theme.text
    font.family: field.mono ? Theme.mono : Theme.sans
    font.pixelSize: Theme.label
    elide: wrapMode === Text.NoWrap ? Text.ElideMiddle : Text.ElideNone
    textFormat: Text.PlainText
    text: field.shown.length > 0 ? field.shown : "—"
    Accessible.role: Accessible.StaticText
    Accessible.name: label
    Accessible.description: value
    Accessible.ignored: false

    Text {
        id: caption
        anchors.left: parent.left
        anchors.top: parent.top
        text: field.label
        color: Theme.textTertiary
        font.family: Theme.sans
        font.pixelSize: Theme.caption
        Accessible.ignored: true
    }
}
