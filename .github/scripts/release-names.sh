#!/usr/bin/env bash
# Gives every release file one naming scheme, whatever tool built it:
#   printcraft-<version>-<os>-<processor>[-<kind>].<ext>
# e.g. printcraft-0.1.0-linux-x86_64.rpm, printcraft-0.1.0-windows-arm64-setup.exe,
#      printcraft-0.1.0-macos-universal.dmg, printcraft-0.1.0-web.zip
# Processors are spelled x86_64 and arm64.
#
# Usage: release-names.sh <folder> <version>   (renames the files in <folder> and its subfolders, in place)
set -euo pipefail
dir=$1
ver=$2

# the processor named in a file name, in PrintCraft's spelling (empty if there isn't one)
arch_of() {
  case "$1" in
    *x86_64* | *amd64* | *x64*) echo x86_64 ;;
    *aarch64* | *arm64*) echo arm64 ;;
    *) echo "" ;;
  esac
}

status=0
while IFS= read -r -d '' f; do
  b=$(basename "$f")
  lower=$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')
  a=$(arch_of "$lower")
  needs_arch=1
  case "$lower" in
    *.rpm) new="linux-$a.rpm" ;;
    *.deb) new="linux-$a.deb" ;;
    *.appimage) new="linux-$a.AppImage" ;;
    *.flatpak) new="linux-$a.flatpak" ;;
    *linux*.tar.gz) new="linux-$a.tar.gz" ;;
    *.msi) new="windows-$a.msi" ;;
    *-setup.exe) new="windows-$a-setup.exe" ;;
    *windows*.zip) new="windows-$a-portable.zip" ;;
    *.dmg) new="macos-universal.dmg"; needs_arch=0 ;;
    *.app.zip) new="macos-universal.app.zip"; needs_arch=0 ;;
    *web*.zip) new="web.zip"; needs_arch=0 ;;
    *) echo "left as is (not a known release file): $b" >&2; continue ;;
  esac
  if [ "$needs_arch" = 1 ] && [ -z "$a" ]; then
    echo "::warning::left as is (no processor in its name): $b"
    continue
  fi
  new="printcraft-$ver-$new"
  [ "$b" = "$new" ] && continue
  target="$(dirname "$f")/$new"
  if [ -e "$target" ]; then
    echo "::error::two release files would both be called $new ($b)"
    status=1
    continue
  fi
  mv "$f" "$target"
  echo "$b -> $new"
done < <(find "$dir" -type f -print0)
exit $status
