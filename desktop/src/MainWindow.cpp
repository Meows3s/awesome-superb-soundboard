#include "MainWindow.h"

#include <QCheckBox>
#include <QColorDialog>
#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMediaDevices>
#include <QPushButton>
#include <QSlider>
#include <QStatusBar>
#include <QVBoxLayout>

namespace {
// Pick black or white text depending on the key colour's luminance.
QString keyStyle(const QColor &c, bool bound) {
    const bool dark = c.lightness() < 110;
    return QString("QPushButton { background:%1; color:%2; border:2px solid %3; "
                   "border-radius:8px; font-weight:bold; padding:8px; }"
                   "QPushButton:hover { border-color:#ffffff; }")
        .arg(c.name(), dark ? "#ffffff" : "#000000", bound ? "#222222" : "#777777");
}
}  // namespace

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent), m_link(this), m_audio(this) {
    m_cfg.load();
    buildUi();

    connect(&m_link, &SerialLink::stateChanged, this, &MainWindow::onLinkState);
    connect(&m_link, &SerialLink::eventReceived, this, &MainWindow::onEvent);
    connect(&m_audio, &AudioEngine::deviceError, this,
            [this](const QString &m) { statusBar()->showMessage(m, 6000); });
    connect(&m_audio, &AudioEngine::soundLoaded, this, [this](int key, bool ok, const QString &err) {
        if (!ok)
            statusBar()->showMessage(tr("Key %1: could not load sound (%2)").arg(key + 1).arg(err), 8000);
    });

    populateDevices();
    m_audio.setMicEnabled(m_cfg.micEnabled);
    m_audio.setVolume(m_cfg.volume);
    for (int i = 0; i < Protocol::kKeyCount; ++i) {
        m_audio.loadSound(i, m_cfg.keys[i].sound);
        refreshKeyButton(i);
    }
    setVolume(m_cfg.volume, false);
    onLinkState(SerialLink::State::Disconnected);
    m_link.start();
}

void MainWindow::buildUi() {
    setWindowTitle("awesome-superb-soundboard");
    auto *central = new QWidget(this);
    auto *root = new QVBoxLayout(central);

    m_status = new QLabel(central);
    m_status->setAlignment(Qt::AlignCenter);
    m_status->setMargin(6);
    root->addWidget(m_status);

    // 3 columns x 4 rows, matching the physical keypad (key 1 = top-left).
    auto *grid = new QGridLayout;
    for (int i = 0; i < Protocol::kKeyCount; ++i) {
        auto *b = new QPushButton(central);
        b->setMinimumSize(130, 70);
        b->setToolTip(tr("Click to assign a sound / colour. Right-click to test-play."));
        b->setContextMenuPolicy(Qt::CustomContextMenu);
        connect(b, &QPushButton::clicked, this, [this, i] { configureKey(i); });
        connect(b, &QPushButton::customContextMenuRequested, this, [this, i] { m_audio.play(i); });
        grid->addWidget(b, i / 3, i % 3);
        m_keyButtons[i] = b;
    }
    root->addLayout(grid);

    auto *vrow = new QHBoxLayout;
    vrow->addWidget(new QLabel(tr("Effect volume"), central));
    m_volume = new QSlider(Qt::Horizontal, central);
    m_volume->setRange(0, 100);
    connect(m_volume, &QSlider::valueChanged, this, &MainWindow::onVolumeSlider);
    vrow->addWidget(m_volume, 1);
    m_volumeLabel = new QLabel(central);
    m_volumeLabel->setMinimumWidth(40);
    vrow->addWidget(m_volumeLabel);
    root->addLayout(vrow);

    auto *audio = new QGroupBox(tr("Audio routing"), central);
    auto *form = new QFormLayout(audio);
    m_outputBox = new QComboBox(audio);
    m_inputBox = new QComboBox(audio);
    m_micCheck = new QCheckBox(tr("Mix microphone into output"), audio);
    m_micCheck->setChecked(m_cfg.micEnabled);
    form->addRow(tr("Output (VB-Cable \"CABLE Input\")"), m_outputBox);
    form->addRow(tr("Microphone"), m_inputBox);
    form->addRow(m_micCheck);
    root->addWidget(audio);

    connect(m_outputBox, &QComboBox::activated, this, [this] {
        m_cfg.outputDevice = m_outputBox->currentData().toString();
        m_audio.setOutputDevice(m_cfg.outputDevice);
        saveConfig();
    });
    connect(m_inputBox, &QComboBox::activated, this, [this] {
        m_cfg.inputDevice = m_inputBox->currentData().toString();
        m_audio.setInputDevice(m_cfg.inputDevice);
        saveConfig();
    });
    connect(m_micCheck, &QCheckBox::toggled, this, [this](bool on) {
        m_cfg.micEnabled = on;
        m_audio.setMicEnabled(on);
        saveConfig();
    });

    setCentralWidget(central);
}

void MainWindow::populateDevices() {
    m_outputBox->clear();
    m_outputBox->addItem(tr("System default"), QString());
    for (const QAudioDevice &d : QMediaDevices::audioOutputs())
        m_outputBox->addItem(d.description(), d.description());
    m_inputBox->clear();
    m_inputBox->addItem(tr("System default"), QString());
    for (const QAudioDevice &d : QMediaDevices::audioInputs())
        m_inputBox->addItem(d.description(), d.description());

    // First run: prefer VB-Cable automatically if it is installed.
    if (m_cfg.outputDevice.isEmpty()) m_cfg.outputDevice = AudioEngine::findOutputHint();
    int idx = m_outputBox->findData(m_cfg.outputDevice);
    m_outputBox->setCurrentIndex(idx < 0 ? 0 : idx);
    idx = m_inputBox->findData(m_cfg.inputDevice);
    m_inputBox->setCurrentIndex(idx < 0 ? 0 : idx);

    m_audio.setOutputDevice(m_cfg.outputDevice);
    m_audio.setInputDevice(m_cfg.inputDevice);
    if (m_cfg.outputDevice.isEmpty())
        statusBar()->showMessage(tr("VB-Cable not found: playing to the default output. "
                                    "Install VB-Cable to use this as a Discord microphone."), 10000);
}

