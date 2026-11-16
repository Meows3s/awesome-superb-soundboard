#include <QtTest>
#include "Mixer.h"

static Mixer::Pcm constant(qint16 v, int frames) {
    auto b = std::make_shared<QByteArray>(frames * Mixer::kFrameBytes, Qt::Uninitialized);
    qint16 *p = reinterpret_cast<qint16 *>(b->data());
    for (int i = 0; i < frames * Mixer::kChannels; ++i) p[i] = v;
    return b;
}

class TestMixer : public QObject {
    Q_OBJECT
private slots:
    void silenceWhenIdle() {
        Mixer m;
        qint16 out[16];
        std::fill(out, out + 16, qint16(7));
        m.mix(out, 8);
        for (qint16 s : out) QCOMPARE(s, qint16(0));
    }
    void volumeScalesSamples() {
        Mixer m;
        m.setVolume(100);
        m.play(0, constant(1000, 4));
        qint16 out[8];
        m.mix(out, 4);
        QCOMPARE(out[0], qint16(1000));
        m.play(0, constant(1000, 4));
        m.setVolume(50);  // quadratic taper -> 0.25
        m.mix(out, 4);
        QCOMPARE(out[0], qint16(250));
        m.play(0, constant(1000, 4));
        m.setVolume(0);
        m.mix(out, 4);
        QCOMPARE(out[0], qint16(0));
    }
    void voicesFinishAndRestart() {
        Mixer m;
        m.setVolume(100);
        m.play(3, constant(100, 2));
        QVERIFY(m.isPlaying(3));
        qint16 out[16];
        m.mix(out, 8);  // longer than the sample: tail must be zero
        QCOMPARE(out[0], qint16(100));
        QCOMPARE(out[4], qint16(0));
        QVERIFY(!m.isPlaying(3));
    }
    void sumsAndClips() {
        Mixer m;
        m.setVolume(100);
        m.play(0, constant(30000, 2));
        m.play(1, constant(30000, 2));
        qint16 out[4];
        m.mix(out, 2);
        QCOMPARE(out[0], qint16(32767));
    }
    void micPassThroughAndStop() {
        Mixer m;
        m.setVolume(100);
        QByteArray mic(4 * Mixer::kFrameBytes, 0);
        qint16 *p = reinterpret_cast<qint16 *>(mic.data());
        for (int i = 0; i < 8; ++i) p[i] = 500;
        m.feedMic(mic);
        m.play(0, constant(100, 4));
        qint16 out[8];
        m.mix(out, 4);
        QCOMPARE(out[0], qint16(600));
        m.play(0, constant(100, 100));
        m.stopAll();
        m.mix(out, 4);
        QCOMPARE(out[0], qint16(0));
    }
    void independentMixersShareSamples() {
        // Monitor design: the same PCM buffer is played by two mixers with
        // independent positions; mic only reaches the cable mixer.
        Mixer cable, monitor;
        monitor.setMicEnabled(false);
        cable.setVolume(100);
        monitor.setVolume(100);
        auto pcm = constant(1000, 8);
        QByteArray mic(4 * Mixer::kFrameBytes, 0);
        qint16 *mp = reinterpret_cast<qint16 *>(mic.data());
        for (int i = 0; i < 8; ++i) mp[i] = 500;
        cable.feedMic(mic);
        monitor.feedMic(mic);
        cable.play(0, pcm);
        monitor.play(0, pcm);
        qint16 a[8], b[8];
        cable.mix(a, 4);
        monitor.mix(b, 4);
        QCOMPARE(a[0], qint16(1500));  // effect + mic
        QCOMPARE(b[0], qint16(1000));  // effect only
        monitor.setVolume(0);
        monitor.mix(b, 4);  // second half of the sample, now muted
        QCOMPARE(b[0], qint16(0));
        cable.mix(a, 4);
        QCOMPARE(a[0], qint16(1000));  // cable unaffected
    }
    void micDisabledDropsInput() {
        Mixer m;
        m.setMicEnabled(false);
        m.feedMic(QByteArray(64, 'x'));
        qint16 out[4];
        m.mix(out, 2);
        QCOMPARE(out[0], qint16(0));
    }
};
QTEST_APPLESS_MAIN(TestMixer)
#include "test_mixer.moc"
