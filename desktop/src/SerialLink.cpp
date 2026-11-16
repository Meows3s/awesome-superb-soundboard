#include "SerialLink.h"

#include <QSerialPortInfo>

namespace {
constexpr int kTickMs = 500;
constexpr int kTimeoutMs = 2000;
}  // namespace

SerialLink::SerialLink(QObject *parent) : QObject(parent), m_port(this), m_timer(this) {
    connect(&m_port, &QSerialPort::readyRead, this, &SerialLink::onReadyRead);
    connect(&m_port, &QSerialPort::errorOccurred, this, &SerialLink::onError);
    connect(&m_timer, &QTimer::timeout, this, &SerialLink::tick);
}

void SerialLink::start() {
    m_timer.start(kTickMs);
    tick();
}

void SerialLink::send(const QByteArray &line) {
    if (m_port.isOpen()) m_port.write(line);
}

void SerialLink::tick() {
    if (!m_port.isOpen()) {
        for (const QSerialPortInfo &info : QSerialPortInfo::availablePorts()) {
            if (info.vendorIdentifier() != Protocol::kVendorId ||
                info.productIdentifier() != Protocol::kProductId)
                continue;
            m_port.setPort(info);
            if (m_port.open(QIODevice::ReadWrite)) {
                m_port.setBaudRate(QSerialPort::Baud115200);
                m_port.setDataTerminalReady(true);
                m_rx.clear();
                m_lastRx.restart();
                m_lastError.clear();
                setState(State::NoReply);
                break;
            }
            // Port is there but could not be opened (in use / driver problem).
            const QString err = m_port.errorString();
            if (err != m_lastError) {
                m_lastError = err;
                emit stateChanged(m_state);  // refresh the banner with the reason
            }
        }
        if (!m_port.isOpen()) return;
    }
    send(Protocol::ping());
    if (m_state == State::Connected && m_lastRx.elapsed() > kTimeoutMs) setState(State::NoReply);
}

void SerialLink::onReadyRead() {
    m_rx += m_port.readAll();
    int nl;
    while ((nl = m_rx.indexOf('\n')) >= 0) {
        const QByteArray line = m_rx.left(nl);
        m_rx.remove(0, nl + 1);
        const Protocol::Event ev = Protocol::parseLine(line);
        if (ev.type == Protocol::Event::Type::Invalid) continue;
        m_lastRx.restart();
        setState(State::Connected);
        if (ev.type != Protocol::Event::Type::Pong) emit eventReceived(ev);
    }
    if (m_rx.size() > 4096) m_rx.clear();  // garbage guard
}

void SerialLink::onError(QSerialPort::SerialPortError error) {
    if (error != QSerialPort::NoError) m_lastError = m_port.errorString();
    if (error == QSerialPort::ResourceError || error == QSerialPort::DeviceNotFoundError ||
        error == QSerialPort::PermissionError)
        closePort();
}

void SerialLink::closePort() {
    if (m_port.isOpen()) m_port.close();
    setState(State::Disconnected);
}

void SerialLink::setState(State s) {
    if (s == m_state) return;
    m_state = s;
    emit stateChanged(s);
}
