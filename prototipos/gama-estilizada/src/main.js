import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { GLSL } from './materials.glsl.js';
import { RANGE } from './palette.js';

// ---- Renderer, camera ------------------------------------------------------

const canvas = document.getElementById('view');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFShadowMap;

const camera = new THREE.PerspectiveCamera(32, 1, 0.5, 600);
camera.up.set(0, 0, 1);
camera.layers.enable(1);
const controls = new OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.maxPolarAngle = Math.PI * 0.49;

// ---- Shared switches ---------------------------------------------------------

const G = {
  uStyle: { value: 1 },
  uRelief: { value: 1 },
  uGround: { value: 0 },
  uTime: { value: 0 },
};

// ---- Materials ---------------------------------------------------------------

const lambertChunk = THREE.ShaderChunk.lights_lambert_pars_fragment.replace(
  'float dotNL = saturate( dot( geometryNormal, directLight.direction ) );',
  `float ndl = dot( geometryNormal, directLight.direction );
   // Stylized: the lit side wraps a little further round and its edge is
   // crisper, like a painted shadow line.
   float dotNL = mix( saturate( ndl ), smoothstep( -0.08, 0.42, ndl ) * 0.82 + 0.18 * saturate( ndl ), uStyle );`
);

const materials = {};
function material(key, opts = {}) {
  const id = key + JSON.stringify(opts);
  if (materials[id]) return materials[id];
  const d = RANGE[key];
  const m = new THREE.MeshLambertMaterial({ color: 0xffffff });
  // Water is seen through: little when looking straight down, nearly
  // opaque at a glancing look (Fresnel).
  const water = d.water;
  if (water) { m.transparent = true; m.depthWrite = false; }
  const u = {
    uKind: { value: d.kind },
    uOld: { value: d.old },
    uOldScale: { value: d.oldScale },
    uBase: { value: new THREE.Color(d.base) },
    uBase2: { value: new THREE.Color(d.base2) },
    uJoint: { value: new THREE.Color(d.joint) },
    uDims: { value: new THREE.Vector4(...d.dims) },
    uMap: { value: opts.around ? 1 : 0 },
    uCenter: { value: new THREE.Vector3(...(opts.center || [0, 0, 0])) },
    uRadius: { value: opts.radius || 1 },
    ...G,
  };
  m.userData.u = u;
  m.userData.key = key;
  m.onBeforeCompile = (s) => {
    Object.assign(s.uniforms, u);
    s.vertexShader = s.vertexShader
      .replace('#include <common>', '#include <common>\nvarying vec3 vWorld;\nvarying vec3 vWN;')
      .replace('#include <project_vertex>', `#include <project_vertex>
        vWorld = (modelMatrix * vec4(transformed, 1.0)).xyz;
        vWN = normalize(mat3(modelMatrix) * objectNormal);`);
    s.fragmentShader = s.fragmentShader
      .replace('#include <common>', '#include <common>\nuniform float uGround;\n' + GLSL + '\nfloat gH = 0.0;')
      .replace('#include <lights_lambert_pars_fragment>', lambertChunk)
      .replace('#include <map_fragment>', `
        vec3 Nw0 = normalize(vWN);
        vec2 uvF; vec3 tF; vec3 bF;
        frameOf(Nw0, vWorld, uvF, tF, bF);
        vec3 col;
        if (uStyle > 0.5) {
          Look lk = stylized(uvF, uBase);
          col = lk.c; gH = lk.h;
          // A painted darkening at the foot of walls.
          float foot = smoothstep(0.0, 0.9, vWorld.z - uGround);
          col *= mix(0.8, 1.0, mix(1.0, foot, step(abs(Nw0.z), 0.5)));
        } else {
          col = current(uOld == 0 ? uBase : uBase, vWorld, Nw0);
        }
        diffuseColor.rgb = col;`)
      .replace('#include <normal_fragment_maps>', `#include <normal_fragment_maps>
        if (uStyle > 0.5 && uRelief > 0.5) {
          vec3 Nw = normalize(vWN);
          vec3 dpx = dFdx(vWorld), dpy = dFdy(vWorld);
          float hx = dFdx(gH), hy = dFdy(gH);
          vec3 r1 = cross(dpy, Nw), r2 = cross(Nw, dpx);
          float det = dot(dpx, r1);
          vec3 grad = sign(det) * (hx * r1 + hy * r2);
          float fade = 1.0 - smoothstep(0.006, 0.03, length(fwidth(vWorld)));
          vec3 Nb = normalize(abs(det) * Nw - grad * fade);
          normal = normalize((viewMatrix * vec4(Nb, 0.0)).xyz);
        }
        ${water ? `{
          float fres = pow(1.0 - abs(dot(normalize(vViewPosition), normal)), 3.0);
          diffuseColor.a = mix(${water[0].toFixed(3)}, ${water[1].toFixed(3)}, fres);
        }` : ''}`);
  };
  m.customProgramCacheKey = () => 'archisim-stylized' + (water ? water.join() : '');
  materials[id] = m;
  return m;
}

