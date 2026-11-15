#!/usr/bin/env python3
"""Opens the web version (dist/web) in a headless Chromium and checks that
ArchiSim starts: the «loading» text goes, nothing panics, and the plan and
the 3D view draw something. Writes pictures of both into the folder given
(and, as text, into the log, between the marks PNG-INICIO/PNG-FIN).

    python scripts/web_smoke.py dist/web out/web-fotos

Needs: pip install playwright && playwright install --with-deps chromium
"""

import base64
import functools
import http.server
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

site = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "dist/web").resolve()
out = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "out/web-fotos")
out.mkdir(parents=True, exist_ok=True)


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
    }

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", 0), functools.partial(Quiet, directory=str(site))
)
threading.Thread(target=server.serve_forever, daemon=True).start()
url = f"http://127.0.0.1:{server.server_address[1]}/"

console = []
failed = False


def picture(page, name):
    path = out / f"{name}.png"
    page.screenshot(path=str(path))
    print(f"PNG-INICIO {name}")
    print(base64.b64encode(path.read_bytes()).decode())
    print(f"PNG-FIN {name}")
    return path


DEMO_IFC = pathlib.Path("out/web-demo/demo.ifc").resolve()


def files(page):
    """Opening, importing and saving, as the person would: Ctrl+O and the
    picker card, Ctrl+S and «Descargar», and the copy kept in the browser
    when the page opens again."""
    global failed
    try:
        page.focus("#archisim")
        page.keyboard.press("Control+O")
        page.wait_for_selector(".as-veil", timeout=10_000)
        page.set_input_files("input[type=file]", str(site / "demo.archisim"))
        page.wait_for_selector(".as-veil", state="detached", timeout=30_000)
        page.wait_for_timeout(4_000)
        picture(page, "webgl-3-abierto")
        if DEMO_IFC.exists():
            page.focus("#archisim")
            page.keyboard.press("Control+O")
            page.wait_for_selector(".as-veil", timeout=10_000)
            page.set_input_files("input[type=file]", str(DEMO_IFC))
            page.wait_for_selector(".as-veil", state="detached", timeout=30_000)
            page.wait_for_timeout(6_000)
            picture(page, "webgl-4-ifc")
        page.focus("#archisim")
        page.keyboard.press("Control+S")
        page.wait_for_selector(".as-veil", timeout=10_000)
        with page.expect_download(timeout=30_000) as d:
            page.click("button:has-text('Descargar')")
        target = out / d.value.suggested_filename
        d.value.save_as(str(target))
        size = target.stat().st_size
        text = target.read_text(encoding="utf-8")[:1]
        print(f"descargado {target.name}: {size} bytes")
        if size < 1000 or text != "{":
            print("La descarga no es un proyecto")
            failed = True
        # Leaving the page and coming back opens the copy kept.
        page.reload()
        page.wait_for_selector("#cargando", state="detached", timeout=180_000)
        page.wait_for_timeout(5_000)
        kept = any("Abierta la copia guardada" in c for c in console)
        print(f"copia del navegador al volver: {kept}")
        if not kept:
            failed = True
        picture(page, "webgl-5-copia")
    except Exception as e:  # noqa: BLE001 - any failure is reported
        print(f"Archivos: {e}")
        failed = True


MODES = {
    # WebGPU, as in Safari 26 (a software adapter here: its pictures may
    # come out blank in a headless browser, but errors would show).
    "webgpu": [
        "--enable-unsafe-webgpu",
        "--enable-features=Vulkan",
        "--use-webgpu-adapter=swiftshader",
        "--use-angle=swiftshader",
        "--ignore-gpu-blocklist",
    ],
    # Without WebGPU the program draws with WebGL: the pictures show it.
    "webgl": ["--disable-features=WebGPU", "--use-angle=swiftshader"],
}

with sync_playwright() as p:
    for mode, args in MODES.items():
        console.append(f"=== {mode} ===")
        browser = p.chromium.launch(headless=True, args=args)
        page = browser.new_page(viewport={"width": 1280, "height": 800})
        page.on("console", lambda m: console.append(f"[{m.type}] {m.text}"))
        page.on("pageerror", lambda e: console.append(f"[pageerror] {e}"))
        page.on("dialog", lambda d: d.accept())
        page.goto(url)
        gpu = page.evaluate("() => !!navigator.gpu")
        print(f"{mode}: navigator.gpu = {gpu}")
        try:
            page.wait_for_selector("#cargando", state="detached", timeout=180_000)
            print(f"{mode}: ArchiSim ha arrancado")
        except Exception:
            failed = True
            text = page.evaluate(
                "() => document.getElementById('cargando')?.innerText ?? ''"
            )
            print(f"{mode}: ArchiSim no ha arrancado: {text}")
        page.wait_for_timeout(8_000)
        picture(page, f"{mode}-1-planta")
        if mode == "webgl":
            files(page)
        # The 3D view: F3, with the canvas focused.
        page.focus("#archisim")
        page.keyboard.press("F3")
        page.wait_for_timeout(10_000)
        picture(page, f"{mode}-2-vista3d")
        browser.close()

print("--- consola del navegador ---")
for line in console[-200:]:
    print(line)
if any("panicked" in c or "[pageerror]" in c for c in console):
    failed = True
sys.exit(1 if failed else 0)
