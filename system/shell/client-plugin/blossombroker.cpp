#include "blossombroker.h"

#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

namespace {
constexpr auto BusName = "org.blossomos.Shell1";
constexpr auto ObjectPath = "/org/blossomos/Shell1";
constexpr auto Interface = "org.blossomos.Shell1";
constexpr quint16 ProtocolVersion = 1;
constexpr qsizetype MaxReplyBytes = 32 * 1024;
constexpr quint16 ActivityLimit = 64;

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

BlossomBroker::BlossomBroker(QObject *parent) : QObject(parent) {}

QString BlossomBroker::state() const { return m_state; }
QVariantMap BlossomBroker::preview() const { return m_preview; }
QVariantList BlossomBroker::activity() const { return m_activity; }

void BlossomBroker::requestSystemUname() {
    if (m_state == QStringLiteral("waiting")) {
        return;
    }
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("StartSystemUname1"), ProtocolVersion), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
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
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("SubmitDecision1"), QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    setState(QStringLiteral("submitting"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
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
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("CancelPending1"), QJsonDocument(request).toJson(QJsonDocument::Compact)), this);
    setState(QStringLiteral("cancelling"));
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
        if (reply.isError()) {
            failClosed();
            return;
        }
        handleOutcome(reply.value());
    });
}

void BlossomBroker::refreshActivity(qulonglong afterSequence, bool hasCursor) {
    auto interface = fixedInterface();
    auto *watcher = new QDBusPendingCallWatcher(
        interface.asyncCall(QStringLiteral("ReadActivity1"), ProtocolVersion, hasCursor,
                            QVariant::fromValue(afterSequence), ActivityLimit), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher] {
        const QDBusPendingReply<QByteArray> reply = *watcher;
        watcher->deleteLater();
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
        return;
    }
    if (status == QStringLiteral("denied") || status == QStringLiteral("cancelled") ||
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
    m_preview.clear();
    emit previewChanged();
    setState(QStringLiteral("unavailable"));
}

void BlossomBroker::setState(const QString &value) {
    if (m_state == value) {
        return;
    }
    m_state = value;
    emit stateChanged();
}