// ---- Geometry helpers (z up, metres) ----------------------------------------

function mesh(geo, mat, parent) {
  const o = new THREE.Mesh(geo, mat);
  o.castShadow = true;
  o.receiveShadow = true;
  parent.add(o);
  return o;
}

function box(parent, key, x, y, z, w, d, h, opts) {
  const o = mesh(new THREE.BoxGeometry(w, d, h), material(key, opts), parent);
  if (RANGE[key].water) o.castShadow = false;
  o.position.set(x, y, z + h / 2);
  return o;
}

// A solid from triangles (flat shaded).
function tris(parent, key, pts, opts) {
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pts.flat(), 3));
  g.computeVertexNormals();
  return mesh(g, material(key, opts), parent);
}
function quad(a, b, c, d) { return [a, b, c, a, c, d]; }

// A gable roof over a w × d rectangle centred on (x, y) with its eaves at z,
// ridge along x (or y if `alongY`), with overhang `o` and thickness `t`.
function gable(parent, key, x, y, z, w, d, rise, o = 0.4, alongY = false, t = 0.18) {
  const g = new THREE.Group();
  parent.add(g);
  g.position.set(x, y, z);
  if (alongY) g.rotation.z = Math.PI / 2;
  const W = (alongY ? d : w) / 2 + o, D = (alongY ? w : d) / 2 + o;
  const R = rise * D / (D - o);
  const z0 = -rise * o / (D - o);
  const P = (px, py, pz) => [px, py, pz];
  const pts = [];
  // top slopes
  pts.push(...quad(P(-W, -D, z0 + t), P(W, -D, z0 + t), P(W, 0, z0 + R + t), P(-W, 0, z0 + R + t)));
  pts.push(...quad(P(W, D, z0 + t), P(-W, D, z0 + t), P(-W, 0, z0 + R + t), P(W, 0, z0 + R + t)));
  // underside
  pts.push(...quad(P(W, -D, z0), P(-W, -D, z0), P(-W, 0, z0 + R), P(W, 0, z0 + R)));
  pts.push(...quad(P(-W, D, z0), P(W, D, z0), P(W, 0, z0 + R), P(-W, 0, z0 + R)));
  // eaves edges
  pts.push(...quad(P(-W, -D, z0), P(W, -D, z0), P(W, -D, z0 + t), P(-W, -D, z0 + t)));
  pts.push(...quad(P(W, D, z0), P(-W, D, z0), P(-W, D, z0 + t), P(W, D, z0 + t)));
  // verges
  for (const sx of [-1, 1]) {
    const X = sx * W;
    const v = [P(X, -D, z0), P(X, 0, z0 + R), P(X, D, z0), P(X, D, z0 + t), P(X, 0, z0 + R + t), P(X, -D, z0 + t)];
    const f = sx > 0 ? (a, b, c) => [v[a], v[b], v[c]] : (a, b, c) => [v[a], v[c], v[b]];
    pts.push(...f(0, 1, 5), ...f(1, 4, 5), ...f(1, 2, 4), ...f(2, 3, 4));
  }
  // The roof material is mapped in world coordinates: give its group's
  // transform to the geometry so the frame is right.
  const o2 = tris(g, key, pts);
  return { group: g, gableTri: (wallKey) => {
    const W2 = (alongY ? d : w) / 2, D2 = (alongY ? w : d) / 2;
    const q = [];
    for (const sx of [-1, 1]) {
      const X = sx * W2;
      const tri = [P(X, -D2, 0), P(X, 0, rise), P(X, D2, 0)];
      q.push(...(sx > 0 ? [tri[0], tri[2], tri[1]] : tri));
    }
    tris(g, wallKey, q);
  } };
}

function cylinder(parent, key, x, y, z, r, h, seg = 40, opts = {}) {
  const geo = new THREE.CylinderGeometry(r, r, h, seg, 1);
  geo.rotateX(Math.PI / 2);
  const o = mesh(geo, material(key, { around: true, center: [x, y, z], radius: r, ...opts }), parent);
  o.position.set(x, y, z + h / 2);
  return o;
}
function cone(parent, key, x, y, z, r, h, seg = 40) {
  const geo = new THREE.ConeGeometry(r, h, seg, 1, true);
  geo.rotateX(Math.PI / 2);
  const o = mesh(geo, material(key, { around: true, center: [x, y, z + h], radius: r }), parent);
  o.position.set(x, y, z + h / 2);
  return o;
}

