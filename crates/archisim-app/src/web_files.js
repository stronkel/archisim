// Files in the browser (docs/IPAD.md, T2): choosing a file, handing one
// over (share sheet or download) and the copy kept in the browser.
//
// Safari only opens the file picker or the share sheet when the person
// taps something, and the program's own taps reach it a moment later: so
// both are a small card over the program with a real button to tap.

const STYLE = `
.as-veil { position: fixed; inset: 0; background: rgba(0,0,0,.45); display: flex;
  align-items: center; justify-content: center; z-index: 10; font: 16px -apple-system, "Segoe UI", sans-serif; }
.as-card { background: #ffffff; color: #1f2328; border-radius: 14px; padding: 22px 24px; width: min(420px, 86vw);
  box-shadow: 0 12px 40px rgba(0,0,0,.35); }
.as-card h2 { font-size: 18px; margin: 0 0 6px 0; }
.as-card p { margin: 0 0 16px 0; color: #57606a; font-size: 14px; line-height: 1.4; }
.as-btn { display: flex; align-items: center; justify-content: center; min-height: 48px; border-radius: 10px;
  margin-top: 10px; font-weight: 600; cursor: pointer; border: 0; width: 100%; font-size: 16px; }
.as-main { background: #0969da; color: #ffffff; }
.as-other { background: #eaeef2; color: #1f2328; }
.as-btn input { display: none; }
@media (prefers-color-scheme: dark) {
  .as-card { background: #2d333b; color: #e6edf3; } .as-card p { color: #9da7b3; }
  .as-other { background: #444c56; color: #e6edf3; }
}`;

function card(title, text) {
  if (!document.getElementById("as-style")) {
    const s = document.createElement("style");
    s.id = "as-style";
    s.textContent = STYLE;
    document.head.appendChild(s);
  }
  const veil = document.createElement("div");
  veil.className = "as-veil";
  const box = document.createElement("div");
  box.className = "as-card";
  const h = document.createElement("h2");
  h.textContent = title;
  const p = document.createElement("p");
  p.textContent = text;
  box.append(h, p);
  veil.appendChild(box);
  document.body.appendChild(veil);
  return { veil, box };
}

function button(box, label, main) {
  const b = document.createElement("button");
  b.className = "as-btn " + (main ? "as-main" : "as-other");
  b.textContent = label;
  box.appendChild(b);
  return b;
}

// Asks for a file. Resolves to [name, Uint8Array], or null if cancelled.
// No file types are given to the picker: on the iPad, types it does not
// know (.archisim) would come out greyed.
export function archisim_pick(title, text) {
  return new Promise((resolve) => {
    const { veil, box } = card(title, text);
    const label = document.createElement("label");
    label.className = "as-btn as-main";
    label.textContent = "Elegir archivo…";
    const input = document.createElement("input");
    input.type = "file";
    label.appendChild(input);
    box.appendChild(label);
    const cancel = button(box, "Cancelar", false);
    const done = (value) => { veil.remove(); resolve(value); };
    cancel.onclick = () => done(null);
    input.onchange = () => {
      const f = input.files && input.files[0];
      if (!f) { done(null); return; }
      label.textContent = "Leyendo " + f.name + "…";
      f.arrayBuffer().then((buf) => done([f.name, new Uint8Array(buf)]), () => done(null));
    };
  });
}

// Hands a file over: to the share sheet (on the iPad, «Guardar en
// Archivos» lets the person choose iCloud Drive or any folder) or as a
// download. Resolves when the card is closed.
export function archisim_deliver(name, bytes, mime) {
  return new Promise((resolve) => {
    const file = new File([bytes], name, { type: mime });
    const { veil, box } = card("Guardar «" + name + "»",
      "Elige dónde: «Guardar en Archivos» deja escoger la carpeta (iCloud Drive, el iPad…); «Descargar» lo deja en Descargas.");
    const done = () => { veil.remove(); resolve(); };
    if (navigator.canShare && navigator.canShare({ files: [file] })) {
      const share = button(box, "Guardar en Archivos…", true);
      share.onclick = () => { navigator.share({ files: [file] }).then(done, () => {}); };
    }
    const down = button(box, "Descargar", !(navigator.canShare && navigator.canShare({ files: [file] })));
    down.onclick = () => {
      const url = URL.createObjectURL(file);
      const a = document.createElement("a");
      a.href = url;
      a.download = name;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 60000);
      done();
    };
    button(box, "Cerrar", false).onclick = done;
  });
}

