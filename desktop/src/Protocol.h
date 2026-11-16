#pragma once
#include <QByteArray>
#include <QColor>
#include <QString>

// Wire protocol with the macropad firmware. See docs/PROTOCOL.md.
namespace Protocol {

constexpr int kKeyCount = 12;
constexpr int kMaxLabelLen = 7;
constexpr quint16 kVendorId = 0x2e8a;
constexpr quint16 kProductId = 0x000a;

struct Event {
    enum class Type { Invalid, Key, Encoder, Switch, Mask, Pong };
    Type type = Type::Invalid;
    int key = 0;        // Key: 0-based key index
    bool down = false;  // Key / Switch
    int delta = 0;      // Encoder: +1 / -1
    quint16 mask = 0;   // Mask
};

Event parseLine(const QByteArray &line);

QByteArray ping();
QByteArray setColor(int key, const QColor &color);   // key: 0-based
QByteArray setLabel(int key, const QString &label);  // key: 0-based
QByteArray setVolume(int percent);

}  // namespace Protocol