// A window on a façade looking towards -y at y = fy.
function windowAt(parent, x, fy, z, w, h, o = {}) {
  const trim = o.trim || 'trimCream';
  const f = 0.07;
  box(parent, 'glass', x, fy - 0.01, z, w, 0.05, h);
  box(parent, trim, x, fy - 0.05, z - f, w + 2 * f, 0.12, f);           // bottom
  box(parent, trim, x, fy - 0.05, z + h, w + 2 * f, 0.12, f);           // top
  box(parent, trim, x - w / 2 - f / 2, fy - 0.05, z, f, 0.12, h);
  box(parent, trim, x + w / 2 + f / 2, fy - 0.05, z, f, 0.12, h);
  box(parent, trim, x, fy - 0.04, z, 0.04, 0.08, h);                     // mullion
  box(parent, trim, x, fy - 0.04, z + h * 0.58, w, 0.08, 0.04);          // transom
  box(parent, trim, x, fy - 0.1, z - f - 0.05, w + 0.3, 0.22, 0.06);     // sill
  if (o.head) box(parent, o.head, x, fy - 0.08, z + h + f, w + 0.36, 0.18, 0.16);
  if (o.shutters) {
    for (const s of [-1, 1]) box(parent, o.shutters, x + s * (w / 2 + f + 0.28), fy - 0.06, z, w / 2 + 0.05, 0.05, h);
  }
  if (o.flowers) {
    box(parent, 'trimBrown', x, fy - 0.2, z - f - 0.32, w + 0.1, 0.24, 0.24);
    const blob = new THREE.IcosahedronGeometry(0.16, 0);
    for (let i = -2; i <= 2; i++) {
      const b = mesh(blob, material('foliage'), parent);
      b.position.set(x + i * w / 5, fy - 0.22, z - f - 0.02 + (i % 2) * 0.04);
      b.scale.set(1.2, 0.9, 0.8);
    }
  }
}
function doorAt(parent, x, fy, z, w, h, key = 'trimBrown', trim = 'trimCream') {
  box(parent, key, x, fy - 0.01, z, w, 0.06, h);
  box(parent, trim, x, fy - 0.05, z + h, w + 0.24, 0.12, 0.12);
  box(parent, trim, x - w / 2 - 0.06, fy - 0.05, z, 0.12, 0.12, h);
  box(parent, trim, x + w / 2 + 0.06, fy - 0.05, z, 0.12, 0.12, h);
  box(parent, 'slabs', x, fy - 0.45, z - 0.18, w + 0.8, 0.9, 0.18);
}

function tree(parent, x, y, z, s = 1, round = true) {
  cylinder(parent, 'pole', x, y, z, 0.16 * s, 2.2 * s, 10);
  const blob = new THREE.IcosahedronGeometry(1, 1);
  const parts = round ? [[0, 0, 3.2, 1.6], [0.9, 0.3, 2.7, 1.1], [-0.8, -0.2, 2.8, 1.15], [0.1, -0.5, 4.0, 1.0]] :
    [[0, 0, 2.4, 1.2], [0, 0, 3.4, 0.95], [0, 0, 4.3, 0.7], [0, 0, 5.0, 0.4]];
  for (const [dx, dy, dz, r] of parts) {
    const b = mesh(blob, material('foliage'), parent);
    b.position.set(x + dx * s, y + dy * s, z + dz * s);
    b.scale.set(r * s, r * s, r * s * (round ? 0.9 : 0.75));
  }
}
function bush(parent, x, y, z, s = 1) {
  const blob = new THREE.IcosahedronGeometry(1, 1);
  for (const [dx, dy, dz, r] of [[0, 0, 0.4, 0.55], [0.5, 0.1, 0.32, 0.4], [-0.45, 0.05, 0.3, 0.42]]) {
    const b = mesh(blob, material('foliage'), parent);
    b.position.set(x + dx * s, y + dy * s, z + dz * s);
    b.scale.set(r * s, r * s, r * s * 0.85);
  }
}

