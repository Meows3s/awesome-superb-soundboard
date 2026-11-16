#pragma once
#include <QAudioDevice>
#include <QAudioSink>
#include <QAudioSource>
#include <QHash>
#include <QObject>
#include <memory>

#include "Mixer.h"

// Owns the mixer, the microphone capture, the output sink (VB-Cable) and the
// per-key decoded sample cache.
class AudioEngine : public QObject {
    Q_OBJECT
public:
    explicit AudioEngine(QObject *parent = nullptr);
    ~AudioEngine() override;

    static QAudioFormat format();
    // Empty name = system default device.
    bool setOutputDevice(const QString &description);
    bool setInputDevice(const QString &description);
    void setMicEnabled(bool on);
    void setVolume(int percent) { m_mixer.setVolume(percent); }

    // Decode `path` into the cache slot for `key` (async). Empty path clears.
    void loadSound(int key, const QString &path);
    void play(int key);
    void stopAll() { m_mixer.stopAll(); }
    Mixer &mixer() { return m_mixer; }

    // Decode a whole file synchronously-ish; used by tests / tools.
    static QString findOutputHint();  // best guess at the VB-Cable input device

signals:
    void soundLoaded(int key, bool ok, const QString &error);
    void deviceError(const QString &message);

private:
    void restartSink(const QAudioDevice &dev);
    void restartSource(const QAudioDevice &dev);

    Mixer m_mixer;
    std::unique_ptr<QAudioSink> m_sink;
    std::unique_ptr<QAudioSource> m_source;
    QIODevice *m_sourceIo = nullptr;
    QHash<int, Mixer::Pcm> m_cache;
    bool m_micEnabled = true;
    QAudioDevice m_inputDevice;
};
