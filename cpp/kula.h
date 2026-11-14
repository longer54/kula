#pragma once
#include <cstdint>
#include <memory>
#include <QtGui/QGuiApplication>
#include "rust/cxx.h"

std::unique_ptr<QGuiApplication> kula_new_application(rust::Slice<const rust::String> args);
// thumbnail (longer side = size, JPEG q80, respecting EXIF orientation); false = cannot be decoded
bool kula_make_thumb(rust::Str src, rust::Str dst, int32_t size);
// dimensions after rotation per EXIF; false = unknown format
bool kula_probe(rust::Str src, int32_t &w, int32_t &h);
// circular face crop (relative bbox x,y,w,h; with margin), PNG with alpha, side = size
bool kula_make_face(rust::Str src, rust::Str dst, double x, double y, double w, double h, int32_t size);