// ---- Grass growing on the grass: tufts of blades, flowers in patches and a
// few small bushes (ArchiSim's "growth", drawn without contour lines).
const GROW_LAYER = 1;
let tuftGeo = null;
function tuftGeometry() {
  if (tuftGeo) return tuftGeo;
  const pos = [], col = [];
  const dark = new THREE.Color('#3f7a2c'), light = new THREE.Color('#a7cf5a');
  for (let i = 0; i < 7; i++) {
    const a = (i / 7) * Math.PI * 2 + Math.random() * 0.5;
    const r = 0.03 + Math.random() * 0.03;
    const h = 0.1 + Math.random() * 0.12;
    const lean = 0.03 + Math.random() * 0.06;
    const bx = Math.cos(a) * r, by = Math.sin(a) * r;
    const px = -Math.sin(a) * 0.012, py = Math.cos(a) * 0.012;
    pos.push(bx - px, by - py, 0, bx + px, by + py, 0, bx + Math.cos(a) * lean, by + Math.sin(a) * lean, h);
    for (const c of [dark, dark, light]) col.push(c.r, c.g, c.b);
  }
  tuftGeo = new THREE.BufferGeometry();
  tuftGeo.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  tuftGeo.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
  tuftGeo.computeVertexNormals();
  return tuftGeo;
}
const tuftMat = new THREE.MeshLambertMaterial({ vertexColors: true, side: THREE.DoubleSide });
const flowerMat = new THREE.MeshLambertMaterial({ color: 0xffffff });
const FLOWERS = ['#f4f1e6', '#f6d24a', '#e8697a', '#a98ad8', '#f29b45'];
function rand(seed) { let t = seed; return () => { t = (t * 16807) % 2147483647; return (t - 1) / 2147483646; }; }
function growGrass(parent, x0, x1, y0, y1, z, density, flowers, onlyBetween) {
  const r = rand(Math.floor(Math.abs(x0 * 131 + y0 * 977)) + 7);
  const area = (x1 - x0) * (y1 - y0);
  const n = Math.floor(area * density);
  const tufts = new THREE.InstancedMesh(tuftGeometry(), tuftMat, n);
  const m = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), v = new THREE.Vector3(), sc = new THREE.Vector3();
  let k = 0;
  for (let i = 0; i < n; i++) {
    const x = x0 + r() * (x1 - x0), y = y0 + r() * (y1 - y0);
    if (onlyBetween && !betweenFlags(x, y)) continue;
    e.set(0, 0, r() * 6.28); q.setFromEuler(e);
    const s = 0.7 + r() * 0.8;
    m.compose(v.set(x, y, z), q, sc.set(s, s, s));
    tufts.setMatrixAt(k++, m);
  }
  tufts.count = k;
  tufts.receiveShadow = true;
  tufts.layers.set(GROW_LAYER);
  parent.add(tufts);
  if (!flowers) return;
  // Flowers in patches: where a slow noise is high, of one or two colours.
  const heads = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(0.035, 0), flowerMat, Math.floor(area * flowers));
  let f = 0;
  const color = new THREE.Color();
  for (let i = 0; i < heads.count; i++) {
    const x = x0 + r() * (x1 - x0), y = y0 + r() * (y1 - y0);
    const patch = Math.sin(x * 0.7) * Math.cos(y * 0.9) + Math.sin(x * 0.23 + y * 0.31);
    if (patch < 0.9) continue;
    const pick = Math.floor((Math.sin(Math.floor(x / 3) * 12.9 + Math.floor(y / 3) * 78.2) * 0.5 + 0.5) * FLOWERS.length) % FLOWERS.length;
    color.set(FLOWERS[r() < 0.8 ? pick : (pick + 1) % FLOWERS.length]);
    m.compose(v.set(x, y, z + 0.14 + r() * 0.1), q.identity(), sc.set(1, 1, 0.8));
    heads.setMatrixAt(f, m);
    heads.setColorAt(f, color);
    f++;
  }
  heads.count = f;
  heads.layers.set(GROW_LAYER);
  parent.add(heads);
}
// Between the stepping stones of 'flagGrass' (as its shader's cells).
function betweenFlags() { return true; }

// ---- The street --------------------------------------------------------------

