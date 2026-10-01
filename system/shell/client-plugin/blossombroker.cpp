#include "blossombroker.h"

#include <QDBusError>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDateTime>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStringList>

#include <chrono>

namespace {
constexpr auto BusName = "org.blossomos.Shell1";
constexpr auto ObjectPath = "/org/blossomos/Shell1";
constexpr auto Interface = "org.blossomos.Shell1";
constexpr quint16 ProtocolVersion = 1;
constexpr qsizetype MaxReplyBytes = 32 * 1024;
constexpr qsizetype MaxAgentPromptBytes = 4 * 1024;
constexpr quint16 ActivityLimit = 64;
constexpr qulonglong MaxApprovalDelayMs = 60 * 1000;
constexpr qulonglong MaxBatteryLifetimeMs = 5 * 1000;
constexpr qulonglong MaxNetworkLifetimeMs = 5 * 1000;
constexpr qsizetype MaxFailureReasonCharacters = 240;
constexpr qsizetype MaxCommandRows = 12;

bool safeDisplayText(const QString &text, qsizetype limit) {
    if (text.size() > limit) return false;
    for (const auto character : text) {
        if (!character.isPrint() && character != QLatin1Char('\n')) return false;
    }
    return true;
}

bool validRowId(const QString &id) {
    if (id.size() != 32) return false;
    for (const auto character : id) {
        if (!character.isDigit() && !(character >= QLatin1Char('a') && character <= QLatin1Char('f')))
            return false;
    }
    return true;
}

QDBusInterface fixedInterface() {
    return QDBusInterface(QString::fromLatin1(BusName), QString::fromLatin1(ObjectPath),
                          QString::fromLatin1(Interface), QDBusConnection::sessionBus());
}

QDBusInterface desktopInterface() {
    return QDBusInterface(QStringLiteral("org.blossomos.Desktop1"),
                          QStringLiteral("/org/blossomos/Desktop1"),
                          QStringLiteral("org.blossomos.Desktop1"),
                          QDBusConnection::sessionBus());
}

QJsonObject boundedObject(const QByteArray &bytes, bool *ok) {
    *ok = false;
    if (bytes.size() > MaxReplyBytes) {
        return {};
    }
    QJsonParseError error;
    const auto document = QJsonDocument::fromJson(bytes, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return {};
    }
    *ok = true;
    return document.object();
}

QString boundedFailureReason(const QString &reason) {
    QString result;
    result.reserve(qMin(reason.size(), MaxFailureReasonCharacters));
    for (const auto character : reason.left(MaxFailureReasonCharacters)) {
        if (character.isPrint() || character == QLatin1Char(' ')) {
            result.append(character);
        } else {
            result.append(QChar::Space);
        }
    }
    return result.simplified();
}

QString decisionFailureReason(const QDBusError &error) {
    if (error.type() == QDBusError::AccessDenied) {
        return QStringLiteral("The approval was rejected or expired.");
    }
    const QString detail = boundedFailureReason(error.message());
    return detail.isEmpty() ? QStringLiteral("The approval service became unavailable.") : detail;
}
} // namespace

BlossomBroker::BlossomBroker(QObject *parent) : QObject(parent) {
    m_serviceWatcher.setConnection(QDBusConnection::sessionBus());
    m_serviceWatcher.setWatchMode(QDBusServiceWatcher::WatchForUnregistration);
    m_serviceWatcher.addWatchedService(QString::fromLatin1(BusName));
    connect(&m_serviceWatcher, &QDBusServiceWatcher::serviceUnregistered,
            this, [this](const QString &) {
                ++m_serviceGeneration;
                ++m_commandGeneration;
                m_commandRows.clear();
                emit commandRowsChanged();
                setCommandState(QStringLiteral("error"), QStringLiteral("The Blossom service became unavailable."));
                failClosed();
            });
    m_expiryTimer.setSingleShot(true);
    connect(&m_expiryTimer, &QTimer::timeout, this, &BlossomBroker::cancelPending);
    m_batteryExpiryTimer.setSingleShot(true);
    connect(&m_batteryExpiryTimer, &QTimer::timeout, this, &BlossomBroker::refreshBattery);
    m_networkExpiryTimer.setSingleShot(true);
    connect(&m_networkExpiryTimer, &QTimer::timeout, this, &BlossomBroker::refreshNetwork);
    m_quickStatusTimer.setInterval(5000);
    connect(&m_quickStatusTimer, &QTimer::timeout, this, &BlossomBroker::refreshQuickStatus);
    m_quickStatusTimer.start();
    refreshOnboarding();
}

