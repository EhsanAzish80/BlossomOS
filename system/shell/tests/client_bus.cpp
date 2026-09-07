// Run on an isolated session bus with the real Blossom service already running.
// This test only requests previews, denies and cancels; it never approves.
#include "blossombroker.h"
#include <QCoreApplication>
#include <QDBusMetaType>
#include <QEventLoop>
#include <QTimer>
#include <cstdio>

template <typename Start, typename Check>
bool observe(BlossomBroker &broker, Start start, Check check) {
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    QObject::connect(&broker, &BlossomBroker::stateChanged, &loop, [&] {
        if (check() || broker.state() == "unavailable") loop.quit();
    });
    QObject::connect(&broker, &BlossomBroker::activityChanged, &loop, [&] {
        if (check()) loop.quit();
    });
    start();
    timeout.start(5000);
    loop.exec();
    return check();
}

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    const auto exact = QVariant::fromValue(quint16(1));
    const auto promoted = QVariant(quint16(1));
    if (QByteArray(QDBusMetaType::typeToSignature(exact.metaType())) != "q") return 1;
    std::printf("Qt implicit signature: %s; explicit signature: q\n",
                QDBusMetaType::typeToSignature(promoted.metaType()));
    BlossomBroker broker;
    bool activityReceived = false;
    QObject::connect(&broker, &BlossomBroker::activityChanged, &app, [&] { activityReceived = true; });
    if (!observe(broker, [&] { broker.refreshActivity(); }, [&] { return activityReceived; })) return 2;
    if (!observe(broker, [&] { broker.requestSystemUname(); }, [&] { return broker.state() == "waiting"; })) return 3;
    if (!observe(broker, [&] { broker.deny(); }, [&] { return broker.state() == "denied"; })) return 4;
    const auto deniedActivityCount = broker.activity().size();
    if (!observe(broker, [] {}, [&] { return broker.activity().size() > deniedActivityCount; })) return 7;
    if (broker.state() != "denied") return 8;
    if (!observe(broker, [&] { broker.requestSystemUname(); }, [&] { return broker.state() == "waiting"; })) return 5;
    if (!observe(broker, [&] { broker.cancelPending(); }, [&] { return broker.state() == "cancelled"; })) return 6;
    const auto cancelledActivityCount = broker.activity().size();
    if (!observe(broker, [] {}, [&] { return broker.activity().size() > cancelledActivityCount; })) return 9;
    if (broker.state() != "cancelled") return 10;
    std::puts("Real Qt/Rust bus: activity, preview, denial, cancellation and post-decision refresh passed; no approval sent.");
    return 0;
}