const street = new THREE.Group();
function buildStreet() {
  const S = street;
  // Ground: a plinth of rock with grass on top, the road and pavement in
  // front, water below.
  box(S, 'grass', 0, 6, -0.3, 64, 24, 0.3);
  box(S, 'strata', 0, 6, -7, 64, 24, 6.7);
  box(S, 'strata', 0, -10.5, -7, 64, 9, 6.45);
  box(S, 'slabs', 0, -4.5, -0.3, 64, 3, 0.42);
  box(S, 'concrete', 0, -6.08, -0.3, 64, 0.16, 0.38);                        // kerb
  box(S, 'asphalt', 0, -10.5, -0.55, 64, 9, 0.1);
  box(S, 'water', 0, -40, -6.3, 160, 50, 0.1);
  box(S, 'seabed', 0, -40, -12, 160, 50, 4.6);   // the sea floor, seen through
  box(S, 'grass', 0, 30, -0.3, 64, 24, 0.3);
  box(S, 'strata', 0, 30, -7, 64, 24, 6.7);

  // A low garden wall in front of the houses, and its coping.
  box(S, 'rubble', -6, -2.9, 0, 36, 0.35, 0.7);
  box(S, 'concrete', -6, -2.9, 0.7, 36.2, 0.45, 0.08);

  // 1 · Ochre brick house with a grey shingle roof and a lower wing.
  {
    const x = -20, fy = 0;
    box(S, 'brickOchre', x, 4, 0, 8, 8, 6.4);
    const r = gable(S, 'shingleGrey', x, 4, 6.4, 8, 8, 2.6, 0.35);
    r.gableTri('brickOchre');
    box(S, 'trimCream', x, fy - 0.03, 3.05, 8.04, 0.16, 0.18);              // string course
    for (const dx of [-2.3, 0, 2.3]) windowAt(S, x + dx, fy, 0.9, 1.3, 1.6, { head: 'trimCream' });
    for (const dx of [-2.3, 2.3]) windowAt(S, x + dx, fy, 3.8, 1.5, 1.7, {});
    windowAt(S, x, fy, 3.8, 0.8, 1.5, {});
    box(S, 'brickOchre', x + 6, 3, 0, 4, 6, 3.6);
    gable(S, 'shingleGrey', x + 6, 3, 3.6, 4, 6, 1.6, 0.3, true).gableTri('brickOchre');
    windowAt(S, x + 6, 0, 0.9, 2.4, 1.7, { head: 'trimCream' });
    box(S, 'brickRed', x - 2.6, 5, 7.5, 0.7, 0.7, 2.6);                      // chimney
  }
  // 2 · Blue clapboard house, white trims, chocolate shutters, dormers.
  {
    const x = -7.5, fy = 0;
    box(S, 'clapBlue', x, 4, 0, 7, 8, 6.2);
    gable(S, 'shingleGrey', x, 4, 6.2, 7, 8, 3.0, 0.4).gableTri('clapBlue');
    box(S, 'trimCream', x, fy - 0.05, 6.05, 7.3, 0.2, 0.25);
    for (const dx of [-2.2, 2.2]) windowAt(S, x + dx, fy, 0.9, 1.0, 1.7, { shutters: 'trimBrown' });
    doorAt(S, x, fy, 0.25, 1.1, 2.3);
    for (const dx of [-2.2, 0, 2.2]) windowAt(S, x + dx, fy, 3.6, 1.0, 1.6, { shutters: 'trimBrown', flowers: dx === 0 ? false : true });
    for (const dx of [-1.9, 1.9]) {
      box(S, 'clapBlue', x + dx, 1.4, 7.0, 1.4, 1.8, 1.4);
      gable(S, 'shingleGrey', x + dx, 1.4, 8.4, 1.4, 1.8, 0.8, 0.12, true).gableTri('clapBlue');
      windowAt(S, x + dx, 0.5, 7.25, 0.7, 0.9, {});
    }
    box(S, 'brickRed', x + 2.2, 6, 7.8, 0.6, 0.6, 2.2);
  }
  // 3 · Ashlar house, three storeys, green shutters, green roof.
  {
    const x = 1.5, fy = 0;
    box(S, 'ashlar', x, 4, 0, 6.5, 8, 9.2);
    gable(S, 'shingleGreen', x, 4, 9.2, 6.5, 8, 2.8, 0.3).gableTri('ashlar');
    doorAt(S, x - 1.2, fy, 0.25, 1.4, 2.6, 'trimGreen');
    windowAt(S, x + 1.6, fy, 0.9, 1.0, 1.8, { shutters: 'trimGreen' });
    for (const lvl of [3.6, 6.4]) for (const dx of [-2.1, 0, 2.1]) windowAt(S, x + dx, fy, lvl, 0.9, 1.6, { shutters: 'trimGreen', flowers: lvl < 4 && dx !== 0 });
    for (const lvl of [3.2, 6.0]) box(S, 'trimCream', x, fy - 0.04, lvl, 6.6, 0.14, 0.14);
    box(S, 'ashlar', x, 1.6, 10.0, 2.2, 2, 1.8);
    gable(S, 'shingleGreen', x, 1.6, 11.8, 2.2, 2, 1.1, 0.15, true).gableTri('ashlar');
    windowAt(S, x, 0.6, 10.3, 0.9, 1.1, {});
    box(S, 'brickRed', x - 2.4, 6.5, 10, 0.8, 0.7, 3.4);
  }
  // 4 · Yellow render house with a timber bay window and a scale roof.
  {
    const x = 8.6, fy = 0;
    box(S, 'stuccoYel', x, 4, 0, 5.4, 8, 7.4);
    gable(S, 'scales', x, 4, 7.4, 5.4, 8, 2.3, 0.3).gableTri('stuccoYel');
    doorAt(S, x - 1.3, fy, 0.25, 1.3, 2.5, 'trimBrown');
    windowAt(S, x + 1.4, fy, 0.9, 1.0, 1.8, {});
    box(S, 'planks', x + 0.2, -0.6, 3.4, 2.6, 1.2, 2.6);
    gable(S, 'zinc', x + 0.2, -0.6, 6.0, 2.6, 1.2, 0.9, 0.12, true);
    windowAt(S, x + 0.2, -1.2, 3.9, 1.8, 1.6, { flowers: true });
    windowAt(S, x - 1.7, fy, 3.9, 0.7, 1.6, {});
  }
  // 5 · The round tower: rubble and a copper cone.
  {
    const x = 15, y = 3.5;
    cylinder(S, 'rubble', x, y, 0, 2.7, 12.5, 48);
    cylinder(S, 'trimCream', x, y, 12.2, 3.0, 0.5, 48);
    cone(S, 'copper', x, y, 12.7, 3.25, 5.2, 48);
    for (const [a, z] of [[-1.9, 2.4], [-1.3, 6], [-1.75, 9.4]]) {
      const g = new THREE.Group();
      S.add(g);
      g.position.set(x + Math.cos(a) * 2.7, y + Math.sin(a) * 2.7, 0);
      g.rotation.z = a + Math.PI / 2;
      windowAt(g, 0, 0, z, 0.7, 1.3, {});
    }
  }
  // 6 · Andalusian whitewashed house, barrel tiles, green bars.
  {
    const x = 23.5, fy = 0;
    box(S, 'stuccoWhite', x, 4, 0, 7, 8, 6.0);
    box(S, 'stuccoPink', x, fy - 0.02, 0, 7.02, 0.12, 0.9);                    // painted plinth
    gable(S, 'barrel', x, 4, 6.0, 7, 8, 2.0, 0.35).gableTri('stuccoWhite');
    doorAt(S, x, fy, 0.25, 1.4, 2.6, 'trimGreen', 'stuccoPink');
    for (const dx of [-2.3, 2.3]) {
      windowAt(S, x + dx, fy, 0.9, 1.1, 1.8, { trim: 'trimGreen', shutters: 'trimGreen' });
      windowAt(S, x + dx, fy, 3.6, 1.1, 1.6, { trim: 'trimGreen', flowers: true });
    }
    windowAt(S, x, fy, 3.6, 1.1, 1.6, { trim: 'trimGreen' });
  }
  // A raised pool in the front yard of the ochre brick house.
  {
    const x = -20, y = -1.45, L = 6.4, W = 1.9, H = 0.55;
    box(S, 'gresite', x, y, 0, L, W, 0.06);
    for (const s of [-1, 1]) {
      box(S, 'gresite', x, y + s * (W / 2 - 0.06), 0, L, 0.12, H);
      box(S, 'gresite', x + s * (L / 2 - 0.06), y, 0, 0.12, W, H);
      box(S, 'slabs', x, y + s * (W / 2 - 0.02), H, L + 0.3, 0.3, 0.07);
      box(S, 'slabs', x + s * (L / 2 + 0.0), y, H, 0.3, W - 0.3, 0.07);
    }
    box(S, 'poolWater', x, y, H - 0.12, L - 0.24, W - 0.24, 0.02);
  }
  // Pavers in the side yard, a path, trees and hedges.
  box(S, 'pavers', 12.3, -1.5, -0.02, 2.4, 3, 0.04);
  box(S, 'planks', -13.8, 2, 0, 2.6, 5.5, 0.25);
  tree(S, -27, 5, 0, 1.2);
  tree(S, 29.5, 8, 0, 1.0, false);
  tree(S, 11.8, 9, 0, 1.3);
  for (const bx of [-24, -17, -11, -4, 4, 19.5]) bush(S, bx, -2.3, 0, 0.9);
  // The grass grows: tufts everywhere, flowers in patches, small bushes.
  growGrass(S, -32, 32, -2.7, 18, 0, 5, 1.2);
  for (const [bx, by] of [[-26, 9], [-14.5, 12], [5.5, 13], [13, 15], [27, 13], [-3, 15]]) bush(S, bx, by, 0, 0.6);
  // Street poles.
  for (const px of [-14, 5, 20]) {
    cylinder(S, 'pole', px, -3.3, 0, 0.13, 9, 10);
    box(S, 'pole', px, -3.3, 8.2, 2.4, 0.12, 0.12);
  }
}

