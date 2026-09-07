#include "blossombroker.h"

#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDateTime>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#include <chrono>

namespace {
constexpr auto BusName = "org.blossomos.Shell1";
constexpr auto ObjectPath = "/org/blossomos/Shell1";
constexpr auto Interface = "org.blossomos.Shell1";
constexpr quint16 ProtocolVersion = 1;
constexpr qsizetype MaxReplyBytes = 32 * 1024;
constexpr quint16 ActivityLimit = 64;
constexpr qulonglong MaxApprovalDelayMs = 60 * 1000;
constexpr qulonglong MaxBatteryLifetimeMs = 5 * 1000;

QDBusInterface fixedInterface() {
    return QDBusInterface(QString::fromLatin1(BusName), QString::fromLatin1(ObjectPath),
                          QString::fromLatin1(Interface), QDBusConnection::sessionBus());
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
} // namespace

BlossomBroker::BlossomBroker(QObject *parent) : QObject(parent) {
    m_serviceWatcher.setConnection(QDBusConnection::sessionBus());
    m_serviceWatcher.setWatchMode(QDBusServiceWatcher::WatchForUnregistration);
    m_serviceWatcher.addWatchedService(QString::fromLatin1(BusName));
    connect(&m_serviceWatcher, &QDBusServiceWatcher::serviceUnregistered,
            this, [this](const QString &) {
                ++m_serviceGeneration;
                failClosed();
            });
    m_expiryTimer.setSingleShot(true);
    connect(&m_expiryTimer, &QTimer::timeout, this, &BlossomBroker::cancelPending);
    m_batteryExpiryTimer.setSingleShot(true);
    connect(&m_batteryExpiryTimer, &QTimer::timeout, this, &BlossomBroker::refreshBattery);
}

QString BlossomBroker::state() const { return m_state; }
QVariantMap BlossomBroker::preview() const { return m_preview; }
QVariantList BlossomBroker::activity() const { return m_activity; }
QVariantMap BlossomBroker::battery() const { return m_battery; }

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
    setState(QStringLiteral("submitting"));
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
        if (m_state == QStringLiteral("unavailable")) {
            setState(QStringLiteral("idle"));
        }
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

void BlossomBroker::handleOutcome(const QByteArray &bytes) {
    bool ok = false;
    const auto object = boundedObject(bytes, &ok);
    if (!ok) {
        failClosed();
        return;
    }
    const auto status = object.value(QStringLiteral("status")).toString();
    if (status == QStringLiteral("awaiting_approval") && object.value(QStringLiteral("preview")).isObject()) {
        m_preview = object.value(QStringLiteral("preview")).toObject().toVariantMap();
        emit previewChanged();
        setState(QStringLiteral("waiting"));
        armExpiryTimer();
        return;
    }
    if (status == QStringLiteral("denied") || status == QStringLiteral("cancelled") ||
        status == QStringLiteral("expired") ||
        status == QStringLiteral("verified") || status == QStringLiteral("verification_failed")) {
        m_preview.clear();
        emit previewChanged();
        setState(status);
        refreshActivity();
        return;
    }
    failClosed();
}

void BlossomBroker::failClosed() {
    m_expiryTimer.stop();
    m_preview.clear();
    emit previewChanged();
    clearBattery();
    setState(QStringLiteral("unavailable"));
}

void BlossomBroker::clearBattery() {
    m_batteryExpiryTimer.stop();
    const QVariantMap unavailable{{QStringLiteral("status"), QStringLiteral("unavailable")}};
    if (m_battery == unavailable) return;
    m_battery = unavailable;
    emit batteryChanged();
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
