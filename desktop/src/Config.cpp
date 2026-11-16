#include "Config.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QStandardPaths>

AppConfig::AppConfig() {
    for (int i = 0; i < Protocol::kKeyCount; ++i)
        keys[i].color = QColor::fromHsv(i * 360 / Protocol::kKeyCount, 255, 255);
}

QString AppConfig::defaultPath() {
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/config.json";
}

bool AppConfig::load(const QString &path) {
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly)) return false;
    const QJsonObject o = QJsonDocument::fromJson(f.readAll()).object();
    volume = qBound(0, o.value("volume").toInt(volume), 100);
    outputDevice = o.value("outputDevice").toString();
    inputDevice = o.value("inputDevice").toString();
    micEnabled = o.value("micEnabled").toBool(true);
    const QJsonArray arr = o.value("keys").toArray();
    for (int i = 0; i < Protocol::kKeyCount && i < arr.size(); ++i) {
        const QJsonObject k = arr[i].toObject();
        keys[i].label = k.value("label").toString();
        keys[i].sound = k.value("sound").toString();
        const QColor c(k.value("color").toString());
        if (c.isValid()) keys[i].color = c;
    }
    return true;
}

bool AppConfig::save(const QString &path) const {
    QDir().mkpath(QFileInfo(path).absolutePath());
    QJsonArray arr;
    for (const KeyConfig &k : keys)
        arr.append(QJsonObject{{"label", k.label}, {"sound", k.sound}, {"color", k.color.name()}});
    const QJsonObject o{{"volume", volume},         {"outputDevice", outputDevice},
                        {"inputDevice", inputDevice}, {"micEnabled", micEnabled},
                        {"keys", arr}};
    QSaveFile f(path);
    if (!f.open(QIODevice::WriteOnly)) return false;
    f.write(QJsonDocument(o).toJson());
    return f.commit();
}

QString AppConfig::displayLabel(int key) const {
    const KeyConfig &k = keys.at(key);
    if (!k.label.isEmpty()) return k.label;
    if (!k.sound.isEmpty()) return QFileInfo(k.sound).completeBaseName();
    return {};
}
