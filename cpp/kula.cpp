#include "kula.h"

#include <algorithm>
#include <string>
#include <vector>

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QImageReader>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

std::unique_ptr<QGuiApplication> kula_new_application(rust::Slice<const rust::String> args)
{
    static std::vector<std::string> storage;
    static std::vector<char *> argv;
    static int argc = 0;
    storage.clear();
    for (const auto &a : args)
        storage.emplace_back(std::string(a));
    argv.clear();
    for (auto &s : storage)
        argv.push_back(s.data());
    argv.push_back(nullptr);
    argc = static_cast<int>(storage.size());
    auto app = std::make_unique<QGuiApplication>(argc, argv.data());
    QIcon icon;
    for (int s : { 16, 32, 48, 64, 128, 256, 512 })
        icon.addFile(QStringLiteral(":/qt/qml/org/kula/icons/kula-%1.png").arg(s), QSize(s, s));
    app->setWindowIcon(icon);
    QImageReader::setAllocationLimit(0);
    return app;
}

static QString qs(rust::Str s) { return QString::fromUtf8(s.data(), static_cast<qsizetype>(s.size())); }

bool kula_make_thumb(rust::Str src, rust::Str dst, int32_t size)
{
    QImageReader r(qs(src));
    r.setAutoTransform(true);
    const QSize sz = r.size();
    if (sz.isValid() && (sz.width() > size || sz.height() > size)) {
        QSize t = sz;
        t.scale(size, size, Qt::KeepAspectRatio);
        r.setScaledSize(t);
    }
    QImage img = r.read();
    if (img.isNull())
        return false;
    if (img.width() > size || img.height() > size)
        img = img.scaled(size, size, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    if (img.hasAlphaChannel()) {
        QImage bg(img.size(), QImage::Format_RGB32);
        bg.fill(QColor(0x1b, 0x1d, 0x2d));
        QPainter p(&bg);
        p.drawImage(0, 0, img);
        p.end();
        img = bg;
    }
    return img.save(qs(dst), "JPG", 80);
}

bool kula_probe(rust::Str src, int32_t &w, int32_t &h)
{
    QImageReader r(qs(src));
    r.setAutoTransform(true);
    QSize sz = r.size();
    if (!sz.isValid())
        return false;
    if (r.transformation() & QImageIOHandler::TransformationRotate90)
        sz.transpose();
    w = sz.width();
    h = sz.height();
    return true;
}

bool kula_make_face(rust::Str src, rust::Str dst, double x, double y, double w, double h, int32_t size)
{
    QImageReader r(qs(src));
    r.setAutoTransform(true);
    QSize sz = r.size();
    if (sz.isValid() && (sz.width() > 2400 || sz.height() > 2400)) {
        QSize t = sz;
        t.scale(2400, 2400, Qt::KeepAspectRatio);
        r.setScaledSize(t);
    }
    QImage img = r.read();
    if (img.isNull())
        return false;
    const double W = img.width(), H = img.height();
    double cx = (x + w / 2) * W, cy = (y + h / 2) * H;
    double side = std::max(w * W, h * H) * 1.7;
    side = std::min(side, std::min(W, H));
    double sx = std::clamp(cx - side / 2, 0.0, W - side), sy = std::clamp(cy - side / 2, 0.0, H - side);
    QImage crop = img.copy(QRect(int(sx), int(sy), int(side), int(side))).scaled(size, size, Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    QImage out(size, size, QImage::Format_ARGB32_Premultiplied);
    out.fill(Qt::transparent);
    QPainter p(&out);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setRenderHint(QPainter::SmoothPixmapTransform, true);
    QPainterPath path;
    path.addEllipse(0, 0, size, size);
    p.setClipPath(path);
    p.drawImage(0, 0, crop);
    p.end();
    return out.save(qs(dst), "PNG");
}
