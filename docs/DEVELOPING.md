# Working on PrintCraft

## Layout

PrintCraft is one Rust crate with two programs:

| Program | Source | What it is |
|---|---|---|
| `printcraft` | `src/main.rs` | The app (egui / eframe). Also the browser version. |
| `printcraft-web` | `src/bin/printcraft-web.rs` | The Google Images window (desktop only; see below). |

Main parts of `src/`:

| File | Contents |
|---|---|
| `app.rs` | The app state, tabs, undo, and every action (`Act`) |
| `chrome.rs`, `modern.rs` | Menus, toolbars, tool panel (Classic layout) and the Modern layout |
| `canvas.rs` | The page: drawing, selecting, moving, rulers and guidelines |
| `dialogs.rs`, `windows.rs` | Pop-up windows (pictures, headlines, photo editor, print, settings...) |
| `model.rs`, `kinds.rs` | The design (`.pcraft` is this as JSON) and the project kinds and paper sizes |
| `raster.rs`, `effects.rs`, `export.rs` | Drawing a design to pixels, effects, PDF / picture output and printing |
| `fonts.rs`, `online.rs`, `ai.rs` | Fonts, online fonts and pictures, optional AI pictures |
| `photo.rs`, `photo_editor.rs` | Photo changes (crop, color, effects, cut-outs) and the Photo Editor window |
| `update.rs` | Update checker: finds the release file matching this install, downloads, checks the SHA-256, installs |
| `templates.rs` | Ready-made designs (built in code; see `--design-sheet` below) |
| `calendar.rs` | Calendar maker: dates, US holidays, month grids |
| `merge.rs` | Address merge fields («Name», «Address»…) filled in per person when printing |
| `report.rs` | Report a Problem or Suggest an Idea: fills in a GitHub issue form (field ids match `.github/ISSUE_TEMPLATE`) |
| `widgets.rs` | Shared controls; `widgets::slider` / `widgets::number` add mouse-wheel and arrow-key control |
| `legacy/` | Readers for older project files (`.sig` and siblings) and `.CDT` art packs |
| `platform.rs` | Everything that differs between the desktop and browser versions |

Numbers and rules (paper sheets, effects, fold layouts) are in
[DESIGN.md](DESIGN.md); the older file formats in [FORMATS.md](FORMATS.md);
plans in [ROADMAP.md](ROADMAP.md).

## Building

```sh
cargo build --release
cargo test
```

Linux needs the development packages for the window and the web engine:

- Fedora: `sudo dnf install gtk3-devel webkit2gtk4.1-devel libxkbcommon-devel`
- Debian / Ubuntu: `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libxkbcommon-dev`

### Browser version

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve web/index.html            # http://127.0.0.1:8080, rebuilds on change
trunk build --release web/index.html  # static site in web/dist
```

In the browser, opening a file uses an upload button and saving downloads
the file. Settings are kept in the browser. Printing downloads a PDF to
print from. These need the desktop app: content packs, My Art, the Google
Images window, online fonts, AI pictures and choosing a printer.
`src/platform.rs` holds both versions of each of these.

### Windows from Linux or macOS

`scripts/build-windows.sh` makes a portable Windows x86_64 zip with
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) (Zig 0.13 on
the `PATH` or in `.tools/`, plus `rustup target add x86_64-pc-windows-gnu`).
Without a Windows resource compiler the `.exe` has no icon; the release
builds on GitHub have one.

### Releases

Pushing a tag like `v0.1.0` runs `.github/workflows/release.yml`, which builds
Linux (x86_64, arm64: `.deb`, `.rpm`, `.flatpak`, `.AppImage`, `.tar.gz`), Windows
(x86_64: `.msi`, setup `.exe`, portable `.zip`; arm64: setup `.exe`, portable
`.zip`), macOS (one universal `.dmg` and `.app.zip`) and the web version, and
attaches them to a GitHub release. `.github/workflows/pages.yml` publishes the
web version to GitHub Pages on the same tag. Keep `version` in `Cargo.toml`
in step with the tag.

Flatpak locally (uses the GNOME runtime, which includes WebKitGTK):

```sh
flatpak-builder --user --install-deps-from=flathub --install build-flatpak packaging/flatpak/org.printcraft.PrintCraft.yml
flatpak run org.printcraft.PrintCraft
```

## The Google Images window

Google only answers real web browsers (a direct request gets "turn on
JavaScript"), so the search runs in `printcraft-web`, which uses the
computer's own web engine through [tao](https://github.com/tauri-apps/tao)
and [wry](https://github.com/tauri-apps/wry): WebKitGTK on Linux, WebView2 on
Windows, WebKit on macOS. It adds a "+ Add to my project" button over
pictures and writes each pick as a line of JSON on standard output;
PrintCraft reads it and adds the picture (`data:` addresses are decoded, web
addresses downloaded). Cookies (consent, robot check) are kept in
PrintCraft's data folder under `web/`.

The main program doesn't link the web engine. If the companion is missing or
can't start, the search opens in the browser instead and PrintCraft watches
the clipboard for a copied picture. Ship `printcraft-web` beside
`printcraft` (every release package does). Self-test:
`PRINTCRAFT_WEB_SELFTEST=1 printcraft-web <address>`.

## Checking the look

`printcraft --screenshot out.png` opens a sample design, saves a picture of
the window and quits. Switches (environment variables):

| Variable | Values |
|---|---|
| `PRINTCRAFT_SAMPLE` | `card` (a 3-page card), `calendar` (a filled month), `lines` (every line style and flourish); otherwise a poster |
| `PRINTCRAFT_LAYOUT` | `modern` |
| `PRINTCRAFT_PAGES` | `legacy` (one page at a time) or `tiled` (side by side) |
| `PRINTCRAFT_LOOK` | `dark` |
| `PRINTCRAFT_OPEN` | `google` (the Pictures window), `new` / `new-calendar` (New Project), `pagesetup`, `report`, `update` (a real check), or `photo:<tool>[:<cut-out tool>]` (the Photo Editor on a made-up photo; tools 0 crop … 5 cut out; cut-out tool 0 starts with a heart shape, 2 with a magic-wand pick) |

`printcraft --design-sheet out.png` draws every ready-made design on one
picture, for checking them after a change.

`printcraft --export <project> <out.pdf|out.png>` converts without a window,
which is handy for checking how an older project file is read.

## Updates

`update::detect` works out how the running copy was installed (Windows
setup/MSI/portable, macOS app, AppImage, Flatpak, .deb, .rpm, tarball) and
`Release::asset_for` picks the release file with the same package type and
processor, using the names from `.github/scripts/release-names.sh`; keep the
two in step. Downloads go to the Downloads folder and are checked against the
`sha256:` digest GitHub lists for each release file. The Flatpak installs its
update through `flatpak-spawn --host` (the manifest allows
`org.freedesktop.Flatpak`).

## Ruler guidelines and pages

Guidelines are stored per page in the `.pcraft` file (`guides`) and never
drawn when printing or exporting. Click a ruler to add one; drag its marker
along the ruler (1/16-inch steps); remove it with its ×, Delete, or by
dragging the marker more than about 40 points off the ruler. Items snap to
them. Rulers mark down to 1/16 inch when the marks are at least 5 points
apart.

Every page is shown in the workspace (in a list, or side by side); Settings >
Pages > Legacy Pages shows one page at a time with page buttons along the
bottom. Pressing on another page makes it the page being edited; the rulers
and guidelines follow it.

## Private test files

Keep real projects and art collections in `corpus/` (ignored by git). Don't
commit anyone's projects or purchased art; describe file contents in issues
instead.