void MainWindow::refreshKeyButton(int key) {
    const KeyConfig &k = m_cfg.keys[key];
    QPushButton *b = m_keyButtons[key];
    const QString label = m_cfg.displayLabel(key);
    b->setText(QString("%1\n%2").arg(key + 1).arg(label.isEmpty() ? tr("(unassigned)") : label));
    b->setStyleSheet(keyStyle(k.color, !k.sound.isEmpty()));
}

void MainWindow::configureKey(int key) {
    KeyConfig &k = m_cfg.keys[key];
    QDialog dlg(this);
    dlg.setWindowTitle(tr("Key %1").arg(key + 1));
    auto *form = new QFormLayout(&dlg);

    auto *soundEdit = new QLineEdit(k.sound, &dlg);
    auto *browse = new QPushButton(tr("Browse..."), &dlg);
    auto *srow = new QHBoxLayout;
    srow->addWidget(soundEdit, 1);
    srow->addWidget(browse);
    form->addRow(tr("Sound file"), srow);
    connect(browse, &QPushButton::clicked, &dlg, [&] {
        const QString f = QFileDialog::getOpenFileName(
            &dlg, tr("Choose sound"), soundEdit->text(),
            tr("Audio (*.wav *.mp3 *.ogg *.flac *.m4a);;All files (*)"));
        if (!f.isEmpty()) soundEdit->setText(f);
    });

    auto *labelEdit = new QLineEdit(k.label, &dlg);
    labelEdit->setMaxLength(Protocol::kMaxLabelLen);
    labelEdit->setPlaceholderText(tr("(file name)"));
    form->addRow(tr("Label (max %1 chars)").arg(Protocol::kMaxLabelLen), labelEdit);

    QColor color = k.color;
    auto *colorBtn = new QPushButton(&dlg);
    auto paint = [&] { colorBtn->setStyleSheet(keyStyle(color, true)); colorBtn->setText(color.name()); };
    paint();
    connect(colorBtn, &QPushButton::clicked, &dlg, [&] {
        const QColor c = QColorDialog::getColor(color, &dlg, tr("Key colour"));
        if (c.isValid()) { color = c; paint(); }
    });
    form->addRow(tr("Key colour"), colorBtn);

    auto *bb = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    connect(bb, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(bb, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    form->addRow(bb);

    if (dlg.exec() != QDialog::Accepted) return;
    const bool soundChanged = k.sound != soundEdit->text();
    k.sound = soundEdit->text();
    k.label = labelEdit->text();
    k.color = color;
    if (soundChanged) m_audio.loadSound(key, k.sound);
    refreshKeyButton(key);
    pushKey(key);
    saveConfig();
}

void MainWindow::pushKey(int key) {
    m_link.send(Protocol::setColor(key, m_cfg.keys[key].color));
    m_link.send(Protocol::setLabel(key, m_cfg.displayLabel(key)));
}

void MainWindow::pushStateToBoard() {
    for (int i = 0; i < Protocol::kKeyCount; ++i) pushKey(i);
    m_link.send(Protocol::setVolume(m_cfg.volume));
}

void MainWindow::onLinkState(SerialLink::State s) {
    switch (s) {
    case SerialLink::State::Connected:
        m_status->setText(tr("Macropad connected (%1)").arg(m_link.portName()));
        m_status->setStyleSheet("background:#1b5e20; color:white; font-weight:bold;");
        pushStateToBoard();  // board forgets everything on power-up
        break;
    case SerialLink::State::NoReply:
        m_status->setText(tr("Port %1 open but the board is not answering (old firmware or another program on the port?)")
                              .arg(m_link.portName()));
        m_status->setStyleSheet("background:#e65100; color:white; font-weight:bold;");
        break;
    case SerialLink::State::Disconnected:
        m_status->setText(m_link.lastError().isEmpty()
                              ? tr("Macropad not connected")
                              : tr("Macropad found but cannot open port: %1").arg(m_link.lastError()));
        m_status->setStyleSheet("background:#b71c1c; color:white; font-weight:bold;");
        break;
    }
}

void MainWindow::onEvent(const Protocol::Event &ev) {
    using T = Protocol::Event::Type;
    switch (ev.type) {
    case T::Key:
        if (ev.down) m_audio.play(ev.key);
        m_keyButtons[ev.key]->setDown(ev.down);
        break;
    case T::Encoder:
        setVolume(m_cfg.volume + ev.delta * 2, true);  // 20 detents = 40%/turn
        break;
    case T::Switch:
        if (ev.down) m_audio.stopAll();  // knob push = stop all sounds
        break;
    default:
        break;
    }
}

void MainWindow::onVolumeSlider(int v) { setVolume(v, false); }

void MainWindow::setVolume(int percent, bool fromBoard) {
    percent = qBound(0, percent, 100);
    const bool changed = percent != m_cfg.volume;
    m_cfg.volume = percent;
    m_audio.setVolume(percent);
    m_volumeLabel->setText(QString("%1%").arg(percent));
    {
        QSignalBlocker block(m_volume);
        m_volume->setValue(percent);
    }
    m_link.send(Protocol::setVolume(percent));
    if (changed) saveConfig();
    Q_UNUSED(fromBoard);
}

void MainWindow::saveConfig() { m_cfg.save(); }
