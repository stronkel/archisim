#!/usr/bin/env python3
"""The heavy part of the web version (docs/IPAD.md §11, T5), as the NAS
serves it: Caddy with the Caddyfile of scripts/nas (user and password, and
the bridge to the services of the surroundings) in front of dist/web.

  1. The bridge: «proxy/» answers, a service of the list answers through
     it, one out of the list does not.
  2. A place looked up by the program through the bridge (?prueba=lugar).
  3. A render made by the browser's workers (?prueba=render): how long and
     with how many.
  4. Florence Street (the big building, `archisim edificio`) opened in the
     page: the memory the program holds, after opening and in 3D.

    python scripts/web_heavy.py dist/web out/edificio/edificio.archisim out/web-pesado

Needs: caddy in the PATH, pip install playwright bcrypt, playwright install chromium
"""

import base64
import os
import pathlib
import subprocess
import sys
import tempfile
import time

from playwright.sync_api import sync_playwright

site = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "dist/web").resolve()
building = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "out/edificio/edificio.archisim")
out = pathlib.Path(sys.argv[3] if len(sys.argv) > 3 else "out/web-pesado")
out.mkdir(parents=True, exist_ok=True)
repo = pathlib.Path(__file__).resolve().parent.parent

USER, PASSWORD, PORT = "antonio", "prueba-del-nas", 4281
failed = False
console = []


def fail(text):
    global failed
    print(f"FALLO: {text}")
    failed = True


def picture(page, name):
    path = out / f"{name}.png"
    page.screenshot(path=str(path))
    print(f"PNG-INICIO {name}")
    print(base64.b64encode(path.read_bytes()).decode())
    print(f"PNG-FIN {name}")


# Caddy as on the NAS: the same Caddyfile, the page where the updater
# leaves it.
work = pathlib.Path(tempfile.mkdtemp())
(work / "web").mkdir()
(work / "web" / "actual").symlink_to(site)
caddyfile = (repo / "scripts/nas/Caddyfile").read_text(encoding="utf-8")
caddyfile = caddyfile.replace("/srv/web", str(work / "web")).replace(":80 {", f":{PORT} {{")
(work / "Caddyfile").write_text(caddyfile, encoding="utf-8")
hashed = subprocess.run(
    ["caddy", "hash-password", "--plaintext", PASSWORD], capture_output=True, text=True, check=True
).stdout.strip()
env = {**os.environ, "ARCHISIM_USUARIO": USER, "ARCHISIM_CLAVE_CIFRADA": hashed}
caddy = subprocess.Popen(
    ["caddy", "run", "--config", str(work / "Caddyfile"), "--adapter", "caddyfile"],
    env=env,
    stdout=open(work / "caddy.log", "w"),
    stderr=subprocess.STDOUT,
)
time.sleep(2)
url = f"http://127.0.0.1:{PORT}/"


# Not the first time: no welcome card over the plan.
WELCOMED = (
    "if (!localStorage.getItem('archisim-preferencias')) localStorage.setItem('archisim-preferencias', JSON.stringify({ bienvenida: true }))"
)


def page_number(page, key, seconds):
    """Waits for the program to leave a number in window[key]."""
    end = time.time() + seconds
    while time.time() < end:
        v = page.evaluate(f"() => window.{key}")
        if v is not None:
            return v
        page.wait_for_timeout(1_000)
    return None


def mb(n):
    return f"{n / 1048576:.0f} MB" if n is not None else "?"