QString BlossomBroker::state() const { return m_state; }
QVariantMap BlossomBroker::preview() const { return m_preview; }
QVariantList BlossomBroker::activity() const { return m_activity; }
QVariantMap BlossomBroker::battery() const { return m_battery; }
QVariantMap BlossomBroker::network() const { return m_network; }
QVariantMap BlossomBroker::quickStatus() const { return m_quickStatus; }
bool BlossomBroker::liveEnvironment() const {
    return qEnvironmentVariableIsSet("BLOSSOM_LIVE");
}
QString BlossomBroker::desktopMessage() const { return m_desktopMessage; }
QString BlossomBroker::failureReason() const { return m_failureReason; }
bool BlossomBroker::onboardingRequired() const { return m_onboardingRequired; }
QVariantList BlossomBroker::commandRows() const { return m_commandRows; }
QString BlossomBroker::commandState() const { return m_commandState; }
QString BlossomBroker::commandMessage() const { return m_commandMessage; }

void BlossomBroker::setCommandState(const QString &state, const QString &message) {
    if (m_commandState != state) {
        m_commandState = state;
        emit commandStateChanged();
    }
    if (m_commandMessage != message) {
        m_commandMessage = message;
        emit commandMessageChanged();
    }
}

void BlossomBroker::queryCommandBar(const QString &query) {
    const QByteArray queryBytes = query.toUtf8();
    const quint64 commandGeneration = ++m_commandGeneration;
    if (queryBytes.isEmpty()) {
        if (!m_commandRows.isEmpty()) {
            m_commandRows.clear();
            emit commandRowsChanged();
        }
        setCommandState(QStringLiteral("idle"));
        return;
    }
    if (queryBytes.size() > MaxAgentPromptBytes || query.contains(QChar::Null)) {
        setCommandState(QStringLiteral("error"), QStringLiteral("That request is too long."));
        return;
    }
    const QJsonObject request{{QStringLiteral("version"), ProtocolVersion},
                              {QStringLiteral("query"), query}};
    if (!m_commandRows.isEmpty()) {
        m_commandRows.clear();
        emit commandRowsChanged();
    }
    const auto call = fixedInterface().asyncCall(QStringLiteral("QueryCommandBar1"),
        QJsonDocument(request).toJson(QJsonDocument::Compact));
    setCommandState(QStringLiteral("querying"));
    watchCommandRows(call, commandGeneration);
}

void BlossomBroker::queryCommandSuggestions() {
    const quint64 commandGeneration = ++m_commandGeneration;
    if (!m_commandRows.isEmpty()) {
        m_commandRows.clear();
        emit commandRowsChanged();
    }
    const auto call = fixedInterface().asyncCall(QStringLiteral("QueryCommandSuggestions1"),
        QVariant::fromValue(ProtocolVersion));
    setCommandState(QStringLiteral("querying"));
    watchCommandRows(call, commandGeneration);
}

