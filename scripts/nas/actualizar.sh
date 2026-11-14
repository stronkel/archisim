#!/bin/sh
# Pone al día /srv/web/actual con la última versión web de ArchiSim
# publicada en GitHub (archisim-web.zip), cada 10 minutos.
set -u
apk add --no-cache curl unzip >/dev/null 2>&1

REPO=stronkel/archisim
WEB=/srv/web
ZIP_URL="https://github.com/$REPO/releases/latest/download/archisim-web.zip"

while true; do
  latest=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -1)
  current=$(cat "$WEB/actual/version.txt" 2>/dev/null || echo "")
  if [ -n "$latest" ] && [ "$latest" != "$current" ]; then
    echo "ArchiSim $latest (había ${current:-ninguna})"
    rm -rf "$WEB/nueva" "$WEB/nueva.zip"
    if curl -fsSL -o "$WEB/nueva.zip" "$ZIP_URL" \
      && mkdir -p "$WEB/nueva" \
      && unzip -q "$WEB/nueva.zip" -d "$WEB/nueva" \
      && [ -f "$WEB/nueva/index.html" ]; then
      rm -rf "$WEB/anterior"
      [ -d "$WEB/actual" ] && mv "$WEB/actual" "$WEB/anterior"
      mv "$WEB/nueva" "$WEB/actual"
      echo "Lista: ArchiSim $latest"
    else
      echo "No se ha podido descargar ArchiSim $latest; se vuelve a intentar"
    fi
    rm -f "$WEB/nueva.zip"
  fi
  sleep 600
done
