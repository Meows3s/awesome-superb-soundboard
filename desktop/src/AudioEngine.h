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
    // Master effect volume: applies to both the cable mix and the monitor.
    void setVolume(int percent) { m_mixer.setVolume(percent); m_monitor.setVolume(percent); }

    // Monitor = effects only, to your own speakers/headphones (no mic).
    // Empty description = system default output.
    void setMonitor(bool enabled, const QString &description);
    // True if the monitor would land on the same device as the cable output
    // (effects would then be doubled in the call).
    bool monitorConflictsWithOutput() const;

    // Decode `path` into the cache slot for `key` (async). Empty path clears.
    void loadSound(int key, const QString &path);
    void play(int key);
    void stopAll() { m_mixer.stopAll(); m_monitor.stopAll(); }
    Mixer &mixer() { return m_mixer; }

    // Decode a whole file synchronously-ish; used by tests / tools.
    static QString findOutputHint();  // best guess at the VB-Cable input device

signals:
    void soundLoaded(int key, bool ok, const QString &error);
    void deviceError(const QString &message);

private:
    void restartSink(const QAudioDevice &dev);
    void restartSource(const QAudioDevice &dev);
    void restartMonitor();

    Mixer m_mixer;
    Mixer m_monitor;
    std::unique_ptr<QAudioSink> m_monitorSink;
    QAudioDevice m_outputDevice;
    QAudioDevice m_monitorDevice;
    bool m_monitorEnabled = false;
    std::unique_ptr<QAudioSink> m_sink;
    std::unique_ptr<QAudioSource> m_source;
    QIODevice *m_sourceIo = nullptr;
    QHash<int, Mixer::Pcm> m_cache;
    bool m_micEnabled = true;
    QAudioDevice m_inputDevice;
};