void BlossomBroker::watchCommandRows(
    const QDBusPendingCall &call,
    quint64 commandGeneration
) {
    auto *watcher = new QDBusPendingCallWatcher(call, this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, commandGeneration] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (commandGeneration != m_commandGeneration) return;
        if (reply.isError() || reply.value().size() > MaxReplyBytes) {
            setCommandState(QStringLiteral("error"), QStringLiteral("Blossom could not search right now."));
            return;
        }
        QJsonParseError error;
        const auto document = QJsonDocument::fromJson(reply.value(), &error);
        if (error.error != QJsonParseError::NoError || !document.isArray() ||
            document.array().size() > MaxCommandRows) {
            setCommandState(QStringLiteral("error"), QStringLiteral("Blossom returned an invalid result."));
            return;
        }
        QVariantList rows;
        const QStringList kinds{QStringLiteral("application"), QStringLiteral("workspace_file"),
            QStringLiteral("workspace_create"), QStringLiteral("ask_blossom"),
            QStringLiteral("invalid_create"), QStringLiteral("unsupported")};
        for (const auto value : document.array()) {
            if (!value.isObject()) { rows.clear(); break; }
            const auto row = value.toObject();
            const QString id = row.value(QStringLiteral("id")).toString();
            const QString kind = row.value(QStringLiteral("kind")).toString();
            const QString title = row.value(QStringLiteral("title")).toString();
            const QString detail = row.value(QStringLiteral("detail")).toString();
            const auto badges = row.value(QStringLiteral("badges")).toArray();
            bool validBadges = badges.size() <= 3;
            const QStringList tones{QStringLiteral("local"), QStringLiteral("approval"),
                QStringLiteral("model"), QStringLiteral("neutral")};
            for (const auto badgeValue : badges) {
                if (!badgeValue.isObject()) { validBadges = false; break; }
                const auto badge = badgeValue.toObject();
                if (badge.size() != 2 || !safeDisplayText(badge.value(QStringLiteral("text")).toString(), 128) ||
                    !tones.contains(badge.value(QStringLiteral("tone")).toString())) {
                    validBadges = false; break;
                }
            }
            if (row.size() != 5 || !validRowId(id) || !kinds.contains(kind) || !validBadges ||
                !safeDisplayText(title, 512) || !safeDisplayText(detail, 4096)) {
                rows.clear(); break;
            }
            rows.append(row.toVariantMap());
        }
        if (rows.size() != document.array().size()) {
            setCommandState(QStringLiteral("error"), QStringLiteral("Blossom returned an invalid result."));
            return;
        }
        m_commandRows = rows;
        emit commandRowsChanged();
        setCommandState(QStringLiteral("ready"));
    });
}

