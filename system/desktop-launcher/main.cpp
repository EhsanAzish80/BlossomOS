#include <QCoreApplication>
#include <QDBusConnection>
#include <QObject>
#include <QProcess>
#include <QStringList>

class DesktopLauncher final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.blossomos.Desktop1")

public slots:
    bool Launch1(const QString &action) {
        QStringList command;
        if (action == QStringLiteral("terminal")) command = {QStringLiteral("/usr/bin/foot")};
        else if (action == QStringLiteral("files")) command = {QStringLiteral("/usr/bin/thunar")};
        else if (action == QStringLiteral("browser")) command = {QStringLiteral("/usr/bin/firefox")};
        else if (action == QStringLiteral("editor")) command = {QStringLiteral("/usr/bin/mousepad")};
        else if (action == QStringLiteral("network")) command = {QStringLiteral("/usr/bin/foot"), QStringLiteral("--title=Network Settings"), QStringLiteral("/usr/bin/nmtui")};
        else if (action == QStringLiteral("audio")) command = {QStringLiteral("/usr/bin/pavucontrol")};
        else if (action == QStringLiteral("installer")) command = {QStringLiteral("/usr/bin/foot"), QStringLiteral("--title=Install Blossom OS"), QStringLiteral("/usr/bin/pkexec"), QStringLiteral("/usr/local/bin/blossom-physical-install")};
        else if (action == QStringLiteral("restart")) command = {QStringLiteral("/usr/bin/systemctl"), QStringLiteral("reboot")};
        else if (action == QStringLiteral("poweroff")) command = {QStringLiteral("/usr/bin/systemctl"), QStringLiteral("poweroff")};
        else return false;

        QStringList dispatch{QStringLiteral("dispatch"), QStringLiteral("exec"), QStringLiteral("--")};
        dispatch.append(command);
        return QProcess::startDetached(QStringLiteral("/usr/bin/hyprctl"), dispatch);
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
