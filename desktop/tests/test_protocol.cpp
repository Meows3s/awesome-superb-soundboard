#include <QtTest>
#include "Protocol.h"

using namespace Protocol;
using T = Event::Type;

class TestProtocol : public QObject {
    Q_OBJECT
private slots:
    void keyEvents() {
        Event e = parseLine("K031\n");
        QCOMPARE(e.type, T::Key);
        QCOMPARE(e.key, 2);
        QVERIFY(e.down);
        e = parseLine("K120");
        QCOMPARE(e.key, 11);
        QVERIFY(!e.down);
    }
    void rejectsBadKeys() {
        QCOMPARE(parseLine("K001").type, T::Invalid);
        QCOMPARE(parseLine("K131").type, T::Invalid);
        QCOMPARE(parseLine("K012").type, T::Invalid);
        QCOMPARE(parseLine("Kab1").type, T::Invalid);
        QCOMPARE(parseLine("").type, T::Invalid);
    }
    void encoderSwitchPong() {
        QCOMPARE(parseLine("E+").delta, 1);
        QCOMPARE(parseLine("E-\r\n").delta, -1);
        QVERIFY(parseLine("S1").down);
        QVERIFY(!parseLine("S0").down);
        QCOMPARE(parseLine("PONG").type, T::Pong);
    }
    void mask() {
        const Event e = parseLine("R0a7");
        QCOMPARE(e.type, T::Mask);
        QCOMPARE(e.mask, quint16(0x0a7));
    }
    void builders() {
        QCOMPARE(ping(), QByteArray("PING\n"));
        QCOMPARE(setColor(0, QColor(255, 128, 0)), QByteArray("COL 01 ff8000\n"));
        QCOMPARE(setColor(11, QColor(0, 0, 10)), QByteArray("COL 12 00000a\n"));
        QCOMPARE(setLabel(2, "Airhorn!!!"), QByteArray("LBL 03 Airhorn\n"));
        QCOMPARE(setLabel(0, QString::fromUtf8("h\u00e9llo")), QByteArray("LBL 01 h?llo\n"));
        QCOMPARE(setLabel(4, ""), QByteArray("LBL 05 \n"));
        QCOMPARE(setVolume(150), QByteArray("VOL 100\n"));
        QCOMPARE(setVolume(-5), QByteArray("VOL 0\n"));
    }
};
QTEST_APPLESS_MAIN(TestProtocol)
#include "test_protocol.moc"