void BlossomBroker::activateCommandRow(const QString &rowId) {
    if (!validRowId(rowId) || m_commandState != QStringLiteral("ready")) return;
    const QJsonObject request{{QStringLiteral("version"), ProtocolVersion},
                              {QStringLiteral("row_id"), rowId}};
    const quint64 commandGeneration = ++m_commandGeneration;
    auto *watcher = new QDBusPendingCallWatcher(
        fixedInterface().asyncCall(QStringLiteral("ActivateCommandRow1"),
            QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    setCommandState(QStringLiteral("activating"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, commandGeneration] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (commandGeneration != m_commandGeneration) return;
        if (reply.isError()) {
            setCommandState(QStringLiteral("error"), QStringLiteral("That result expired or could not be opened."));
            return;
        }
        bool parsed = false;
        const auto object = boundedObject(reply.value(), &parsed);
        if (parsed && object.size() == 1 && object.value(QStringLiteral("status")) == QStringLiteral("launched")) {
            setCommandState(QStringLiteral("completed"), QStringLiteral("Opened."));
            return;
        }
        handleOutcome(reply.value());
        if (m_state == QStringLiteral("unsupported")) {
            setCommandState(QStringLiteral("result"),
                QStringLiteral("Blossom can't do this yet. Today it can create one file in your workspace."));
        } else if (m_state == QStringLiteral("model_failed")) {
            setCommandState(QStringLiteral("result"),
                QStringLiteral("Blossom couldn't work out that request. Nothing was done."));
        } else {
            setCommandState(QStringLiteral("completed"));
        }
    });
}

void BlossomBroker::openTerminal() {
    launchDesktop(QStringLiteral("terminal"));
}

void BlossomBroker::openInstaller() {
    if (!liveEnvironment()) {
        return;
    }
    launchDesktop(QStringLiteral("installer"));
}

void BlossomBroker::openFiles() {
    launchDesktop(QStringLiteral("files"));
}

void BlossomBroker::openBrowser() {
    launchDesktop(QStringLiteral("browser"));
}

void BlossomBroker::openEditor() {
    launchDesktop(QStringLiteral("editor"));
}

void BlossomBroker::openNetworkSettings() {
    launchDesktop(QStringLiteral("network"));
}

void BlossomBroker::openAudioSettings() {
    launchDesktop(QStringLiteral("audio"));
}

void BlossomBroker::openBluetoothSettings() {
    launchDesktop(QStringLiteral("bluetooth"));
}

void BlossomBroker::toggleAudioMute() {
    launchDesktop(QStringLiteral("audio-mute"));
}

void BlossomBroker::lowerVolume() {
    launchDesktop(QStringLiteral("audio-down"));
}

void BlossomBroker::raiseVolume() {
    launchDesktop(QStringLiteral("audio-up"));
}

void BlossomBroker::toggleDoNotDisturb() {
    launchDesktop(QStringLiteral("notifications"));
}

void BlossomBroker::logOut() {
    launchDesktop(QStringLiteral("logout"));
}

void BlossomBroker::restartSystem() {
    launchDesktop(QStringLiteral("restart"));
}

void BlossomBroker::powerOff() {
    launchDesktop(QStringLiteral("poweroff"));
}

void BlossomBroker::refreshQuickStatus() {
    const quint64 generation = ++m_quickStatusGeneration;
    auto *watcher = new QDBusPendingCallWatcher(
        desktopInterface().asyncCall(QStringLiteral("QuickStatus1")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, generation] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_quickStatusGeneration) return;
        if (reply.isError() || reply.value().size() > 16 * 1024) {
            clearQuickStatus();
            return;
        }
        bool parsed = false;
        const auto object = boundedObject(reply.value(), &parsed);
        const QString network = object.value(QStringLiteral("network")).toString();
        const QString bluetooth = object.value(QStringLiteral("bluetooth")).toString();
        const int volume = object.value(QStringLiteral("volume_percent")).toInt(-1);
        if (!parsed || object.size() != 5 || object.value(QStringLiteral("schema")).toInt() != 1 ||
            !QStringList{QStringLiteral("wifi"), QStringLiteral("ethernet"), QStringLiteral("disconnected"), QStringLiteral("unavailable")}.contains(network) ||
            !QStringList{QStringLiteral("on"), QStringLiteral("off"), QStringLiteral("unavailable")}.contains(bluetooth) ||
            !object.value(QStringLiteral("muted")).isBool() || volume < -1 || volume > 150) {
            clearQuickStatus();
            return;
        }
        m_quickStatus = object.toVariantMap();
        emit quickStatusChanged();
    });
}

void BlossomBroker::launchDesktop(const QString &action) {
    m_desktopMessage = action == QStringLiteral("installer")
        ? QStringLiteral("Opening the Blossom OS installer…")
        : QStringLiteral("Opening %1…").arg(action);
    emit desktopMessageChanged();
    auto *watcher = new QDBusPendingCallWatcher(
        desktopInterface().asyncCall(QStringLiteral("Launch1"), action), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, action] {
        const QDBusPendingReply<bool> reply = *watcher;
        watcher->deleteLater();
        m_desktopMessage = reply.isError() || !reply.value()
            ? QStringLiteral("Could not open %1. Try again or open Terminal for recovery.").arg(action)
            : QStringLiteral("Opened %1.").arg(action);
        emit desktopMessageChanged();
        if (action.startsWith(QStringLiteral("audio")) || action == QStringLiteral("bluetooth") ||
            action == QStringLiteral("network")) refreshQuickStatus();
    });
}

void BlossomBroker::refreshOnboarding() {
    auto *watcher = new QDBusPendingCallWatcher(
        desktopInterface().asyncCall(QStringLiteral("OnboardingRequired1")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<bool> reply = *watcher;
        watcher->deleteLater();
        const bool required = reply.isError() ? true : reply.value();
        if (required != m_onboardingRequired) {
            m_onboardingRequired = required;
            emit onboardingRequiredChanged();
        }
    });
}

void BlossomBroker::dismissOnboarding() {
    auto *watcher = new QDBusPendingCallWatcher(
        desktopInterface().asyncCall(QStringLiteral("CompleteOnboarding1")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<bool> reply = *watcher;
        watcher->deleteLater();
        if (!reply.isError() && reply.value() && m_onboardingRequired) {
            m_onboardingRequired = false;
            emit onboardingRequiredChanged();
        }
    });
}

