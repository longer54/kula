# Kula

A fast, local photo and video browser for Linux. Rust + Qt 6 / QML. Kula indexes a folder of photos into a local SQLite database, builds thumbnails, and shows everything on a smooth date timeline. It never modifies your original files.

## Features

- Continuous date timeline with a draggable scrubber, adjustable grid size
- Dates from EXIF, file names (e.g. `IMG_20240131_...`, WhatsApp) and optional Google Takeout JSON metadata
- Viewer with zoom, photo info panel and video playback
- Favorites, archive, albums, "No date" and "Recently added" views
- Safe delete: files go to the system trash, with restore and permanent delete
- Multi-select (click, Shift+click range, Ctrl+A), context menus
- Light / dark / system theme
- Interface in English and Polish (automatic by system locale, switchable in Settings)
- Optional AI module (content search, people) — coming soon. Kula works fully without it; the related controls stay hidden.

## Requirements

- Linux with Qt 6 (Quick, Quick Controls 2, Multimedia, Svg, Widgets) and its development headers
- Rust (stable) and a C++ compiler
- Optional: `xdg-user-dir` (to find your Pictures folder)

## Build

```sh
cargo build --release
./target/release/kula
```

A desktop entry is in `packaging/kula.desktop` and icons in `icons/kula-*.png`.

## Photo folder

By default Kula scans your XDG Pictures folder (`~/Pictures`). Change it in **Settings → Photo folder** (the library is rescanned after saving), or set `KULA_SRC=/path/to/photos`. Optional Google Takeout metadata folder: Settings, or `KULA_TAKEOUT`. Data lives in `$XDG_DATA_HOME/kula`, thumbnails in `$XDG_CACHE_HOME/kula`.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `/` | Search |
| `←` `→` | Previous / next photo (viewer) |
| `z`, `+`, `−`, `0` | Zoom |
| `i` | Info panel |
| `Space` | Play / pause video |
| `x` | Select item under cursor |
| `Shift`+click | Select range |
| `Ctrl+A` | Select all |
| `f` | Toggle favorite |
| `Shift+A` | Archive |
| `#` or `Delete` | Move to trash |
| `Ctrl`+wheel | Grid size |
| `Ctrl+B` | Toggle side panel |
| `F11` | Full screen |
| `Esc` | Back / deselect |

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