// The copy kept in the browser (IndexedDB), so a closed page loses
// nothing.
function db() {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open("archisim", 1);
    open.onupgradeneeded = () => open.result.createObjectStore("copias");
    open.onsuccess = () => resolve(open.result);
    open.onerror = () => reject(open.error);
  });
}

export function archisim_keep(key, name, bytes) {
  return db().then((d) => new Promise((resolve, reject) => {
    const t = d.transaction("copias", "readwrite");
    t.objectStore("copias").put({ name, bytes, time: Date.now() }, key);
    t.oncomplete = () => resolve(true);
    t.onerror = () => reject(t.error);
  }));
}

// Resolves to [name, Uint8Array, milliseconds] or null.
export function archisim_kept(key) {
  return db().then((d) => new Promise((resolve) => {
    const r = d.transaction("copias").objectStore("copias").get(key);
    r.onsuccess = () => {
      const v = r.result;
      resolve(v ? [v.name, v.bytes, v.time] : null);
    };
    r.onerror = () => resolve(null);
  })).catch(() => null);
}

// Whether leaving the page should ask first (changes not handed over).
export function archisim_set_unsaved(unsaved) {
  window.onbeforeunload = unsaved ? (e) => { e.preventDefault(); e.returnValue = ""; return ""; } : null;
}

// The Apple Pencil (docs/IPAD.md §2): which touches are the Pencil
// (Safari says so in `touchType`), and the Pencil held over the screen
// without touching it, which moves the cursor (iPad Pro and Air M2 or
// later), passed on as the mouse moving.
const PENS = new Set();

export function archisim_watch_pencil(canvasId) {
  const canvas = document.getElementById(canvasId);
  if (!canvas) return;
  const note = (e) => {
    for (const t of e.changedTouches) {
      if (t.touchType === "stylus") PENS.add(t.identifier);
    }
  };
  // Before the program's own listeners, on the way down.
  canvas.addEventListener("touchstart", note, { capture: true, passive: true });
  const forget = (e) => {
    for (const t of e.changedTouches) {
      const id = t.identifier;
      setTimeout(() => PENS.delete(id), 2000);
    }
  };
  canvas.addEventListener("touchend", forget, { capture: true, passive: true });
  canvas.addEventListener("touchcancel", forget, { capture: true, passive: true });
  // A finger or the Pencil also comes as a pointer, before it comes as a
  // touch; the program reads touches (crate::touch), so the pointer copy
  // stops here, or a finger meant to move the view would first press the
  // left button.
  const onlyTouches = (e) => {
    if (e.pointerType !== "mouse" && e.target === canvas) e.stopPropagation();
  };
  window.addEventListener("pointerdown", onlyTouches, { capture: true });
  window.addEventListener("pointerup", onlyTouches, { capture: true });
  canvas.addEventListener("pointermove", (e) => {
    if (e.pointerType === "pen" && e.buttons === 0) {
      canvas.dispatchEvent(new MouseEvent("mousemove", {
        clientX: e.clientX, clientY: e.clientY, bubbles: true,
      }));
    }
  }, { passive: true });
}

export function archisim_is_pen(identifier) {
  return PENS.has(identifier);
}

// --- Lo pesado (T5) ---

// The program's own script (wasm-bindgen's), which the workers load: the
// page has it as a module preload, or in its starting script.
function glueUrl() {
  const link = document.querySelector('link[rel="modulepreload"][href$=".js"]');
  if (link) return new URL(link.getAttribute("href"), document.baseURI).href;
  for (const s of document.querySelectorAll('script[type="module"]')) {
    const m = /['"]([^'"]+\.js)['"]/.exec(s.textContent || "");
    if (m) return new URL(m[1], document.baseURI).href;
  }
  return null;
}