void BlossomBroker::requestSystemUname() {
    if (m_state == QStringLiteral("requesting") || m_state == QStringLiteral("waiting") ||
        m_state == QStringLiteral("submitting") || m_state == QStringLiteral("cancelling")) {
        return;
    }
    const quint64 generation = m_serviceGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("StartSystemUname1"), QVariant::fromValue(ProtocolVersion)), this);
    setState(QStringLiteral("requesting"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, generation] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (reply.isError()) {
            failClosed();
            return;
        }
        handleOutcome(reply.value());
    });
}

void BlossomBroker::requestAgentTurn(const QString &prompt) {
    if (m_state == QStringLiteral("requesting") || m_state == QStringLiteral("waiting") ||
        m_state == QStringLiteral("submitting") || m_state == QStringLiteral("cancelling")) {
        return;
    }
    const QByteArray promptBytes = prompt.toUtf8();
    if (promptBytes.isEmpty() || promptBytes.size() > MaxAgentPromptBytes ||
        prompt.contains(QChar::Null)) {
        failClosed();
        return;
    }
    const QJsonObject request{{QStringLiteral("version"), ProtocolVersion},
                              {QStringLiteral("prompt"), prompt}};
    const quint64 generation = m_serviceGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("StartAgentTurn1"),
                            QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    setState(QStringLiteral("requesting"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, generation] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (reply.isError()) {
            if (reply.error().type() == QDBusError::InvalidArgs) {
                failClosed(QStringLiteral("The direct create request has an invalid file name or content."));
            } else {
                failClosed();
            }
            return;
        }
        handleOutcome(reply.value());
    });
}

void BlossomBroker::approveOnce() { submitDecision(QStringLiteral("approve_once")); }
void BlossomBroker::deny() { submitDecision(QStringLiteral("deny")); }

void BlossomBroker::submitDecision(const QString &decision) {
    if (m_state != QStringLiteral("waiting") ||
        (decision != QStringLiteral("approve_once") && decision != QStringLiteral("deny"))) {
        return;
    }
    const QJsonObject request{{QStringLiteral("kind"), QStringLiteral("submit_decision")},
                              {QStringLiteral("version"), ProtocolVersion},
                              {QStringLiteral("request_id"), m_preview.value(QStringLiteral("request_id")).toString()},
                              {QStringLiteral("preview_sha256"), m_preview.value(QStringLiteral("preview_sha256")).toString()},
                              {QStringLiteral("decision"), decision}};
    const quint64 generation = m_serviceGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("SubmitDecision1"), QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    clearFailureReason();
    setState(QStringLiteral("submitting"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, generation] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (reply.isError()) {
            failClosed(decisionFailureReason(reply.error()));
            refreshActivity();
            return;
        }
        handleOutcome(reply.value());
    });
}

void BlossomBroker::cancelPending() {
    if (m_state != QStringLiteral("waiting")) {
        return;
    }
    const QJsonObject request{{QStringLiteral("kind"), QStringLiteral("cancel_pending")},
                              {QStringLiteral("version"), ProtocolVersion},
                              {QStringLiteral("request_id"), m_preview.value(QStringLiteral("request_id")).toString()},
                              {QStringLiteral("preview_sha256"), m_preview.value(QStringLiteral("preview_sha256")).toString()}};
    const quint64 generation = m_serviceGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("CancelPending1"), QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    setState(QStringLiteral("cancelling"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher, generation] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (reply.isError()) {
            failClosed();
            return;
        }
        handleOutcome(reply.value());
    });
}

