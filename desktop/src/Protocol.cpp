#include "Protocol.h"

#include <algorithm>

namespace Protocol {

Event parseLine(const QByteArray &raw) {
    const QByteArray line = raw.trimmed();
    Event ev;
    if (line == "PONG") {
        ev.type = Event::Type::Pong;
    } else if (line == "E+" || line == "E-") {
        ev.type = Event::Type::Encoder;
        ev.delta = (line == "E+") ? 1 : -1;
    } else if (line == "S0" || line == "S1") {
        ev.type = Event::Type::Switch;
        ev.down = (line == "S1");
    } else if (line.size() == 4 && line[0] == 'K') {
        bool ok = false;
        const int nn = line.mid(1, 2).toInt(&ok);
        const char d = line[3];
        if (ok && nn >= 1 && nn <= kKeyCount && (d == '0' || d == '1')) {
            ev.type = Event::Type::Key;
            ev.key = nn - 1;
            ev.down = (d == '1');
        }
    } else if (line.size() == 4 && line[0] == 'R') {
        bool ok = false;
        const uint m = line.mid(1, 3).toUInt(&ok, 16);
        if (ok) {
            ev.type = Event::Type::Mask;
            ev.mask = static_cast<quint16>(m & 0xfff);
        }
    }
    return ev;
}

QByteArray ping() { return "PING\n"; }

QByteArray setColor(int key, const QColor &c) {
    return QByteArray("COL ") + QByteArray::number(key + 1).rightJustified(2, '0') + ' ' +
           QByteArray::number(c.rgb() & 0xffffff, 16).rightJustified(6, '0') + '\n';
}

QByteArray setLabel(int key, const QString &label) {
    QByteArray text = label.toLatin1().left(kMaxLabelLen);
    for (char &ch : text) {
        if (ch < 32 || ch > 126) ch = '?';
    }
    return QByteArray("LBL ") + QByteArray::number(key + 1).rightJustified(2, '0') + ' ' + text + '\n';
}

QByteArray setVolume(int percent) {
    return "VOL " + QByteArray::number(std::clamp(percent, 0, 100)) + '\n';
}

}  // namespace Protocol