let WORKER_URL;
function workerUrl() {
  if (WORKER_URL !== undefined) return WORKER_URL;
  WORKER_URL = null;
  const glue = glueUrl();
  if (!glue || typeof Worker === "undefined") return null;
  const code = `import init, { archisim_worker_job } from ${JSON.stringify(glue)};
self.onmessage = async (e) => {
  const { kind, input, params, module } = e.data;
  try {
    await init({ module_or_path: module });
    const out = archisim_worker_job(kind, input, params, (f) => self.postMessage({ progress: f }));
    self.postMessage({ out }, [out.buffer]);
  } catch (err) {
    self.postMessage({ error: String((err && err.message) || err) });
  }
};`;
  WORKER_URL = URL.createObjectURL(new Blob([code], { type: "text/javascript" }));
  return WORKER_URL;
}

// How many workers share a long job: one per core but one, up to four
// (each holds its own copy of the project).
export function archisim_workers() {
  if (!workerUrl()) return 0;
  return Math.max(1, Math.min(4, (navigator.hardwareConcurrency || 2) - 1));
}

// A job in a worker of its own, ended (and its memory freed) when the job
// ends. `module` is the program, already compiled.
export function archisim_run_aside(module, kind, input, params, onProgress) {
  return new Promise((resolve, reject) => {
    const url = workerUrl();
    if (!url) {
      reject("este navegador no tiene hilos aparte");
      return;
    }
    const w = new Worker(url, { type: "module" });
    w.onmessage = (e) => {
      const d = e.data;
      if ("progress" in d) {
        onProgress(d.progress);
        return;
      }
      w.terminate();
      if (d.error !== undefined) reject(d.error);
      else resolve(d.out);
    };
    w.onerror = (e) => {
      w.terminate();
      reject((e && e.message) || "el hilo aparte ha fallado");
    };
    w.postMessage({ kind, input, params, module });
  });
}

// A request that waits for its answer (the services of the surroundings,
// which the program asks one after the other), as bytes. Through the NAS's
// bridge when there is one (`archisim_has_bridge`).
export function archisim_http_sync(url, form) {
  const x = new XMLHttpRequest();
  x.open(form == null ? "GET" : "POST", url, false);
  // Bytes, not text: each byte comes as one character.
  x.overrideMimeType("text/plain; charset=x-user-defined");
  if (form != null) x.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
  try {
    x.send(form == null ? null : form);
  } catch (e) {
    throw "no se llega al servicio (" + ((e && e.message) || e) + ")";
  }
  if (x.status === 0) {
    throw "el navegador no deja pedírselo a ese servicio desde la página; hace falta el puente del NAS";
  }
  const t = x.responseText || "";
  const out = new Uint8Array(t.length);
  for (let i = 0; i < t.length; i++) out[i] = t.charCodeAt(i) & 0xff;
  if (x.status < 200 || x.status >= 300) {
    const body = new TextDecoder().decode(out.slice(0, 120)).trim();
    throw "el servicio responde " + x.status + " " + body;
  }
  return out;
}

// Whether the page is served by the NAS with its bridge (scripts/nas).
let BRIDGE;
export function archisim_has_bridge() {
  if (BRIDGE !== undefined) return BRIDGE;
  BRIDGE = false;
  try {
    const x = new XMLHttpRequest();
    x.open("GET", "proxy/", false);
    x.send(null);
    BRIDGE = x.status === 200 && (x.responseText || "").startsWith("archisim-proxy");
  } catch (e) {
    BRIDGE = false;
  }
  return BRIDGE;
}

// How much memory the program holds (its WebAssembly memory), in bytes,
// left where the page's tests read it.
export function archisim_note_memory(bytes) {
  window.archisimMemoria = bytes;
}
