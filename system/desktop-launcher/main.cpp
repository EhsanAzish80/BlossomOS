#include <QCoreApplication>
#include <QDBusConnection>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QObject>
#include <QProcess>
#include <QRegularExpression>
#include <QSaveFile>
#include <QStringList>

class DesktopLauncher final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.blossomos.Desktop1")

public slots:
    QByteArray QuickStatus1() const {
        QString network = QStringLiteral("unavailable");
        const QString networkOutput = runBounded(
            QStringLiteral("/usr/bin/nmcli"),
            {QStringLiteral("-t"), QStringLiteral("-f"), QStringLiteral("TYPE,STATE"), QStringLiteral("device")});
        if (networkOutput.contains(QStringLiteral("wifi:connected"))) network = QStringLiteral("wifi");
        else if (networkOutput.contains(QStringLiteral("ethernet:connected"))) network = QStringLiteral("ethernet");
        else if (!networkOutput.isNull()) network = QStringLiteral("disconnected");

        int volume = -1;
        bool muted = false;
        const QString volumeOutput = runBounded(
            QStringLiteral("/usr/bin/wpctl"),
            {QStringLiteral("get-volume"), QStringLiteral("@DEFAULT_AUDIO_SINK@")});
        const QRegularExpression expression(QStringLiteral(R"(Volume:\s+([0-9]+(?:\.[0-9]+)?))"));
        const auto match = expression.match(volumeOutput);
        if (match.hasMatch()) {
            volume = qBound(0, qRound(match.captured(1).toDouble() * 100.0), 150);
            muted = volumeOutput.contains(QStringLiteral("[MUTED]"));
        }

        QString bluetooth = QStringLiteral("unavailable");
        const QString bluetoothOutput = runBounded(QStringLiteral("/usr/bin/bluetoothctl"), {QStringLiteral("show")});
        if (!bluetoothOutput.isNull()) {
            bluetooth = bluetoothOutput.contains(QStringLiteral("Powered: yes"))
                ? QStringLiteral("on") : QStringLiteral("off");
        }
        return QJsonDocument(QJsonObject{{QStringLiteral("schema"), 1},
                                         {QStringLiteral("network"), network},
                                         {QStringLiteral("volume_percent"), volume},
                                         {QStringLiteral("muted"), muted},
                                         {QStringLiteral("bluetooth"), bluetooth}})
            .toJson(QJsonDocument::Compact);
    }

    bool OnboardingRequired1() const {
        if (qEnvironmentVariableIsSet("BLOSSOM_LIVE")) return true;
        const QString stateDirectory = qEnvironmentVariable("STATE_DIRECTORY");
        return stateDirectory.isEmpty() ||
               !QFileInfo::exists(stateDirectory + QStringLiteral("/onboarding-complete"));
    }

    bool CompleteOnboarding1() {
        if (qEnvironmentVariableIsSet("BLOSSOM_LIVE")) return true;
        const QString stateDirectory = qEnvironmentVariable("STATE_DIRECTORY");
        if (stateDirectory.isEmpty()) return false;
        QSaveFile marker(stateDirectory + QStringLiteral("/onboarding-complete"));
        if (!marker.open(QIODevice::WriteOnly | QIODevice::Text)) return false;
        if (marker.write("schema=1\n") != 9) return false;
        return marker.commit();
    }

    bool Launch1(const QString &action) {
        QStringList command;
        if (action == QStringLiteral("terminal")) command = {QStringLiteral("/usr/bin/foot")};
        else if (action == QStringLiteral("files")) command = {QStringLiteral("/usr/bin/thunar")};
        else if (action == QStringLiteral("browser")) command = {QStringLiteral("/usr/bin/firefox")};
        else if (action == QStringLiteral("editor")) command = {QStringLiteral("/usr/bin/mousepad")};
        else if (action == QStringLiteral("network")) command = {QStringLiteral("/usr/bin/nm-connection-editor")};
        else if (action == QStringLiteral("audio")) command = {QStringLiteral("/usr/bin/pavucontrol")};
        else if (action == QStringLiteral("bluetooth")) command = {QStringLiteral("/usr/bin/blueman-manager")};
        else if (action == QStringLiteral("audio-mute")) command = {QStringLiteral("/usr/bin/wpctl"), QStringLiteral("set-mute"), QStringLiteral("@DEFAULT_AUDIO_SINK@"), QStringLiteral("toggle")};
        else if (action == QStringLiteral("audio-down")) command = {QStringLiteral("/usr/bin/wpctl"), QStringLiteral("set-volume"), QStringLiteral("@DEFAULT_AUDIO_SINK@"), QStringLiteral("5%-")};
        else if (action == QStringLiteral("audio-up")) command = {QStringLiteral("/usr/bin/wpctl"), QStringLiteral("set-volume"), QStringLiteral("@DEFAULT_AUDIO_SINK@"), QStringLiteral("5%+")};
        else if (action == QStringLiteral("notifications")) command = {QStringLiteral("/usr/bin/makoctl"), QStringLiteral("mode"), QStringLiteral("-t"), QStringLiteral("do-not-disturb")};
        else if (action == QStringLiteral("logout")) command = {QStringLiteral("/usr/bin/hyprctl"), QStringLiteral("dispatch"), QStringLiteral("exit")};
        else if (action == QStringLiteral("installer")) command = {QStringLiteral("/usr/lib/blossom-os/blossom-installer")};
        else if (action == QStringLiteral("restart")) command = {QStringLiteral("/usr/bin/systemctl"), QStringLiteral("reboot")};
        else if (action == QStringLiteral("poweroff")) command = {QStringLiteral("/usr/bin/systemctl"), QStringLiteral("poweroff")};
        else return false;

        QStringList dispatch{QStringLiteral("dispatch"), QStringLiteral("exec"), QStringLiteral("--")};
        dispatch.append(command);
        return QProcess::startDetached(QStringLiteral("/usr/bin/hyprctl"), dispatch);
    }

private:
    static QString runBounded(const QString &program, const QStringList &arguments) {
        QProcess process;
        process.setProgram(program);
        process.setArguments(arguments);
        process.setProcessChannelMode(QProcess::SeparateChannels);
        process.start();
        if (!process.waitForStarted(500) || !process.waitForFinished(1000)) {
            process.kill();
            process.waitForFinished(250);
            return QString();
        }
        const QByteArray output = process.readAllStandardOutput();
        if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0 ||
            !process.readAllStandardError().isEmpty() || output.size() > 16 * 1024) return QString();
        return QString::fromUtf8(output);
    }
};

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    DesktopLauncher launcher;
    auto bus = QDBusConnection::sessionBus();
    if (!bus.registerService(QStringLiteral("org.blossomos.Desktop1")) ||
        !bus.registerObject(QStringLiteral("/org/blossomos/Desktop1"), &launcher,
                            QDBusConnection::ExportAllSlots)) return 1;
    return app.exec();
}

#include "main.moc"
