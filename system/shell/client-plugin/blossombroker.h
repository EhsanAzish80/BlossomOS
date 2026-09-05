#pragma once

#include <QObject>
#include <QDBusServiceWatcher>
#include <QQmlEngine>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

class BlossomBroker final : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON
    Q_PROPERTY(QString state READ state NOTIFY stateChanged FINAL)
    Q_PROPERTY(QVariantMap preview READ preview NOTIFY previewChanged FINAL)
    Q_PROPERTY(QVariantList activity READ activity NOTIFY activityChanged FINAL)

public:
    explicit BlossomBroker(QObject *parent = nullptr);

    [[nodiscard]] QString state() const;
    [[nodiscard]] QVariantMap preview() const;
    [[nodiscard]] QVariantList activity() const;

    Q_INVOKABLE void requestSystemUname();
    Q_INVOKABLE void approveOnce();
    Q_INVOKABLE void deny();
    Q_INVOKABLE void cancelPending();
    Q_INVOKABLE void refreshActivity(qulonglong afterSequence = 0, bool hasCursor = false);

signals:
    void stateChanged();
    void previewChanged();
    void activityChanged();

private:
    void armExpiryTimer();
    void submitDecision(const QString &decision);
    void handleOutcome(const QByteArray &bytes);
    void failClosed();
    void setState(const QString &value);

    QString m_state = QStringLiteral("idle");
    QVariantMap m_preview;
    QVariantList m_activity;
    QDBusServiceWatcher m_serviceWatcher;
    QTimer m_expiryTimer;
    quint64 m_serviceGeneration = 0;
};