// ---- The sample board --------------------------------------------------------

const board = new THREE.Group();
const labels = [];
function buildBoard() {
  const groups = ['Fachadas', 'Cubiertas', 'Alicatados', 'Suelos', 'Pavimentos', 'Tierras', 'Detalles'];
  box(board, 'concrete', 0, -7, -0.4, 40, 42, 0.4);
  groups.forEach((gname, gi) => {
    const keys = Object.keys(RANGE).filter((k) => RANGE[k].group === gname && !RANGE[k].hidden);
    const y = 9.5 - gi * 5.3;
    keys.forEach((k, i) => {
      const x = (i - (keys.length - 1) / 2) * 3.5;
      if (gname === 'Fachadas' || gname === 'Alicatados') box(board, k, x, y, 0, 2.6, 0.5, 2.6);
      else if (gname === 'Detalles') box(board, k, x, y - 0.6, 0, 1.8, 1.8, 1.8);
      else if (gname === 'Cubiertas') gable(board, k, x, y, 0, 2.4, 2.4, 1.2, 0.1, false, 0.1);
      else {
        box(board, k, x, y - 0.6, 0, 2.7, 2.7, 0.16);
        if (k === 'grass') growGrass(board, x - 1.3, x + 1.3, y - 1.9, y + 0.7, 0.16, 60, 7);
        if (k === 'flagGrass') growGrass(board, x - 1.3, x + 1.3, y - 1.9, y + 0.7, 0.16, 12, 0, 'flagGrass');
      }
      labels.push({ key: k, pos: new THREE.Vector3(x, y - 1.9, 0) });
    });
  });
}

