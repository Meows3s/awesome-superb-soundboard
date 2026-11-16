#include <QApplication>
#include <QPalette>
#include <QStyleFactory>
#include "MainWindow.h"

// Dark theme on every platform: Fusion style + explicit dark palette
// (the native Windows style ignores palettes).
static void applyDarkTheme(QApplication &app) {
    app.setStyle(QStyleFactory::create("Fusion"));
    QPalette p;
    const QColor base(30, 30, 30), window(45, 45, 48), text(230, 230, 230), accent(42, 130, 218);
    p.setColor(QPalette::Window, window);
    p.setColor(QPalette::WindowText, text);
    p.setColor(QPalette::Base, base);
    p.setColor(QPalette::AlternateBase, window);
    p.setColor(QPalette::ToolTipBase, base);
    p.setColor(QPalette::ToolTipText, text);
    p.setColor(QPalette::Text, text);
    p.setColor(QPalette::Button, window);
    p.setColor(QPalette::ButtonText, text);
    p.setColor(QPalette::BrightText, Qt::red);
    p.setColor(QPalette::Link, accent);
    p.setColor(QPalette::Highlight, accent);
    p.setColor(QPalette::HighlightedText, Qt::white);
    p.setColor(QPalette::PlaceholderText, QColor(140, 140, 140));
    const QColor dis(120, 120, 120);
    p.setColor(QPalette::Disabled, QPalette::WindowText, dis);
    p.setColor(QPalette::Disabled, QPalette::Text, dis);
    p.setColor(QPalette::Disabled, QPalette::ButtonText, dis);
    app.setPalette(p);
    app.setStyleSheet("QToolTip { color:#e6e6e6; background:#1e1e1e; border:1px solid #555; }");
}

int main(int argc, char **argv) {
    QApplication app(argc, argv);
    applyDarkTheme(app);
    MainWindow w;
    w.show();
    return app.exec();
}
