#include "AudioEngine.h"

#include <QAudioDecoder>
#include <QMediaDevices>
#include <QUrl>

QAudioFormat AudioEngine::format() {
    QAudioFormat f;
    f.setSampleRate(Mixer::kSampleRate);
    f.setChannelCount(Mixer::kChannels);
    f.setSampleFormat(QAudioFormat::Int16);
    return f;
}

AudioEngine::AudioEngine(QObject *parent) : QObject(parent), m_mixer(this), m_monitor(this) {
    m_monitor.setMicEnabled(false);
    restartSink(QMediaDevices::defaultAudioOutput());
    restartSource(QMediaDevices::defaultAudioInput());
}

AudioEngine::~AudioEngine() {
    if (m_sink) m_sink->stop();
    if (m_monitorSink) m_monitorSink->stop();
    if (m_source) m_source->stop();
}

QString AudioEngine::findOutputHint() {
    // VB-Cable's playback side is called "CABLE Input (VB-Audio Virtual Cable)".
    for (const QAudioDevice &d : QMediaDevices::audioOutputs())
        if (d.description().contains("CABLE Input", Qt::CaseInsensitive)) return d.description();
    return {};
}

bool AudioEngine::setOutputDevice(const QString &description) {
    QAudioDevice dev = QMediaDevices::defaultAudioOutput();
    if (!description.isEmpty()) {
        bool found = false;
        for (const QAudioDevice &d : QMediaDevices::audioOutputs())
            if (d.description() == description) { dev = d; found = true; break; }
        if (!found) {
            emit deviceError(tr("Output device \"%1\" not found, using default").arg(description));
        }
    }
    restartSink(dev);
    restartMonitor();
    return true;
}

bool AudioEngine::setInputDevice(const QString &description) {
    QAudioDevice dev = QMediaDevices::defaultAudioInput();
    if (!description.isEmpty()) {
        for (const QAudioDevice &d : QMediaDevices::audioInputs())
            if (d.description() == description) { dev = d; break; }
    }
    restartSource(dev);
    return true;
}

void AudioEngine::setMicEnabled(bool on) {
    m_micEnabled = on;
    m_mixer.setMicEnabled(on);
    restartSource(m_inputDevice);
}

void AudioEngine::restartSink(const QAudioDevice &dev) {
    m_outputDevice = dev;
    if (m_sink) m_sink->stop();
    if (dev.isNull()) {
        m_sink.reset();
        emit deviceError(tr("No audio output device available"));
        return;
    }
    m_sink = std::make_unique<QAudioSink>(dev, format(), this);
    m_sink->setBufferSize(Mixer::kSampleRate / 20 * Mixer::kFrameBytes);  // ~50 ms
    m_sink->start(&m_mixer);
}

bool AudioEngine::monitorConflictsWithOutput() const {
    return m_monitorEnabled && !m_outputDevice.isNull() && m_monitorDevice.id() == m_outputDevice.id();
}

void AudioEngine::setMonitor(bool enabled, const QString &description) {
    m_monitorEnabled = enabled;
    m_monitorDevice = QMediaDevices::defaultAudioOutput();
    if (!description.isEmpty()) {
        for (const QAudioDevice &d : QMediaDevices::audioOutputs())
            if (d.description() == description) { m_monitorDevice = d; break; }
    }
    restartMonitor();
}

void AudioEngine::restartMonitor() {
    if (m_monitorSink) m_monitorSink->stop();
    m_monitorSink.reset();
    m_monitor.stopAll();
    // Never open the monitor on the cable device: it would double the effects
    // in the call. The UI shows a warning for this case.
    if (!m_monitorEnabled || m_monitorDevice.isNull() || monitorConflictsWithOutput()) return;
    m_monitorSink = std::make_unique<QAudioSink>(m_monitorDevice, format(), this);
    m_monitorSink->setBufferSize(Mixer::kSampleRate / 20 * Mixer::kFrameBytes);
    m_monitorSink->start(&m_monitor);
}

void AudioEngine::restartSource(const QAudioDevice &dev) {
    if (m_source) m_source->stop();
    m_source.reset();
    m_sourceIo = nullptr;
    m_inputDevice = dev;
    if (!m_micEnabled) return;
    if (dev.isNull()) {
        emit deviceError(tr("No microphone available"));
        return;
    }
    m_source = std::make_unique<QAudioSource>(dev, format(), this);
    m_sourceIo = m_source->start();
    if (m_sourceIo)
        connect(m_sourceIo, &QIODevice::readyRead, this,
                [this] { m_mixer.feedMic(m_sourceIo->readAll()); });
}

void AudioEngine::loadSound(int key, const QString &path) {
    m_cache.remove(key);
    if (path.isEmpty()) return;

    auto *dec = new QAudioDecoder(this);
    dec->setAudioFormat(format());
    dec->setSource(QUrl::fromLocalFile(path));
    auto pcm = std::make_shared<QByteArray>();
    connect(dec, &QAudioDecoder::bufferReady, this, [dec, pcm] {
        const QAudioBuffer b = dec->read();
        pcm->append(b.constData<char>(), b.byteCount());
    });
    connect(dec, &QAudioDecoder::finished, this, [this, dec, pcm, key] {
        const bool ok = !pcm->isEmpty();
        if (ok) m_cache.insert(key, pcm);
        emit soundLoaded(key, ok, ok ? QString() : tr("file decoded to no audio"));
        dec->deleteLater();
    });
    connect(dec, qOverload<QAudioDecoder::Error>(&QAudioDecoder::error), this,
            [this, dec, key](QAudioDecoder::Error) {
                emit soundLoaded(key, false, dec->errorString());
                dec->deleteLater();
            });
    dec->start();
}

void AudioEngine::play(int key) {
    const auto it = m_cache.constFind(key);
    if (it == m_cache.constEnd()) return;
    m_mixer.play(key, *it);
    if (m_monitorSink) m_monitor.play(key, *it);
}
