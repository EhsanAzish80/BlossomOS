pragma Singleton
import QtQuick

// Petal design tokens for the Blossom OS shell. Presentation only: no
// behavior, authority, or broker state lives here.
QtObject {
    // Ink neutrals (plum-tinted, dark shell)
    readonly property color ink0: "#0e0c10"
    readonly property color base: "#121015"
    readonly property color surface: "#1a171d"
    readonly property color raised: "#231f27"
    readonly property color hover: "#2c2731"
    readonly property color pressed: "#36303c"
    readonly property color line: "#2f2a34"
    readonly property color lineStrong: "#463f4c"
    readonly property color text: "#f3eef2"
    readonly property color textSecondary: "#bdb3bd"
    readonly property color textTertiary: "#948a95"
    readonly property color textDisabled: "#6a616c"

    // Translucent layers for layer-shell surfaces
    readonly property color barFill: "#e01a171d"
    readonly property color panelFill: "#f51a171d"
    readonly property color scrim: "#bd0e0c10"

    // Semantic accents: each has one meaning
    readonly property color blossom: "#f3a6bc"      // agent, primary action, focus, selection
    readonly property color blossomHover: "#f7c1d1"
    readonly property color blossomPressed: "#e08ba3"
    readonly property color onBlossom: "#2a1520"
    readonly property color blossomTint: "#3a2530"
    readonly property color decision: "#f2c265"     // only: a decision is waiting for you
    readonly property color onDecision: "#2a1e08"
    readonly property color decisionTint: "#3a3020"
    readonly property color verified: "#9bd3a6"     // outcome confirmed by verification
    readonly property color verifiedTint: "#1e2e22"
    readonly property color danger: "#f28b82"       // denied, failed, destructive
    readonly property color dangerPressed: "#e0746b"
    readonly property color onDanger: "#2b0f0d"
    readonly property color dangerTint: "#3a1e1e"
    readonly property color dangerLine: "#5a2a28"
    readonly property color info: "#9ec1f2"         // neutral system facts
    readonly property color infoTint: "#1c2636"

    // Type: IBM Plex (Arch package ttf-ibm-plex); Qt falls back if absent
    readonly property string sans: "IBM Plex Sans"
    readonly property string mono: "IBM Plex Mono"
    readonly property int display: 40
    readonly property int headline: 26
    readonly property int title: 20
    readonly property int bodyLarge: 16
    readonly property int body: 14
    readonly property int label: 13
    readonly property int caption: 12

    // Space (4 px base)
    readonly property int s1: 4
    readonly property int s2: 8
    readonly property int s3: 12
    readonly property int s4: 16
    readonly property int s5: 20
    readonly property int s6: 24
    readonly property int s8: 32
    readonly property int s12: 48

    // Radius
    readonly property int radiusChip: 6
    readonly property int radiusControl: 10
    readonly property int radiusTile: 14
    readonly property int radiusPanel: 20
    readonly property int radiusDock: 28

    // Motion (approval surfaces never animate)
    readonly property int stateMs: 90
    readonly property int popoverMs: 160

    // Control sizes
    readonly property int controlHeight: 40
    readonly property int focusRing: 2
}
