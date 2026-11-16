#pragma once
#include <QColor>
#include <QString>
#include <array>

#include "Protocol.h"

struct KeyConfig {
    QString label;  // empty -> derived from the sound file name
    QString sound;  // path to audio file, empty = unbound
    QColor color;
};

struct AppConfig {
    std::array<KeyConfig, Protocol::kKeyCount> keys;
    int volume = 60;          // effect volume, 0..100 (the knob)
    QString outputDevice;     // audio device description (VB-Cable input)
    QString inputDevice;      // microphone description
    bool micEnabled = true;

    AppConfig();
    static QString defaultPath();
    bool load(const QString &path = defaultPath());
    bool save(const QString &path = defaultPath()) const;

    // Text shown on the OLED / button: explicit label, else the file's base name.
    QString displayLabel(int key) const;
};