try:
    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True, args=["--disable-features=WebGPU", "--use-angle=swiftshader"]
        )
        # Without the password, nothing.
        bare = browser.new_page()
        status = bare.goto(url).status
        print(f"sin contraseña: {status}")
        if status != 401:
            fail("el NAS abre la página sin contraseña")
        bare.close()

        context = browser.new_context(
            viewport={"width": 1280, "height": 800},
            http_credentials={"username": USER, "password": PASSWORD},
        )
        page = context.new_page()
        page.on("console", lambda m: console.append(f"[{m.type}] {m.text}"))
        page.on("pageerror", lambda e: console.append(f"[pageerror] {e}"))
        page.on("dialog", lambda d: d.accept())
        page.add_init_script(WELCOMED)

        # 1, 2 and 3: the bridge, a place, a render.
        page.goto(url + "?prueba=render,lugar")
        page.wait_for_selector("#cargando", state="detached", timeout=180_000)
        bridge = page.evaluate(
            """async () => {
                const ask = async (u) => { const r = await fetch(u); return [r.status, (await r.text()).slice(0, 60)]; };
                return {
                    puente: await ask('proxy/'),
                    cartociudad: await ask('proxy/www.cartociudad.es/geocoder/api/geocoder/candidates?q=Huelva&limit=1'),
                    fuera: await ask('proxy/example.com/'),
                };
            }"""
        )
        print(f"puente: {bridge}")
        if bridge["puente"][0] != 200 or not bridge["puente"][1].startswith("archisim-proxy"):
            fail("el NAS no tiene el puente")
        if bridge["cartociudad"][0] != 200:
            fail("CartoCiudad no responde por el puente")
        if bridge["fuera"][0] != 403:
            fail("el puente deja pedir a un servidor fuera de la lista")

        places = page_number(page, "archisimLugares", 120)
        print(f"lugares encontrados por el puente: {places}")
        if not places or places < 1:
            fail("el programa no encuentra un lugar por el puente")

        workers = page.evaluate("() => navigator.hardwareConcurrency")
        seconds = page_number(page, "archisimRender", 600)
        print(f"render en borrador: {seconds} s, con {workers} núcleos en el navegador")
        if seconds is None:
            fail("el render no ha terminado")
        page.wait_for_timeout(2_000)
        picture(page, "pesado-1-render")
        if not any("hace aquí" in c for c in console):
            print("el render se ha hecho en los hilos del navegador")
        else:
            fail("el render no se ha podido hacer en los hilos del navegador")

        # 4. Florence Street, with WebGPU (as in Safari 26; with WebGL the
        # 3D does not start).
        browser.close()
        browser = p.chromium.launch(
            headless=True,
            args=[
                "--enable-unsafe-webgpu",
                "--enable-features=Vulkan",
                "--use-webgpu-adapter=swiftshader",
                "--use-angle=swiftshader",
                "--ignore-gpu-blocklist",
            ],
        )
        context = browser.new_context(
            viewport={"width": 1280, "height": 800},
            http_credentials={"username": USER, "password": PASSWORD},
        )
        page = context.new_page()
        page.on("console", lambda m: console.append(f"[{m.type}] {m.text}"))
        page.on("pageerror", lambda e: console.append(f"[pageerror] {e}"))
        page.on("dialog", lambda d: d.accept())
        page.add_init_script(WELCOMED)
        if building.exists():
            page.goto(url)
            page.wait_for_selector("#cargando", state="detached", timeout=180_000)
            page.wait_for_timeout(3_000)
            start = page.evaluate("() => window.archisimMemoria")
            t0 = time.time()
            page.focus("#archisim")
            page.keyboard.press("Control+O")
            page.wait_for_selector(".as-veil", timeout=10_000)
            page.set_input_files("input[type=file]", str(building))
            page.wait_for_selector(".as-veil", state="detached", timeout=60_000)
            page.wait_for_timeout(10_000)
            opened = page.evaluate("() => window.archisimMemoria")
            print(f"Florence Street abierto en unos {time.time() - t0:.0f} s")
            picture(page, "pesado-2-edificio-planta")
            page.focus("#archisim")
            page.keyboard.press("F3")
            page.wait_for_timeout(30_000)
            in_3d = page.evaluate("() => window.archisimMemoria")
            picture(page, "pesado-3-edificio-3d")
            print(
                f"memoria del programa: al empezar {mb(start)}, con Florence Street {mb(opened)}, en 3D {mb(in_3d)} ({building.stat().st_size / 1048576:.1f} MB de fichero)"
            )
            if in_3d is None or in_3d > 3.5 * 1024**3:
                fail("la memoria no se ha podido medir o pasa de 3,5 GB")
        else:
            print(f"(sin {building}: no se mide Florence Street)")
        browser.close()
finally:
    caddy.terminate()

print("--- consola del navegador ---")
for line in console[-150:]:
    print(line)
if any("panicked" in c or "[pageerror]" in c for c in console):
    failed = True
print("--- caddy ---")
print((work / "caddy.log").read_text(encoding="utf-8")[-3000:])
sys.exit(1 if failed else 0)
