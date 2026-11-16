#include "Mixer.h"

#include <algorithm>
#include <cstring>

namespace {
constexpr int kMaxMicBytes = Mixer::kSampleRate / 10 * Mixer::kFrameBytes;  // 100 ms
}

Mixer::Mixer(QObject *parent) : QIODevice(parent) { open(QIODevice::ReadOnly); }

void Mixer::play(int key, Pcm pcm) {
    if (!pcm || pcm->isEmpty()) return;
    QMutexLocker l(&m_mutex);
    for (Voice &v : m_voices) {
        if (v.key == key) {
            v.pcm = std::move(pcm);
            v.pos = 0;
            return;
        }
    }
    m_voices.push_back({key, std::move(pcm), 0});
}

void Mixer::stop(int key) {
    QMutexLocker l(&m_mutex);
    m_voices.erase(std::remove_if(m_voices.begin(), m_voices.end(),
                                  [key](const Voice &v) { return v.key == key; }),
                   m_voices.end());
}

void Mixer::stopAll() {
    QMutexLocker l(&m_mutex);
    m_voices.clear();
}

bool Mixer::isPlaying(int key) const {
    QMutexLocker l(&m_mutex);
    return std::any_of(m_voices.begin(), m_voices.end(), [key](const Voice &v) { return v.key == key; });
}

void Mixer::setVolume(int percent) {
    QMutexLocker l(&m_mutex);
    m_volume = std::clamp(percent, 0, 100);
}

void Mixer::setMicEnabled(bool on) {
    QMutexLocker l(&m_mutex);
    m_micEnabled = on;
    if (!on) m_mic.clear();
}

void Mixer::feedMic(const QByteArray &pcm) {
    QMutexLocker l(&m_mutex);
    if (!m_micEnabled) return;
    m_mic += pcm;
    if (m_mic.size() > kMaxMicBytes) m_mic.remove(0, m_mic.size() - kMaxMicBytes);
}

void Mixer::mix(qint16 *out, int frames) {
    const int samples = frames * kChannels;
    QVector<qint32> acc(samples, 0);
    QMutexLocker l(&m_mutex);

    // Microphone: unity gain, consumed from the FIFO.
    const int micBytes = std::min<int>(m_mic.size(), samples * 2);
    const qint16 *mic = reinterpret_cast<const qint16 *>(m_mic.constData());
    for (int i = 0; i < micBytes / 2; ++i) acc[i] += mic[i];
    m_mic.remove(0, micBytes);

    // Sample voices scaled by the effect volume (quadratic taper feels linear).
    const float gain = (m_volume / 100.0f) * (m_volume / 100.0f);
    for (Voice &v : m_voices) {
        const qint64 remaining = v.pcm->size() - v.pos;
        const int n = int(std::min<qint64>(remaining, qint64(samples) * 2) / 2);
        const qint16 *src = reinterpret_cast<const qint16 *>(v.pcm->constData() + v.pos);
        for (int i = 0; i < n; ++i) acc[i] += qint32(src[i] * gain);
        v.pos += qint64(n) * 2;
    }
    m_voices.erase(std::remove_if(m_voices.begin(), m_voices.end(),
                                  [](const Voice &v) { return v.pos >= v.pcm->size(); }),
                   m_voices.end());

    for (int i = 0; i < samples; ++i) out[i] = qint16(std::clamp<qint32>(acc[i], -32768, 32767));
}

qint64 Mixer::readData(char *data, qint64 maxlen) {
    const qint64 frames = maxlen / kFrameBytes;
    mix(reinterpret_cast<qint16 *>(data), int(frames));
    return frames * kFrameBytes;
}