// ---- Light -------------------------------------------------------------------

const scene = new THREE.Scene();
const hemi = new THREE.HemisphereLight(0xbfdcff, 0xd9c49a, 1.0);
hemi.position.set(0, 0, 1);
scene.add(hemi);
const sun = new THREE.DirectionalLight(0xfff0d8, 2.6);
sun.castShadow = true;
sun.shadow.mapSize.set(4096, 4096);
const sc = sun.shadow.camera;
sc.left = -45; sc.right = 45; sc.top = 45; sc.bottom = -45; sc.near = 1; sc.far = 220;
sun.shadow.bias = -0.0004;
sun.shadow.normalBias = 0.03;
scene.add(sun);
scene.add(sun.target);

const LIGHTS = {
  morning: { dir: [-0.55, -0.75, 0.62], sun: 0xfff1d8, sunI: 2.7, sky: 0xbcd8ff, ground: 0xd8c59c, hemiI: 1.05 },
  evening: { dir: [0.75, -0.55, 0.32], sun: 0xffc98a, sunI: 2.6, sky: 0x9fb4e6, ground: 0xd9a77c, hemiI: 0.95 },
};
function setLight(name) {
  const L = LIGHTS[name];
  const d = new THREE.Vector3(...L.dir).normalize();
  sun.position.copy(d.multiplyScalar(100));
  sun.color.set(L.sun); sun.intensity = L.sunI;
  hemi.color.set(L.sky); hemi.groundColor.set(L.ground); hemi.intensity = L.hemiI;
  post.uniforms.uEvening.value = name === 'evening' ? 1 : 0;
}

// ---- Post: contours and sky ----------------------------------------------------

let rtColor, rtNormal;
function makeTargets(w, h) {
  rtColor?.dispose(); rtNormal?.dispose();
  rtColor = new THREE.WebGLRenderTarget(w, h, { samples: 4, type: THREE.HalfFloatType });
  rtColor.depthTexture = new THREE.DepthTexture(w, h);
  rtNormal = new THREE.WebGLRenderTarget(w, h, { type: THREE.HalfFloatType });
  rtNormal.depthTexture = new THREE.DepthTexture(w, h);
}
const normalMat = new THREE.MeshNormalMaterial();
const post = new THREE.ShaderMaterial({
  uniforms: {
    tColor: { value: null }, tNormal: { value: null }, tDepth: { value: null },
    uTexel: { value: new THREE.Vector2() }, uNear: { value: camera.near }, uFar: { value: camera.far },
    uLines: { value: 1 }, uStyle: G.uStyle, uEvening: { value: 0 }, uPx: { value: 1 },
  },
  vertexShader: `varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }`,
  fragmentShader: /* glsl */ `
    uniform sampler2D tColor, tNormal, tDepth;
    uniform vec2 uTexel; uniform float uNear, uFar, uLines, uStyle, uEvening, uPx;
    varying vec2 vUv;
    float lin(float d) { float z = d * 2.0 - 1.0; return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear)); }
    vec3 nrm(vec2 p) { return texture2D(tNormal, p).xyz * 2.0 - 1.0; }
    float dep(vec2 p) { return lin(texture2D(tDepth, p).x); }
    vec3 lsrgb(vec3 c) { return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, c)); }
    void main() {
      vec4 c = texture2D(tColor, vUv);
      float d0raw = texture2D(tDepth, vUv).x;
      vec3 col = c.rgb;
      if (d0raw >= 1.0) {
        vec3 top = mix(vec3(0.17, 0.48, 0.86), vec3(0.32, 0.42, 0.72), uEvening);
        vec3 hor = mix(vec3(0.62, 0.84, 0.95), vec3(0.98, 0.78, 0.6), uEvening);
        col = mix(hor, top, smoothstep(0.35, 1.0, vUv.y));
        gl_FragColor = vec4(lsrgb(col), 1.0);
        return;
      }
      if (uLines > 0.5) {
        vec3 n0 = nrm(vUv); float d0 = dep(vUv);
        float e = 0.0;
        for (int i = 0; i < 4; i++) {
          vec2 o = (i == 0 ? vec2(1, 0) : i == 1 ? vec2(-1, 0) : i == 2 ? vec2(0, 1) : vec2(0, -1)) * uTexel * uPx;
          vec3 n1 = nrm(vUv + o); float d1 = dep(vUv + o);
          e = max(e, smoothstep(0.25, 0.45, 1.0 - dot(n0, n1)));
          e = max(e, smoothstep(0.03, 0.06, (d1 - d0) / d0));
        }
        e *= 1.0 - smoothstep(60.0, 160.0, d0);
        // Stylized: coloured ink, the surface's own colour darkened.
        vec3 ink = mix(vec3(0.16, 0.16, 0.18), col * vec3(0.32, 0.3, 0.36), uStyle);
        col = mix(col, ink, e * mix(0.75, 0.8, uStyle));
      }
      if (uStyle > 0.5) {
        // A painter's grade: a little more colour, warm lights, cool darks.
        float l = dot(col, vec3(0.2126, 0.7152, 0.0722));
        col = mix(vec3(l), col, 1.12);
        col *= mix(vec3(0.97, 0.98, 1.06), vec3(1.03, 1.0, 0.96), smoothstep(0.05, 0.5, l));
      }
      gl_FragColor = vec4(lsrgb(max(col, 0.0)), 1.0);
    }`,
  depthTest: false, depthWrite: false,
});
const quadScene = new THREE.Scene();
const quadCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
quadScene.add(new THREE.Mesh(new THREE.PlaneGeometry(2, 2), post));

