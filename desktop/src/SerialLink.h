#pragma once
#include <QElapsedTimer>
#include <QObject>
#include <QSerialPort>
#include <QTimer>

#include "Protocol.h"

// Finds the macropad by USB VID/PID, keeps the connection alive with a
// heartbeat and reconnects automatically when the board is unplugged.
class SerialLink : public QObject {
    Q_OBJECT
public:
    // NoReply: port opened but the firmware has not answered PING (old firmware?)
    enum class State { Disconnected, NoReply, Connected };
    Q_ENUM(State)

    explicit SerialLink(QObject *parent = nullptr);
    void start();
    State state() const { return m_state; }
    QString portName() const { return m_port.portName(); }
    // Last open/IO error text ("Access denied" = another program holds the port).
    QString lastError() const { return m_lastError; }
    void send(const QByteArray &line);

signals:
    void stateChanged(SerialLink::State state);
    void eventReceived(const Protocol::Event &event);

private slots:
    void tick();
    void onReadyRead();
    void onError(QSerialPort::SerialPortError error);

private:
    void setState(State s);
    void closePort();

    QSerialPort m_port;
    QTimer m_timer;
    QElapsedTimer m_lastRx;
    QByteArray m_rx;
    State m_state = State::Disconnected;
    QString m_lastError;
};