void BlossomBroker::refreshActivity(qulonglong afterSequence, bool hasCursor) {
    const quint64 generation = m_serviceGeneration;
    const quint64 activityGeneration = ++m_activityGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("ReadActivity1"), QVariant::fromValue(ProtocolVersion), hasCursor,
                            QVariant::fromValue(afterSequence), QVariant::fromValue(ActivityLimit)), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, generation, activityGeneration] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (activityGeneration != m_activityGeneration) {
            return;
        }
        if (reply.isError() || reply.value().size() > MaxReplyBytes) {
            failClosed();
            return;
        }
        QJsonParseError error;
        const auto document = QJsonDocument::fromJson(reply.value(), &error);
        if (error.error != QJsonParseError::NoError || !document.isArray()) {
            failClosed();
            return;
        }
        m_activity = document.array().toVariantList();
        emit activityChanged();
    });
}

void BlossomBroker::refreshBattery() {
    const quint64 generation = m_serviceGeneration;
    const quint64 batteryGeneration = ++m_batteryGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("ReadBatterySummary1"), QVariant::fromValue(ProtocolVersion)), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, generation, batteryGeneration] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (batteryGeneration != m_batteryGeneration) return;
        bool parsed = false;
        const auto object = boundedObject(reply.value(), &parsed);
        const auto status = object.value(QStringLiteral("status")).toString();
        const auto state = object.value(QStringLiteral("state")).toString();
        const auto percentage = object.value(QStringLiteral("percentage"));
        const auto expires = object.value(QStringLiteral("expires_at_ms"));
        const bool present = status == QStringLiteral("present");
        const bool absent = status == QStringLiteral("absent");
        const bool validState = state == QStringLiteral("charging") ||
            state == QStringLiteral("discharging") || state == QStringLiteral("full") ||
            state == QStringLiteral("not_charging") || state == QStringLiteral("unknown");
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        bool validExpiry = false;
        const qulonglong expiresAt = expires.toVariant().toULongLong(&validExpiry);
        const qulonglong remaining = now >= 0 && expiresAt > static_cast<qulonglong>(now)
            ? expiresAt - static_cast<qulonglong>(now) : 0;
        const bool exactShape = object.size() == (present ? 5 : 3);
        if (reply.isError() || !parsed || !validExpiry || !exactShape ||
            object.value(QStringLiteral("version")).toInt() != ProtocolVersion ||
            (!present && !absent) || (present && (!percentage.isDouble() ||
            percentage.toInt() < 0 || percentage.toInt() > 100 || !validState)) ||
            (absent && (object.contains(QStringLiteral("percentage")) || object.contains(QStringLiteral("state")))) ||
            remaining == 0 || remaining > MaxBatteryLifetimeMs) {
            clearBattery();
            return;
        }
        m_battery = object.toVariantMap();
        emit batteryChanged();
        m_batteryExpiryTimer.start(std::chrono::milliseconds(remaining + 1));
    });
}

void BlossomBroker::refreshNetwork() {
    const quint64 generation = m_serviceGeneration;
    const quint64 networkGeneration = ++m_networkGeneration;
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("ReadNetworkConnectivity1"),
                            QVariant::fromValue(ProtocolVersion)), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, generation, networkGeneration] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (generation != m_serviceGeneration) {
            failClosed();
            return;
        }
        if (networkGeneration != m_networkGeneration) return;
        bool parsed = false;
        const auto object = boundedObject(reply.value(), &parsed);
        const auto connectivity = object.value(QStringLiteral("connectivity")).toString();
        const auto expires = object.value(QStringLiteral("expires_at_ms"));
        const bool validConnectivity = connectivity == QStringLiteral("offline") ||
            connectivity == QStringLiteral("local") || connectivity == QStringLiteral("limited") ||
            connectivity == QStringLiteral("online") || connectivity == QStringLiteral("unknown");
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        bool validExpiry = false;
        const qulonglong expiresAt = expires.toVariant().toULongLong(&validExpiry);
        const qulonglong remaining = now >= 0 && expiresAt > static_cast<qulonglong>(now)
            ? expiresAt - static_cast<qulonglong>(now) : 0;
        if (reply.isError() || !parsed || !validExpiry || object.size() != 3 ||
            object.value(QStringLiteral("version")).toInt() != ProtocolVersion ||
            !validConnectivity || remaining == 0 || remaining > MaxNetworkLifetimeMs) {
            clearNetwork();
            return;
        }
        m_network = object.toVariantMap();
        emit networkChanged();
        m_networkExpiryTimer.start(std::chrono::milliseconds(remaining + 1));
    });
}