// ---- Views, interface ------------------------------------------------------------

buildStreet();
buildBoard();
scene.add(street);
scene.add(board);

const VIEWS = {
  street: { show: street, pos: [-6, -50, 15], target: [1, 0, 2.5] },
  board: { show: board, pos: [0, -46, 44], target: [0, -7, 0] },
};
let view = 'street';
function setView(name) {
  view = name;
  street.visible = name === 'street';
  board.visible = name === 'board';
  const v = VIEWS[name];
  camera.position.set(...v.pos);
  controls.target.set(...v.target);
  controls.update();
  document.getElementById('labels').style.display = name === 'board' ? 'block' : 'none';
}

const labelBox = document.getElementById('labels');
for (const l of labels) {
  const el = document.createElement('div');
  el.className = 'label';
  const d = RANGE[l.key];
  el.innerHTML = `<i style="background:${d.base}"></i>${d.name}`;
  labelBox.appendChild(el);
  l.el = el;
}

function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
  const pr = renderer.getPixelRatio();
  makeTargets(Math.floor(w * pr), Math.floor(h * pr));
  post.uniforms.uTexel.value.set(1 / (w * pr), 1 / (h * pr));
  post.uniforms.uPx.value = Math.max(1, pr * 0.75);
}
window.addEventListener('resize', resize);

const clock = new THREE.Clock();
function frame() {
  G.uTime.value = clock.getElapsedTime();
  controls.update();
  renderer.setRenderTarget(rtColor);
  renderer.render(scene, camera);
  camera.layers.disable(GROW_LAYER);
  scene.overrideMaterial = normalMat;
  const bg = scene.background;
  renderer.setRenderTarget(rtNormal);
  renderer.render(scene, camera);
  scene.overrideMaterial = null;
  camera.layers.enable(GROW_LAYER);
  scene.background = bg;
  renderer.setRenderTarget(null);
  post.uniforms.tColor.value = rtColor.texture;
  post.uniforms.tNormal.value = rtNormal.texture;
  post.uniforms.tDepth.value = rtNormal.depthTexture;
  renderer.render(quadScene, quadCam);
  if (view === 'board') {
    const w = canvas.clientWidth, h = canvas.clientHeight;
    for (const l of labels) {
      const p = l.pos.clone().project(camera);
      l.el.style.transform = `translate(${(p.x * 0.5 + 0.5) * w}px, ${(-p.y * 0.5 + 0.5) * h}px) translate(-50%, 0)`;
    }
  }
  if (!STILL) requestAnimationFrame(frame);
}
const STILL = location.search.includes('still');

// Buttons
function seg(id, fn) {
  const el = document.getElementById(id);
  el.addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    el.querySelectorAll('button').forEach((x) => x.classList.toggle('on', x === b));
    fn(b.dataset.v);
  });
}
seg('segView', setView);
seg('segStyle', (v) => { G.uStyle.value = v === 'new' ? 1 : 0; });
seg('segLight', setLight);
seg('segLines', (v) => { post.uniforms.uLines.value = v === 'on' ? 1 : 0; });
seg('segRelief', (v) => { G.uRelief.value = v === 'on' ? 1 : 0; });

// Test hooks (screenshots).
window.__set = { render: () => frame(), view: setView, light: setLight, style: (s) => { G.uStyle.value = s; }, cam: (p, t) => { camera.position.set(...p); controls.target.set(...t); controls.update(); } };

setLight('morning');
resize();
setView('street');
frame();
