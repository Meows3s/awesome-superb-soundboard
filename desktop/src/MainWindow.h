#pragma once
#include <QMainWindow>
#include <array>

#include "AudioEngine.h"
#include "Config.h"
#include "Protocol.h"
#include "SerialLink.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QPushButton;
class QSlider;

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);

private slots:
    void onLinkState(SerialLink::State s);
    void onEvent(const Protocol::Event &ev);
    void onVolumeSlider(int v);

private:
    void buildUi();
    void configureKey(int key);
    void refreshKeyButton(int key);
    void pushStateToBoard();
    void pushKey(int key);
    void setVolume(int percent, bool fromBoard);
    void saveConfig();
    void populateDevices();

    AppConfig m_cfg;
    SerialLink m_link;
    AudioEngine m_audio;

    std::array<QPushButton *, Protocol::kKeyCount> m_keyButtons{};
    QLabel *m_status = nullptr;
    QSlider *m_volume = nullptr;
    QLabel *m_volumeLabel = nullptr;
    QComboBox *m_outputBox = nullptr;
    QComboBox *m_inputBox = nullptr;
    QCheckBox *m_micCheck = nullptr;
};
