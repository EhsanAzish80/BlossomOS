import QtQuick
import Blossom.Shell
import "../qml"

QtObject {
    id: root

    readonly property string maximumName: "a".repeat(60) + ".txt"
    readonly property string maximumContent: "Z".repeat(4096)
    readonly property string workspace: "/home/blossom/Workspace"

    property QtObject fixtureBroker: QtObject {
        property string state: "waiting"
        property var preview: ({
            user_request: "create " + root.maximumName + " containing " + root.maximumContent,
            destination: root.workspace + "/" + root.maximumName,
            content: root.maximumContent,
            content_bytes: 4096,
            proposal_source: "parsed_directly",
            operation: "files.write:create",
            purpose: "Create one workspace file",
            executable: "none",
            arguments: [],
            capability: "files.write:create",
            resource_scope: "workspace only",
            filesystem: "create only; overwrite denied",
            network: "denied; no tool execution",
            privilege: "unprivileged user",
            expected_side_effects: "exactly one new file",
            approval: "once only",
            expires_at_ms: 1790780000000,
            request_id: "accessibility-maximum-preview",
            preview_sha256: "0123456789abcdef".repeat(4)
        })

        function cancelPending() {}
        function deny() {}
        function approveOnce() {}
    }

    property ApprovalPanel panel: ApprovalPanel {
        broker: root.fixtureBroker
    }
}
