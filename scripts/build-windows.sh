#!/usr/bin/env bash
# Build a portable Windows (x86_64) copy of PrintCraft from Linux or macOS.
#
# Needs: rustup target add x86_64-pc-windows-gnu
#        Zig 0.13 and cargo-zigbuild, either on PATH or in .tools/ beside
#        this project (see docs/DEVELOPING.md).
# Writes: dist/PrintCraft-windows-x86_64.zip (nothing to install). Release
# builds with an icon and installers come from .github/workflows/release.yml.
set -euo pipefail
cd "$(dirname "$0")/.."
here="$PWD"
for d in "$here"/.tools/zig-*/ "$here/.tools/cargo/bin"; do
    [ -d "$d" ] && PATH="$d:$PATH"
done
export PATH
cargo zigbuild --release --target x86_64-pc-windows-gnu
mkdir -p dist/PrintCraft
cp target/x86_64-pc-windows-gnu/release/printcraft.exe dist/PrintCraft/PrintCraft.exe
# The Google Images window (uses the Edge WebView2 built into Windows 10/11).
cp target/x86_64-pc-windows-gnu/release/printcraft-web.exe dist/PrintCraft/printcraft-web.exe
cp README.md LICENSE THIRD-PARTY-NOTICES.md dist/PrintCraft/
cat > dist/PrintCraft/READ-ME-FIRST.txt <<'EOF'
PrintCraft for Windows (portable test copy)

Double-click PrintCraft.exe (keep printcraft-web.exe beside it: it is the
Google Images window). Nothing is installed; you can run it from a
USB stick or the Desktop. Windows may say "Windows protected your PC" the
first time because this copy isn't signed: click "More info", then
"Run anyway".

Your projects, settings and saved pictures go in your own folders
(Documents, Pictures\PrintCraft Art), not beside the program.

To print, PrintCraft opens the printout as a PDF; print it from there.
EOF
rm -f dist/PrintCraft-windows-x86_64.zip
python3 -I -c "import shutil,sys; shutil.make_archive(sys.argv[1], 'zip', sys.argv[2], 'PrintCraft')" dist/PrintCraft-windows-x86_64 dist
ls -la dist/PrintCraft-windows-x86_64.zip
