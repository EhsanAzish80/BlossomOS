#include <QCoreApplication>
#include <QGuiApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QObject>
#include <QProcess>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QStringList>
#include <QUrl>

class InstallerController final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString targetSummary READ targetSummary NOTIFY observationChanged FINAL)
    Q_PROPERTY(QString expectedConfirmation READ expectedConfirmation NOTIFY observationChanged FINAL)
    Q_PROPERTY(QString observationPath READ observationPath NOTIFY observationChanged FINAL)
    Q_PROPERTY(QString status READ status NOTIFY statusChanged FINAL)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged FINAL)

public:
    explicit InstallerController(QObject *parent = nullptr) : QObject(parent) {}
    QString targetSummary() const { return m_targetSummary; }
    QString expectedConfirmation() const { return m_expectedConfirmation; }
    QString observationPath() const { return m_observationPath; }
    QString status() const { return m_status; }
    bool busy() const { return m_busy; }

    Q_INVOKABLE void observeTarget() {
        if (m_busy) return;
        setBusy(true);
        setStatus(QStringLiteral("Checking hardware and eligible installation disks…"));
        auto *process = new QProcess(this);
        process->setProgram(QStringLiteral("/usr/local/bin/blossom-install-observe"));
        process->setProcessChannelMode(QProcess::SeparateChannels);
        connect(process, &QProcess::errorOccurred, this, [this, process](QProcess::ProcessError error) {
            if (error != QProcess::FailedToStart) return;
            setBusy(false);
            setStatus(QStringLiteral("Could not start hardware observation."));
        });
        connect(process, &QProcess::finished, this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = process->readAllStandardOutput();
            const QString error = QString::fromUtf8(process->readAllStandardError().left(4096)).trimmed();
            process->deleteLater();
            setBusy(false);
            if (exitStatus != QProcess::NormalExit || exitCode != 0 || output.size() > 64 * 1024) {
                setStatus(error.isEmpty() ? QStringLiteral("No qualified installation disk was found.") : error);
                return;
            }
            const auto document = QJsonDocument::fromJson(output);
            if (!document.isObject()) { setStatus(QStringLiteral("Hardware observation returned invalid data.")); return; }
            const auto object = document.object();
            const auto target = object.value(QStringLiteral("target")).toObject();
            m_observationPath = object.value(QStringLiteral("observation")).toString();
            m_expectedConfirmation = object.value(QStringLiteral("expected_confirmation")).toString();
            m_targetSummary = QStringLiteral("%1 · %2 · %3 bytes")
                .arg(target.value(QStringLiteral("path")).toString(),
                     target.value(QStringLiteral("model")).toString(),
                     QString::number(target.value(QStringLiteral("size_bytes")).toInteger()));
            emit observationChanged();
            setStatus(QStringLiteral("Qualified target observed. Review its identity carefully."));
        });
        process->start();
    }

    Q_INVOKABLE void openNetworkSettings() {
        QProcess::startDetached(QStringLiteral("/usr/bin/nm-connection-editor"), QStringList{});
        setStatus(QStringLiteral("Network settings opened. You may also continue offline."));
    }

    Q_INVOKABLE void install(const QString &fullName, const QString &username,
                             const QString &password, const QString &passwordConfirmation,
                             const QString &locale, const QString &timezone,
                             const QString &keymap, bool administrator,
                             const QString &agentMode, const QString &confirmation) {
        if (m_busy || m_observationPath.isEmpty()) return;
        QJsonObject profile{{QStringLiteral("full_name"), fullName},
                            {QStringLiteral("username"), username},
                            {QStringLiteral("locale"), locale},
                            {QStringLiteral("timezone"), timezone},
                            {QStringLiteral("keymap"), keymap},
                            {QStringLiteral("administrator"), administrator},
                            {QStringLiteral("agent_mode"), agentMode}};
        QJsonObject request{{QStringLiteral("schema"), 1},
                            {QStringLiteral("observation"), m_observationPath},
                            {QStringLiteral("confirmation"), confirmation},
                            {QStringLiteral("profile"), profile},
                            {QStringLiteral("password"), password},
                            {QStringLiteral("password_confirmation"), passwordConfirmation}};
        auto *process = new QProcess(this);
        process->setProgram(QStringLiteral("/usr/bin/pkexec"));
        process->setArguments({QStringLiteral("/usr/local/libexec/blossom-graphical-install-backend")});
        process->setProcessChannelMode(QProcess::SeparateChannels);
        connect(process, &QProcess::errorOccurred, this, [this, process](QProcess::ProcessError error) {
            if (error != QProcess::FailedToStart) return;
            setBusy(false);
            setStatus(QStringLiteral("Could not start the privileged installer backend."));
        });
        connect(process, &QProcess::started, this, [process, request] {
            process->write(QJsonDocument(request).toJson(QJsonDocument::Compact));
            process->closeWriteChannel();
        });
        connect(process, &QProcess::finished, this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = process->readAllStandardOutput();
            const QString error = QString::fromUtf8(process->readAllStandardError().left(4096)).trimmed();
            process->deleteLater();
            setBusy(false);
            if (exitStatus == QProcess::NormalExit && exitCode == 0 &&
                output.contains("physical_install_completed")) {
                setStatus(QStringLiteral("Blossom OS was installed successfully. Restart when ready."));
                emit installationCompleted();
            } else {
                setStatus(error.isEmpty() ? QStringLiteral("Installation failed safely. No automatic retry was attempted.") : error);
            }
        });
        setBusy(true);
        setStatus(QStringLiteral("Installing verified Blossom OS files… Keep this computer connected to power."));
        process->start();
    }

signals:
    void observationChanged();
    void statusChanged();
    void busyChanged();
    void installationCompleted();

private:
    void setStatus(const QString &value) { if (m_status == value) return; m_status = value; emit statusChanged(); }
    void setBusy(bool value) { if (m_busy == value) return; m_busy = value; emit busyChanged(); }
    QString m_targetSummary;
    QString m_expectedConfirmation;
    QString m_observationPath;
    QString m_status = QStringLiteral("Ready to begin.");
    bool m_busy = false;
};

int main(int argc, char **argv) {
    QGuiApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("Blossom OS Installer"));
    InstallerController controller;
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("Installer"), &controller);
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed,
                     &app, [] { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.load(QUrl(QStringLiteral("file:///usr/share/blossom-os/installer/Main.qml")));
    return app.exec();
}

#include "main.moc"