void BlossomBroker::handleOutcome(const QByteArray &bytes) {
    bool ok = false;
    const auto object = boundedObject(bytes, &ok);
    if (!ok) {
        failClosed();
        return;
    }
    const auto status = object.value(QStringLiteral("status")).toString();
    if (status == QStringLiteral("awaiting_approval") && object.value(QStringLiteral("preview")).isObject()) {
        clearFailureReason();
        m_preview = object.value(QStringLiteral("preview")).toObject().toVariantMap();
        emit previewChanged();
        setState(QStringLiteral("waiting"));
        armExpiryTimer();
        refreshActivity();
        return;
    }
    if (status == QStringLiteral("denied") || status == QStringLiteral("cancelled") ||
        status == QStringLiteral("expired") ||
        status == QStringLiteral("verified") || status == QStringLiteral("verification_failed") ||
        status == QStringLiteral("unsupported") || status == QStringLiteral("model_failed")) {
        m_preview.clear();
        emit previewChanged();
        clearFailureReason();
        setState(status);
        refreshActivity();
        return;
    }
    failClosed();
}

void BlossomBroker::failClosed(const QString &reason) {
    m_expiryTimer.stop();
    m_preview.clear();
    emit previewChanged();
    const QString boundedReason = boundedFailureReason(reason);
    const QString visibleReason = boundedReason.isEmpty()
        ? QStringLiteral("The request was rejected or the local service became unavailable.")
        : boundedReason;
    if (m_failureReason != visibleReason) {
        m_failureReason = visibleReason;
        emit failureReasonChanged();
    }
    clearBattery();
    clearNetwork();
    setState(QStringLiteral("unavailable"));
}

void BlossomBroker::clearFailureReason() {
    if (m_failureReason.isEmpty()) {
        return;
    }
    m_failureReason.clear();
    emit failureReasonChanged();
}

void BlossomBroker::clearBattery() {
    m_batteryExpiryTimer.stop();
    const QVariantMap unavailable{{QStringLiteral("status"), QStringLiteral("unavailable")}};
    if (m_battery == unavailable) return;
    m_battery = unavailable;
    emit batteryChanged();
}

void BlossomBroker::clearNetwork() {
    m_networkExpiryTimer.stop();
    const QVariantMap unavailable{{QStringLiteral("connectivity"), QStringLiteral("unavailable")}};
    if (m_network == unavailable) return;
    m_network = unavailable;
    emit networkChanged();
}

void BlossomBroker::clearQuickStatus() {
    const QVariantMap unavailable{{QStringLiteral("network"), QStringLiteral("unavailable")},
                                  {QStringLiteral("volume_percent"), -1},
                                  {QStringLiteral("muted"), false},
                                  {QStringLiteral("bluetooth"), QStringLiteral("unavailable")}};
    if (m_quickStatus != unavailable) {
        m_quickStatus = unavailable;
        emit quickStatusChanged();
    }
}

void BlossomBroker::setState(const QString &value) {
    if (m_state == value) {
        return;
    }
    m_state = value;
    if (value != QStringLiteral("waiting")) {
        m_expiryTimer.stop();
    }
    emit stateChanged();
}

void BlossomBroker::armExpiryTimer() {
    bool valid = false;
    const qulonglong expiresAt =
        m_preview.value(QStringLiteral("expires_at_ms")).toULongLong(&valid);
    if (!valid) {
        failClosed();
        return;
    }
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (now < 0) {
        failClosed();
        return;
    }
    const qulonglong remaining = expiresAt > static_cast<qulonglong>(now)
        ? expiresAt - static_cast<qulonglong>(now)
        : 0;
    if (remaining > MaxApprovalDelayMs) {
        failClosed();
        return;
    }
    // The service treats now == expires_at as still valid, so cross the
    // boundary by one millisecond and let the service authoritatively expire it.
    const qint64 delay = static_cast<qint64>(remaining + 1);
    m_expiryTimer.start(std::chrono::milliseconds(delay));
}
