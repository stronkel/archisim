#!/usr/bin/env bash
# Builds ArchiSim for macOS as a universal application (Apple chip and
# Intel) inside a disk image, with the demo house and the IFC examples:
#
#   dist/ArchiSim-<version>.dmg
#
# Run on a Mac (the continuous integration does it on every pull request).
# The application is signed ad hoc, not with an Apple developer account:
# the first time, macOS asks to confirm it in System Settings → Privacy &
# Security → "Open Anyway".
set -euo pipefail

cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
targets=(aarch64-apple-darwin x86_64-apple-darwin)

rustup target add "${targets[@]}"
for t in "${targets[@]}"; do
    cargo build --release -p archisim-app --target "$t"
done

dist=dist
app="$dist/ArchiSim.app"
rm -rf "$dist"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/ArchiSim" \
    "target/aarch64-apple-darwin/release/archisim-app" \
    "target/x86_64-apple-darwin/release/archisim-app"

# Icon: every size macOS asks for, from the 1024 px picture.
iconset="$dist/ArchiSim.iconset"
mkdir -p "$iconset"
for s in 16 32 128 256 512; do
    sips -z $s $s crates/archisim-app/assets/icon.png --out "$iconset/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z $d $d crates/archisim-app/assets/icon.png --out "$iconset/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/ArchiSim.icns"
rm -rf "$iconset"

# Info.plist: name, version, icon, and the files the application opens
# (double click in the Finder): its own projects, and IFC files as an
# alternative application ("Abrir con").
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>ArchiSim</string>
    <key>CFBundleDisplayName</key><string>ArchiSim</string>
    <key>CFBundleIdentifier</key><string>es.ad5.archisim</string>
    <key>CFBundleExecutable</key><string>ArchiSim</string>
    <key>CFBundleIconFile</key><string>ArchiSim</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${version}</string>
    <key>CFBundleVersion</key><string>${version}</string>
    <key>CFBundleDevelopmentRegion</key><string>es</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key><string>Proyecto ArchiSim</string>
            <key>CFBundleTypeRole</key><string>Editor</string>
            <key>CFBundleTypeIconFile</key><string>ArchiSim</string>
            <key>LSHandlerRank</key><string>Owner</string>
            <key>LSItemContentTypes</key><array><string>es.ad5.archisim.project</string></array>
        </dict>
        <dict>
            <key>CFBundleTypeName</key><string>Modelo IFC</string>
            <key>CFBundleTypeRole</key><string>Editor</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>LSItemContentTypes</key><array><string>org.buildingsmart.ifc</string></array>
        </dict>
    </array>
    <key>UTExportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key><string>es.ad5.archisim.project</string>
            <key>UTTypeDescription</key><string>Proyecto ArchiSim</string>
            <key>UTTypeIconFile</key><string>ArchiSim</string>
            <key>UTTypeConformsTo</key><array><string>public.json</string><string>public.data</string></array>
            <key>UTTypeTagSpecification</key>
            <dict><key>public.filename-extension</key><array><string>archisim</string></array></dict>
        </dict>
    </array>
    <key>UTImportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key><string>org.buildingsmart.ifc</string>
            <key>UTTypeDescription</key><string>Modelo IFC</string>
            <key>UTTypeConformsTo</key><array><string>public.data</string></array>
            <key>UTTypeTagSpecification</key>
            <dict><key>public.filename-extension</key><array><string>ifc</string><string>IFC</string></array></dict>
        </dict>
    </array>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"

# Ad hoc signature: required for the Apple chip, and enough to run it
# after confirming once.
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

# Disk image: the application, a link to Applications, the demo house,
# IFC examples written by another program, and how to open it the first
# time.
stage="$dist/ArchiSim"
mkdir -p "$stage/ejemplos-ifc"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Aplicaciones"
cargo run --release -p archisim-cli -- demo "$dist/demo" >/dev/null
cp "$dist/demo/demo.archisim" "$stage/"
cp crates/archisim-ifc/tests/fixtures/*.ifc "$stage/ejemplos-ifc/"
cat > "$stage/LEEME.txt" <<'TXT'
ArchiSim para Mac (Apple M1 y posteriores, e Intel)

1. Arrastra ArchiSim a la carpeta Aplicaciones.
2. La primera vez, macOS dirá que no puede comprobar el desarrollador
   (la aplicación no está firmada con una cuenta de Apple). Pulsa
   «Hecho», abre Ajustes del Sistema → Privacidad y seguridad, baja
   hasta «ArchiSim» y pulsa «Abrir igualmente». Solo hace falta una vez.
   Si después de «Abrir igualmente» vuelve a pedirlo una y otra vez
   (pasa en macOS Sequoia), abre Terminal (Aplicaciones → Utilidades),
   pega esta línea y pulsa Intro; luego ábrela con doble clic:
       xattr -dr com.apple.quarantine /Applications/ArchiSim.app
3. Doble clic en demo.archisim (o arrástralo sobre el icono del Dock)
   para abrir la vivienda de prueba. Los .ifc se abren con
   clic derecho → Abrir con → ArchiSim.
TXT
dmg="$dist/ArchiSim-${version}.dmg"
hdiutil create -volname "ArchiSim ${version}" -srcfolder "$stage" -ov -format UDZO "$dmg" >/dev/null
echo "$dmg"
