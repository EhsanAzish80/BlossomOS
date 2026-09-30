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
    Q_PROPERTY(QVariantMap battery READ battery NOTIFY batteryChanged FINAL)
    Q_PROPERTY(QVariantMap network READ network NOTIFY networkChanged FINAL)
    Q_PROPERTY(QVariantMap quickStatus READ quickStatus NOTIFY quickStatusChanged FINAL)
    Q_PROPERTY(QString desktopMessage READ desktopMessage NOTIFY desktopMessageChanged FINAL)
    Q_PROPERTY(QString failureReason READ failureReason NOTIFY failureReasonChanged FINAL)
    Q_PROPERTY(bool onboardingRequired READ onboardingRequired NOTIFY onboardingRequiredChanged FINAL)
    Q_PROPERTY(bool liveEnvironment READ liveEnvironment CONSTANT FINAL)

public:
    explicit BlossomBroker(QObject *parent = nullptr);

    [[nodiscard]] QString state() const;
    [[nodiscard]] QVariantMap preview() const;
    [[nodiscard]] QVariantList activity() const;
    [[nodiscard]] QVariantMap battery() const;
    [[nodiscard]] QVariantMap network() const;
    [[nodiscard]] QVariantMap quickStatus() const;
    [[nodiscard]] bool liveEnvironment() const;
    [[nodiscard]] QString desktopMessage() const;
    [[nodiscard]] QString failureReason() const;
    [[nodiscard]] bool onboardingRequired() const;

    Q_INVOKABLE void requestSystemUname();
    Q_INVOKABLE void requestAgentTurn(const QString &prompt);
    Q_INVOKABLE void approveOnce();
    Q_INVOKABLE void deny();
    Q_INVOKABLE void cancelPending();
    Q_INVOKABLE void refreshActivity(qulonglong afterSequence = 0, bool hasCursor = false);
    Q_INVOKABLE void refreshBattery();
    Q_INVOKABLE void refreshNetwork();
    Q_INVOKABLE void refreshQuickStatus();
    Q_INVOKABLE void openTerminal();
    Q_INVOKABLE void openInstaller();
    Q_INVOKABLE void openFiles();
    Q_INVOKABLE void openBrowser();
    Q_INVOKABLE void openEditor();
    Q_INVOKABLE void openNetworkSettings();
    Q_INVOKABLE void openAudioSettings();
    Q_INVOKABLE void openBluetoothSettings();
    Q_INVOKABLE void toggleAudioMute();
    Q_INVOKABLE void lowerVolume();
    Q_INVOKABLE void raiseVolume();
    Q_INVOKABLE void toggleDoNotDisturb();
    Q_INVOKABLE void logOut();
    Q_INVOKABLE void restartSystem();
    Q_INVOKABLE void powerOff();
    Q_INVOKABLE void refreshOnboarding();
    Q_INVOKABLE void dismissOnboarding();

signals:
    void stateChanged();
    void previewChanged();
    void activityChanged();
    void batteryChanged();
    void networkChanged();
    void quickStatusChanged();
    void desktopMessageChanged();
    void failureReasonChanged();
    void onboardingRequiredChanged();

private:
    void launchDesktop(const QString &action);
    void armExpiryTimer();
    void submitDecision(const QString &decision);
    void handleOutcome(const QByteArray &bytes);
    void failClosed(const QString &reason = QString());
    void clearFailureReason();
    void clearBattery();
    void clearNetwork();
    void clearQuickStatus();
    void setState(const QString &value);

    QString m_state = QStringLiteral("idle");
    QVariantMap m_preview;
    QVariantList m_activity;
    QVariantMap m_battery{{QStringLiteral("status"), QStringLiteral("unavailable")}};
    QVariantMap m_network{{QStringLiteral("connectivity"), QStringLiteral("unavailable")}};
    QVariantMap m_quickStatus{{QStringLiteral("network"), QStringLiteral("unavailable")},
                              {QStringLiteral("volume_percent"), -1},
                              {QStringLiteral("muted"), false},
                              {QStringLiteral("bluetooth"), QStringLiteral("unavailable")}};
    QString m_desktopMessage;
    QString m_failureReason;
    bool m_onboardingRequired = true;
    QDBusServiceWatcher m_serviceWatcher;
    QTimer m_expiryTimer;
    QTimer m_batteryExpiryTimer;
    QTimer m_networkExpiryTimer;
    QTimer m_quickStatusTimer;
    quint64 m_serviceGeneration = 0;
    quint64 m_activityGeneration = 0;
    quint64 m_batteryGeneration = 0;
    quint64 m_networkGeneration = 0;
    quint64 m_quickStatusGeneration = 0;
};
