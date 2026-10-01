// Display-only command surface. The broker owns routing, opaque row IDs,
// activation, launch authority and approval.
import QtQuick
import QtQuick.Effects
import Quickshell
import Quickshell.Wayland
import Blossom.Shell
import "Petal"

PanelWindow {
    id: bar

    property bool open: false
    property int selectedIndex: 0
    property bool agentRequestActive: false
    property bool lastActivationWasAgent: false
    readonly property var rows: BlossomBroker.commandRows
    readonly property string message: BlossomBroker.commandMessage
    readonly property string displayMessage: agentRequestActive
        ? "Blossom is working locally…"
        : lastActivationWasAgent && BlossomBroker.commandState === "error"
        ? "Blossom couldn't work out that request. Nothing was done."
        : message
    readonly property bool panelVisible: rows.length > 0 || displayMessage.length > 0

    function show() {
        queryDebounce.stop()
        input.clear()
        BlossomBroker.queryCommandBar("")
        selectedIndex = 0
        agentRequestActive = false
        lastActivationWasAgent = false
        open = true
        input.forceActiveFocus()
    }

    function hide() {
        queryDebounce.stop()
        open = false
        input.clear()
        BlossomBroker.queryCommandBar("")
    }

    function toggle() { open ? hide() : show() }

    function closeForActivation() {
        queryDebounce.stop()
        suppressClearQuery = true
        open = false
        input.clear()
        suppressClearQuery = false
    }

    function activateSelected() {
        if (rows.length === 0)
            return
        const selected = rows[Math.min(selectedIndex, rows.length - 1)]
        const id = selected.id
        lastActivationWasAgent = selected.kind === "ask_blossom"
        agentRequestActive = lastActivationWasAgent
        BlossomBroker.activateCommandRow(id)
        if (!lastActivationWasAgent)
            closeForActivation()
    }

    property bool suppressClearQuery: false

    onRowsChanged: selectedIndex = 0

    Connections {
        target: BlossomBroker
        function onStateChanged() {
            if (BlossomBroker.state === "waiting" && bar.agentRequestActive) {
                bar.agentRequestActive = false
                bar.closeForActivation()
            }
        }
        function onCommandStateChanged() {
            if (bar.agentRequestActive && ["result", "error", "completed"].includes(BlossomBroker.commandState))
                bar.agentRequestActive = false
            if (!bar.open && ["result", "error"].includes(BlossomBroker.commandState)
                    && BlossomBroker.commandMessage.length > 0) {
                bar.open = true
                input.forceActiveFocus()
            }
        }
    }

    visible: open
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "blossom-command-bar"
    WlrLayershell.keyboardFocus: open ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Timer {
        id: queryDebounce
        interval: 140
        repeat: false
        onTriggered: BlossomBroker.queryCommandBar(input.text)
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.scrim
        MouseArea { anchors.fill: parent; onClicked: bar.hide() }
    }

    Item {
        id: column
        width: Math.min(720, parent.width - 48)
        x: Math.round((parent.width - width) / 2)
        y: Math.round(parent.height * 0.22)
        height: pill.height + (panel.visible ? panel.height + Theme.s2 : 0)

        MouseArea { anchors.fill: parent; onClicked: input.forceActiveFocus() }

        Rectangle {
            id: pill
            width: parent.width
            height: 60
            radius: 30
            color: Theme.panelFill
            border.color: input.activeFocus ? Theme.info : Theme.lineStrong
            border.width: input.activeFocus ? Theme.focusRing : 1
            layer.enabled: true
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowBlur: 0.8
                shadowVerticalOffset: 10
                shadowColor: Theme.ink0
            }

            Image {
                id: icon
                source: "Petal/icons/search.svg"
                width: 20
                height: 20
                sourceSize: Qt.size(20, 20)
                anchors { left: parent.left; leftMargin: Theme.s6; verticalCenter: parent.verticalCenter }
            }

            TextInput {
                id: input
                anchors {
                    left: icon.right; leftMargin: 14
                    right: parent.right; rightMargin: Theme.s6
                    verticalCenter: parent.verticalCenter
                }
                color: Theme.text
                font.family: Theme.sans
                font.pixelSize: 19
                selectionColor: Theme.blossomTint
                selectedTextColor: Theme.text
                maximumLength: 512
                enabled: !bar.agentRequestActive
                clip: true
                cursorDelegate: Rectangle {
                    width: 2
                    color: Theme.blossom
                    visible: input.activeFocus
                    SequentialAnimation on opacity {
                        loops: Animation.Infinite
                        running: input.activeFocus
                        NumberAnimation { to: 0; duration: 530 }
                        NumberAnimation { to: 1; duration: 530 }
                    }
                }

                Accessible.role: Accessible.EditableText
                Accessible.name: "Blossom command"
                Accessible.description: "Search apps and workspace files, or ask the local Blossom agent."

                onTextChanged: {
                    queryDebounce.stop()
                    if (bar.suppressClearQuery)
                        return
                    if (text.length > 0)
                        queryDebounce.start()
                    else
                        BlossomBroker.queryCommandBar("")
                }

                Keys.onEscapePressed: bar.hide()
                Keys.onReturnPressed: bar.activateSelected()
                Keys.onEnterPressed: bar.activateSelected()
                Keys.onUpPressed: bar.selectedIndex = Math.max(bar.selectedIndex - 1, 0)
                Keys.onDownPressed: {
                    if (text.length === 0 && bar.rows.length === 0) {
                        BlossomBroker.queryCommandSuggestions()
                    } else if (bar.rows.length > 0) {
                        bar.selectedIndex = Math.min(bar.selectedIndex + 1, bar.rows.length - 1)
                    }
                }

                Text {
                    anchors.fill: parent
                    verticalAlignment: Text.AlignVCenter
                    text: "Open an app, find a file, or ask Blossom"
                    color: Theme.textSecondary
                    font: input.font
                    visible: input.text.length === 0
                }
            }
        }

        Rectangle {
            id: panel
            visible: bar.panelVisible
            y: pill.height + Theme.s2
            width: parent.width
            height: content.implicitHeight + Theme.s4
            radius: Theme.radiusPanel
            color: Theme.panelFill
            border.color: Theme.lineStrong
            border.width: 1

            Column {
                id: content
                x: Theme.s2
                y: Theme.s2
                width: parent.width - Theme.s4
                spacing: 2

                Repeater {
                    model: bar.rows
                    delegate: Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        width: content.width
                        height: 52
                        radius: Theme.radiusTile
                        color: index === bar.selectedIndex ? Theme.blossomTint : "transparent"

                        Accessible.role: Accessible.ListItem
                        Accessible.name: modelData.title
                        Accessible.description: modelData.detail

                        Text {
                            anchors {
                                left: parent.left; leftMargin: Theme.s4
                                right: badges.left; rightMargin: Theme.s3
                                verticalCenter: parent.verticalCenter
                            }
                            text: row.modelData.title
                            color: Theme.text
                            font.family: Theme.sans
                            font.pixelSize: Theme.bodyLarge
                            elide: Text.ElideRight
                        }

                        Row {
                            id: badges
                            anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                            spacing: 6
                            Repeater {
                                model: row.modelData.badges || []
                                delegate: Rectangle {
                                    required property var modelData
                                    height: 24
                                    radius: 12
                                    width: badgeText.implicitWidth + Theme.s5
                                    color: "transparent"
                                    border.width: 1
                                    border.color: modelData.tone === "approval" ? Theme.decision
                                        : modelData.tone === "local" ? Theme.verified
                                        : modelData.tone === "model" ? Theme.blossom
                                        : Theme.lineStrong
                                    Text {
                                        id: badgeText
                                        anchors.centerIn: parent
                                        text: modelData.text
                                        font.family: Theme.sans
                                        font.pixelSize: Theme.caption
                                        color: modelData.tone === "approval" ? Theme.decision
                                            : modelData.tone === "local" ? Theme.verified
                                            : modelData.tone === "model" ? Theme.blossom
                                            : Theme.textSecondary
                                    }
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onEntered: bar.selectedIndex = row.index
                            onClicked: { bar.selectedIndex = row.index; bar.activateSelected() }
                        }
                    }
                }

                Item {
                    width: content.width
                    height: msg.implicitHeight + 28
                    visible: bar.displayMessage.length > 0
                    Text {
                        id: msg
                        anchors {
                            left: parent.left; leftMargin: Theme.s4
                            right: parent.right; rightMargin: Theme.s4
                            verticalCenter: parent.verticalCenter
                        }
                        text: bar.displayMessage
                        wrapMode: Text.WordWrap
                        color: Theme.text
                        font.family: Theme.sans
                        font.pixelSize: 15
                    }
                }

                Text {
                    x: Theme.s4
                    topPadding: 6
                    bottomPadding: 2
                    text: bar.rows.length > 0 ? "↑↓ to choose · Enter to run · Esc to close" : "Esc to close"
                    color: Theme.textSecondary
                    font.family: Theme.sans
                    font.pixelSize: Theme.caption
                }
            }
        }
    }
}
