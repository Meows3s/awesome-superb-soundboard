#pragma once
#include <QByteArray>
#include <QIODevice>
#include <QMutex>
#include <QVector>
#include <memory>

// Pull-mode mixer device that feeds a QAudioSink.
//
// Fixed internal format: 48 kHz, 2 channels, signed 16-bit (4 bytes/frame).
// Output = microphone (pass-through) + active sample voices * effect volume.
// readData() always returns the full requested length (silence when idle) so
// the sink never starves or stops pulling.
class Mixer : public QIODevice {
    Q_OBJECT
public:
    static constexpr int kSampleRate = 48000;
    static constexpr int kChannels = 2;
    static constexpr int kFrameBytes = kChannels * 2;

    using Pcm = std::shared_ptr<const QByteArray>;  // kSampleRate/s16/stereo

    explicit Mixer(QObject *parent = nullptr);

    void play(int key, Pcm pcm);        // restarts the sound if already playing
    void stop(int key);
    void stopAll();
    bool isPlaying(int key) const;
    void setVolume(int percent);        // effect volume 0..100
    void setMicEnabled(bool on);
    void feedMic(const QByteArray &pcm);  // raw mic PCM in the same format

    bool isSequential() const override { return true; }
    qint64 bytesAvailable() const override { return (1 << 20) + QIODevice::bytesAvailable(); }

    // Mix `frames` frames into out (testable without an audio device).
    void mix(qint16 *out, int frames);

protected:
    qint64 readData(char *data, qint64 maxlen) override;
    qint64 writeData(const char *, qint64) override { return -1; }

private:
    struct Voice {
        int key;
        Pcm pcm;
        qint64 pos;  // byte offset
    };
    mutable QMutex m_mutex;
    QVector<Voice> m_voices;
    QByteArray m_mic;  // FIFO of mic bytes, bounded to ~100 ms
    int m_volume = 60;
    bool m_micEnabled = true;
};
