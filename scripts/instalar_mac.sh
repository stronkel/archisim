#!/usr/bin/env bash
# Installs (or updates) ArchiSim on a Mac from the latest version
# published on GitHub, in one line:
#
#   curl -fsSL https://github.com/stronkel/archisim/releases/latest/download/instalar-mac.sh | bash
#
# What curl downloads is not put in quarantine, so macOS opens the
# application without asking (it is signed ad hoc, not with an Apple
# developer account, and a browser download would be blocked).
#
# For the continuous integration and for trying a local build:
#   ARCHISIM_DMG=dist/ArchiSim-0.22.1.dmg    use this disk image, no download
#   ARCHISIM_DESTINO=/some/folder            install there, not in /Applications
set -euo pipefail

url="https://github.com/stronkel/archisim/releases/latest/download/ArchiSim-Mac.dmg"
dest="${ARCHISIM_DESTINO:-/Applications}"

work=$(mktemp -d)
mnt="$work/volumen"
cleanup() {
    hdiutil detach "$mnt" -quiet 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

if [ -n "${ARCHISIM_DMG:-}" ]; then
    dmg="$ARCHISIM_DMG"
else
    echo "Descargando la última versión de ArchiSim…"
    dmg="$work/ArchiSim.dmg"
    curl -fL --progress-bar -o "$dmg" "$url"
fi

hdiutil attach "$dmg" -nobrowse -readonly -quiet -mountpoint "$mnt"
if [ ! -d "$mnt/ArchiSim.app" ]; then
    echo "La imagen descargada no contiene ArchiSim.app." >&2
    exit 1
fi

# An open ArchiSim would keep running the old version: ask it to quit
# (it asks to save if there are changes) and wait for it.
if pgrep -xq ArchiSim; then
    echo "Cerrando ArchiSim (si tienes cambios sin guardar, te preguntará)…"
    osascript -e 'tell application id "es.ad5.archisim" to quit' >/dev/null 2>&1 || true
    for _ in $(seq 120); do
        pgrep -xq ArchiSim || break
        sleep 1
    done
    if pgrep -xq ArchiSim; then
        echo "ArchiSim sigue abierto: ciérralo y vuelve a ejecutar la orden." >&2
        exit 1
    fi
fi

mkdir -p "$dest"
rm -rf "$dest/ArchiSim.app"
ditto "$mnt/ArchiSim.app" "$dest/ArchiSim.app"
# Nothing downloaded this way is in quarantine; this only cleans up a
# copy that was installed by hand before from a browser download.
xattr -dr com.apple.quarantine "$dest/ArchiSim.app" 2>/dev/null || true

version=$(defaults read "$dest/ArchiSim.app/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "?")
echo "ArchiSim $version instalado en $dest. Ábrelo desde Aplicaciones o el Launchpad."
