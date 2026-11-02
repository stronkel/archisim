// Stylized material range ("gama estilizada") for ArchiSim — prototype.
// Same method as ArchiSim's 3D view: every texture is drawn from the point's
// position on its face (no images, nothing to unwrap), so it ports to WGSL.
// Each material returns a colour and a small relief height (metres) that
// tilts the normal: bevelled bricks, overlapping boards, tile lips…

export const GLSL = /* glsl */ `
uniform int uKind;      // stylized material
uniform int uOld;       // ArchiSim's current texture (comparison)
uniform float uOldScale;
uniform vec3 uBase;
uniform vec3 uBase2;
uniform vec3 uJoint;
uniform vec4 uDims;     // piece sizes, metres (meaning per material)
uniform float uStyle;   // 1 stylized, 0 current
uniform float uRelief;  // relief on/off
uniform int uMap;       // 0 by face, 1 around an axis (towers, cones)
uniform vec3 uCenter;
uniform float uRadius;
varying vec3 vWorld;
varying vec3 vWN;

float hash2(vec2 p) {
  vec2 q = fract(p * vec2(123.34, 456.21));
  q += dot(q, q + 45.32);
  return fract(q.x * q.y);
}
float noise(vec2 p) {
  vec2 i = floor(p); vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  float a = hash2(i), b = hash2(i + vec2(1, 0)), c = hash2(i + vec2(0, 1)), d = hash2(i + vec2(1, 1));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
vec3 cells(vec2 p) {
  vec2 i = floor(p);
  float d1 = 9.0, d2 = 9.0, id = 0.0;
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 c = i + vec2(float(x), float(y));
    vec2 o = vec2(hash2(c), hash2(c + 17.0)) * 0.8 + 0.1;
    float d = length(c + o - p);
    if (d < d1) { d2 = d1; d1 = d; id = hash2(c + 3.0); } else if (d < d2) { d2 = d; }
  }
  return vec3(d1, d2, id);
}
// Distance (in the same units) from t to the nearest multiple of 'size'.
float edist(float t, float size) { float f = fract(t / size); return min(f, 1.0 - f) * size; }
// 0 in the joint, 1 on the piece, antialiased.
float onPiece(float e, float half_) { float w = max(fwidth(e), 1e-5); return smoothstep(half_ - w, half_ + w, e); }
// Four steps of a 0–1 value: a painter's few tones, not a photo's many.
float tones(float x, float n) { return floor(x * n) / max(n - 1.0, 1.0); }
float lum(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

struct Look { vec3 c; float h; };

// ---- The stylized range ---------------------------------------------------

// 1 Brick: chunky bricks, each a little different, bevelled, light joints.
Look brick(vec2 uv, vec3 base) {
  float bw = uDims.x, bh = uDims.y, j = uDims.z;
  float row = floor(uv.y / bh);
  float u = uv.x + mod(row, 2.0) * 0.5 * bw;
  float col = floor(u / bw);
  float e = min(edist(u, bw), edist(uv.y, bh));
  float id = hash2(vec2(col, row));
  vec3 c = base * (0.84 + 0.24 * tones(id, 4.0));
  if (id > 0.88) c = mix(c, base * vec3(0.78, 0.62, 0.58), 0.6);       // a burnt one
  if (id < 0.08) c = mix(c, base * vec3(1.18, 1.08, 0.95), 0.6);       // a pale one
  c *= 0.95 + 0.07 * noise(uv * 28.0);
  c *= 0.97 + 0.08 * (fract(uv.y / bh) - 0.3);                          // lighter top
  float m = onPiece(e, j * 0.5);
  float h = smoothstep(j * 0.5, j * 0.5 + uDims.w, e);
  return Look(mix(uJoint, c, m), sqrt(h) * 0.012);
}

// 2 Ashlar ("sillería"): big squared blocks with a chamfer.
Look ashlar(vec2 uv, vec3 base) {
  float bh = uDims.y, j = uDims.z;
  float row = floor(uv.y / bh);
  float bw = uDims.x * (0.8 + 0.4 * hash2(vec2(row, 7.0)));
  float u = uv.x + hash2(vec2(row, 3.0)) * bw;
  float col = floor(u / bw);
  float e = min(edist(u, bw), edist(uv.y, bh));
  float id = hash2(vec2(col, row));
  vec3 c = base * (0.88 + 0.16 * tones(id, 4.0));
  c = mix(c, uBase2, 0.25 * step(0.75, hash2(vec2(col + 5.0, row))));
  c *= 0.93 + 0.1 * noise(uv * 6.0) + 0.04 * noise(uv * 40.0);
  float m = onPiece(e, j * 0.5);
  float h = smoothstep(j * 0.5, j * 0.5 + uDims.w, e);
  return Look(mix(uJoint, c, m), h * 0.02);
}

// 3 Rubble ("mampostería"): rounded stones, pillowy, mortar between.
Look rubble(vec2 uv, vec3 base) {
  float s = uDims.x;
  vec3 k = cells(uv / s * vec2(1.0, 1.5));
  float e = (k.y - k.x) * 0.5 * s;
  vec3 c = base * (0.82 + 0.28 * tones(k.z, 5.0));
  c = mix(c, uBase2, step(0.8, k.z) * 0.5);
  c *= 0.94 + 0.08 * noise(uv * 18.0);
  float j = uDims.z;
  float m = onPiece(e, j * 0.5);
  float h = sqrt(smoothstep(j * 0.5, j * 0.5 + uDims.w, e));
  return Look(mix(uJoint, c, m), h * 0.035);
}

// 4 Painted render ("estuco pintado"): broad soft patches, a fine grain.
Look stucco(vec2 uv, vec3 base) {
  float n = noise(uv * 0.9) * 0.55 + noise(uv * 2.7) * 0.3 + noise(uv * 7.0) * 0.15;
  float t = smoothstep(0.35, 0.65, n);
  vec3 c = base * (0.95 + 0.08 * t);
  c *= 0.985 + 0.03 * noise(uv * 90.0);
  return Look(c, noise(uv * 45.0) * 0.0015);
}

// 5 Clapboard ("tablas solapadas"): each board leans out, a shadow under
// the lip of the one above.
Look clapboard(vec2 uv, vec3 base) {
  float w = uDims.x;
  float f = fract(uv.y / w);
  float row = floor(uv.y / w);
  float seg = floor(uv.x / uDims.y + hash2(vec2(row, 2.0)));
  float id = hash2(vec2(seg, row));
  vec3 c = base * (0.95 + 0.07 * tones(id, 3.0));
  c *= 0.97 + 0.05 * noise(vec2(uv.x * 2.0, uv.y * 70.0));
  c *= 1.0 - 0.32 * smoothstep(0.82, 1.0, f);                 // under the lip
  c *= 0.92 + 0.08 * smoothstep(0.0, 0.25, f);                // the lip's edge
  float butt = onPiece(edist(uv.x / uDims.y + hash2(vec2(row, 2.0)), 1.0) * uDims.y, 0.003);
  c = mix(c * 0.7, c, butt);
  float h = (1.0 - f) * 0.018 + 0.004 * smoothstep(0.0, 0.08, f);
  return Look(c, h);
}

// 6 Shingles ("tejuela", wood or slate): rows staggered, each its own tone.
Look shingles(vec2 uv, vec3 base) {
  float rh = uDims.y, sw = uDims.x;
  float row = floor(uv.y / rh);
  float f = fract(uv.y / rh);
  float u = uv.x / sw + hash2(vec2(row, 9.0));
  float col = floor(u);
  float id = hash2(vec2(col, row));
  vec3 c = base * (0.8 + 0.32 * tones(id, 4.0));
  c = mix(c, uBase2, step(0.85, id) * 0.6);
  c *= 0.95 + 0.08 * noise(vec2(uv.x * 5.0, uv.y * 30.0));
  c *= 1.0 - 0.38 * smoothstep(0.78, 1.0, f);
  float gap = onPiece(edist(u, 1.0) * sw, uDims.z * 0.5);
  c = mix(uJoint, c, gap);
  float h = (1.0 - f) * 0.022;
  return Look(c, h);
}

// 7 Fish scales ("escamas"): rounded lower edges.
Look scales(vec2 uv, vec3 base) {
  float rh = uDims.y, sw = uDims.x;
  float row = floor(uv.y / rh);
  float f = fract(uv.y / rh);
  float u = uv.x / sw + 0.5 * mod(row, 2.0);
  float x = fract(u) * 2.0 - 1.0;
  float bottom = 0.45 * (1.0 - sqrt(max(0.0, 1.0 - x * x)));
  if (f < bottom) {   // still the scale of the row below
    row -= 1.0; f += 1.0;
    u = uv.x / sw + 0.5 * mod(row, 2.0);
    x = fract(u) * 2.0 - 1.0;
    bottom = 0.45 * (1.0 - sqrt(max(0.0, 1.0 - x * x)));
  }
  float id = hash2(vec2(floor(u), row));
  vec3 c = base * (0.82 + 0.28 * tones(id, 4.0));
  float edge = smoothstep(0.0, 0.06, f - bottom);
  c *= mix(0.55, 1.0, edge);
  c *= 1.0 - 0.3 * smoothstep(0.85, 1.4, f);
  return Look(c, (1.4 - f) * 0.02 * edge);
}

// 8 Barrel tiles ("teja curva"): channels and caps down the slope, a lip at
// each course.
Look barrel(vec2 uv, vec3 base) {
  float x = uv.x / uDims.x;
  float course = floor(uv.y / uDims.y);
  float f = fract(uv.y / uDims.y);
  float p = sin(fract(x) * 3.14159);
  float id = hash2(vec2(floor(x), course));
  vec3 c = base * (0.84 + 0.26 * tones(id, 4.0));
  c = mix(c, uBase2, smoothstep(0.6, 0.95, noise(uv * 0.8)) * 0.45);  // weathered
  c *= 0.72 + 0.36 * p;
  c *= 1.0 - 0.35 * smoothstep(0.85, 1.0, f);
  return Look(c, p * 0.035 + (1.0 - f) * 0.012);
}

// 9 Standing seam ("chapa de junta alzada"): copper, zinc; patina.
Look seam(vec2 uv, vec3 base) {
  float w = uDims.x;
  float e = edist(uv.x, w);
  float id = hash2(vec2(floor(uv.x / w), 1.0));
  vec3 c = base * (0.95 + 0.08 * tones(id, 3.0));
  float pat = smoothstep(0.45, 0.8, noise(uv * vec2(0.8, 0.35)) * 0.7 + noise(uv * 4.0) * 0.3);
  c = mix(c, uBase2, pat * 0.6);
  float rib = 1.0 - smoothstep(0.0, 0.014, e);
  c = mix(c, c * 1.18, rib);
  c *= 1.0 - 0.18 * (1.0 - smoothstep(0.014, 0.04, e)) * (1.0 - rib);
  return Look(c, rib * 0.022);
}

// 10 Pavers ("adoquín"): rounded setts in courses, a few tones.
Look pavers(vec2 uv, vec3 base) {
  float bw = uDims.x, bh = uDims.y, j = uDims.z;
  float row = floor(uv.y / bh);
  float u = uv.x + mod(row, 2.0) * 0.5 * bw;
  float e = min(edist(u, bw), edist(uv.y, bh));
  float id = hash2(vec2(floor(u / bw), row));
  vec3 c = base * (0.82 + 0.26 * tones(id, 4.0));
  c = mix(c, uBase2, step(0.8, id) * 0.7);
  c *= 0.95 + 0.07 * noise(uv * 20.0);
  float m = onPiece(e, j * 0.5);
  float h = sqrt(smoothstep(j * 0.5, j * 0.5 + uDims.w, e));
  return Look(mix(uJoint, c, m), h * 0.015);
}

// 11 Slabs ("losa de acera"): big squares, thin light joints.
Look slabs(vec2 uv, vec3 base) {
  float s = uDims.x, j = uDims.z;
  float e = min(edist(uv.x, s), edist(uv.y, s));
  float id = hash2(floor(uv / s));
  vec3 c = base * (0.93 + 0.1 * tones(id, 3.0));
  c *= 0.96 + 0.06 * noise(uv * 3.0) + 0.03 * noise(uv * 50.0);
  float m = onPiece(e, j * 0.5);
  float h = smoothstep(j * 0.5, j * 0.5 + uDims.w, e);
  return Look(mix(uJoint, c, m), h * 0.006);
}

// 12 Grass: painted patches in three greens, and little strokes.
Look grass(vec2 uv, vec3 base) {
  float n = noise(uv * 0.35) * 0.5 + noise(uv * 1.1) * 0.3 + noise(uv * 3.0) * 0.2;
  float t = tones(smoothstep(0.25, 0.75, n), 3.0);
  vec3 c = mix(base * 0.8, mix(base * 1.12, uBase2, 0.5), t);
  float blade = smoothstep(0.72, 0.8, noise(vec2(uv.x * 22.0, uv.y * 6.0) + noise(uv * 3.0) * 2.0));
  c = mix(c, c * 1.18 + vec3(0.02, 0.03, 0.0), blade * 0.7);
  float dark = smoothstep(0.75, 0.82, noise(vec2(uv.x * 6.0 + 9.0, uv.y * 18.0)));
  c *= 1.0 - 0.15 * dark;
  return Look(c, 0.0);
}

// 13 Strata ("acantilado"): soft bands of rock, ledges and vertical streaks.
Look strata(vec2 uv, vec3 base) {
  float z = uv.y + 0.25 * noise(vec2(uv.x * 0.6, 3.0)) + 0.1 * noise(vec2(uv.x * 2.0, 1.0));
  float band = floor(z / uDims.y);
  float f = fract(z / uDims.y);
  float id = hash2(vec2(band, 5.0));
  vec3 c = mix(base, uBase2, tones(id, 3.0) * 0.7);
  c *= 0.9 + 0.12 * noise(vec2(uv.x * 2.2, uv.y * 0.25));          // streaks
  c *= 1.0 + 0.12 * smoothstep(0.85, 1.0, f);                      // ledge top
  c *= 1.0 - 0.18 * (1.0 - smoothstep(0.0, 0.12, f));              // under it
  float crack = smoothstep(0.03, 0.0, abs(noise(vec2(uv.x * 1.4, uv.y * 0.3)) - 0.5));
  c *= 1.0 - 0.25 * crack;
  return Look(c, f * 0.06);
}

// 14 Planks ("tarima"): boards end to end, painted grain.
Look planks(vec2 uv, vec3 base) {
  float w = uDims.x, len = uDims.y;
  float row = floor(uv.y / w);
  float u = uv.x / len + hash2(vec2(row, 1.0));
  float id = hash2(vec2(floor(u), row));
  vec3 c = base * (0.8 + 0.3 * tones(id, 4.0));
  float g = noise(vec2(uv.x * 1.2, uv.y * 30.0) + id * 10.0);
  c *= 0.9 + 0.14 * smoothstep(0.3, 0.7, g);
  float knot = 1.0 - smoothstep(0.0, 0.02, length((fract(vec2(u * 3.0, uv.y / w)) - 0.5) * vec2(len / 3.0, w)) - 0.008);
  c *= 1.0 - 0.35 * knot * step(0.8, hash2(vec2(floor(u * 3.0), row)));
  float e = min(edist(uv.y, w), edist(u, 1.0) * len);
  float m = onPiece(e, uDims.z * 0.5);
  return Look(mix(uJoint, c, m), smoothstep(uDims.z * 0.5, uDims.z * 0.5 + 0.004, e) * 0.003);
}

// 15 Asphalt: a blue-grey ground, worn patches and speckles.
Look asphalt(vec2 uv, vec3 base) {
  float n = noise(uv * 0.25) * 0.6 + noise(uv * 1.3) * 0.4;
  vec3 c = mix(base * 0.96, uBase2, tones(smoothstep(0.35, 0.8, n), 3.0) * 0.35);
  vec3 k = cells(uv * 30.0);
  c = mix(c, c * 1.35, (1.0 - smoothstep(0.05, 0.12, k.x)) * step(0.7, k.z));
  return Look(c, 0.0);
}

// 16 Concrete ("hormigón suave"): panels and a gentle mottle.
Look concrete(vec2 uv, vec3 base) {
  float e = min(edist(uv.x, uDims.x), edist(uv.y, uDims.y));
  float id = hash2(floor(uv / uDims.xy));
  vec3 c = base * (0.94 + 0.08 * tones(id, 3.0));
  c *= 0.95 + 0.07 * noise(uv * 1.5) + 0.03 * noise(uv * 30.0);
  float m = onPiece(e, uDims.z * 0.5);
  return Look(mix(c * 0.78, c, m), 0.0);
}

// 17 Water: bands of light.
Look water(vec2 uv, vec3 base) {
  float w = noise(uv * vec2(1.5, 0.5) + vec2(noise(uv * 0.8) * 2.0, 0.0));
  vec3 c = mix(base, uBase2, tones(smoothstep(0.45, 0.85, w), 3.0));
  return Look(c, 0.0);
}

// 18 Glass, painted: a diagonal sheen.
Look glass(vec2 uv, vec3 base) {
  float s = fract((uv.x * 0.6 + uv.y) * 0.55);
  float sheen = smoothstep(0.0, 0.03, s) * (1.0 - smoothstep(0.12, 0.15, s));
  float sheen2 = smoothstep(0.2, 0.22, s) * (1.0 - smoothstep(0.25, 0.27, s));
  vec3 c = mix(base, uBase2, max(sheen, sheen2 * 0.7) * 0.7);
  c = mix(c, uBase2 * 0.9, smoothstep(0.0, 1.0, fract(uv.y / 1.4)) * 0.25);
  return Look(c, 0.0);
}

// 19 Paint ("pintura lisa"): trims, frames, shutters.
Look paint(vec2 uv, vec3 base) {
  return Look(base * (0.98 + 0.03 * noise(uv * 30.0)), 0.0);
}

// 20 Leaves (trees, hedges, painted): blobs of light and shade.
Look foliage(vec2 uv, vec3 base) {
  vec3 k = cells(uv * 3.0);
  float t = tones(k.z, 3.0);
  vec3 c = mix(base * 0.82, uBase2, t * 0.6);
  c *= 0.9 + 0.15 * (1.0 - smoothstep(0.1, 0.5, k.x));
  return Look(c, (1.0 - smoothstep(0.0, 0.5, k.x)) * 0.03);
}

Look stylizedNear(vec2 uv, vec3 base);
// From afar the pieces melt into their mean colour (no shimmer), as a
// painter would draw a far wall.
Look stylized(vec2 uv, vec3 base) {
  Look l = stylizedNear(uv, base);
  float piece = uDims.y > 0.0 ? min(uDims.x > 0.0 ? uDims.x : 9.0, uDims.y) : uDims.x;
  if (piece > 0.0) {
    float px = length(fwidth(uv));
    float far = smoothstep(piece * 0.12, piece * 0.45, px);
    float jw = uDims.z / piece;
    vec3 mean = mix(base * 0.98, uJoint, clamp(jw * 1.5, 0.0, 0.5));
    l.c = mix(l.c, mean, far * 0.85);
    l.h *= 1.0 - far;
  }
  return l;
}
Look stylizedNear(vec2 uv, vec3 base) {
  if (uKind == 1) return brick(uv, base);
  if (uKind == 2) return ashlar(uv, base);
  if (uKind == 3) return rubble(uv, base);
  if (uKind == 4) return stucco(uv, base);
  if (uKind == 5) return clapboard(uv, base);
  if (uKind == 6) return shingles(uv, base);
  if (uKind == 7) return scales(uv, base);
  if (uKind == 8) return barrel(uv, base);
  if (uKind == 9) return seam(uv, base);
  if (uKind == 10) return pavers(uv, base);
  if (uKind == 11) return slabs(uv, base);
  if (uKind == 12) return grass(uv, base);
  if (uKind == 13) return strata(uv, base);
  if (uKind == 14) return planks(uv, base);
  if (uKind == 15) return asphalt(uv, base);
  if (uKind == 16) return concrete(uv, base);
  if (uKind == 17) return water(uv, base);
  if (uKind == 18) return glass(uv, base);
  if (uKind == 20) return foliage(uv, base);
  return paint(uv, base);
}

// ---- ArchiSim's current textures (shaders.wgsl, for comparison) ------------

float joint(float t, float width) { float f = fract(t); float e = min(f, 1.0 - f); return smoothstep(width * 0.5, width, e); }

vec3 current(vec3 base, vec3 world, vec3 n_in) {
  int kind = uOld;
  if (kind == 0) return base;
  vec3 n = abs(normalize(n_in));
  vec2 uv = world.xy;
  if (n.z < 0.7) uv = n.x > n.y ? world.yz : world.xz;
  vec2 p = uv / max(uOldScale, 0.01);
  vec3 mortar = mix(base, vec3(0.78, 0.76, 0.72), 0.75);
  if (kind == 1) {
    float row = floor(p.y * 3.8);
    vec2 q = vec2(p.x + 0.5 * mod(row, 2.0), p.y * 3.8);
    float id = hash2(vec2(floor(q.x), row));
    vec3 body = base * (0.82 + 0.3 * id) * (0.93 + 0.1 * noise(p * 9.0));
    return mix(mortar, body, joint(q.x, 0.06) * joint(q.y, 0.2));
  }
  if (kind == 2) {
    vec3 c = cells(p * 1.6);
    float edge = smoothstep(0.02, 0.09, c.y - c.x);
    vec3 body = base * (0.75 + 0.45 * c.z) * (0.9 + 0.15 * noise(p * 6.0));
    return mix(base * 0.45, body, edge);
  }
  if (kind == 3) {
    float id = hash2(floor(p));
    return mix(mortar, base * (0.95 + 0.08 * id), joint(p.x, 0.03) * joint(p.y, 0.03));
  }
  if (kind == 5) {
    float row = floor(p.y);
    float along = p.x / 8.0 + hash2(vec2(row, 1.0));
    float id = hash2(vec2(floor(along), row));
    float g = sin(p.y * 25.0 + noise(vec2(p.x * 0.6, p.y * 5.0)) * 5.0);
    vec3 body = base * (0.82 + 0.25 * id) * (0.93 + 0.06 * g);
    return mix(base * 0.4, body, joint(p.y, 0.06) * joint(along, 0.01));
  }
  if (kind == 6) {
    vec2 q = vec2(p.x, p.y * 2.0);
    float m = joint(q.x, 0.008) * joint(q.y, 0.012);
    vec3 body = base * (0.94 + 0.1 * noise(p * 12.0) + 0.04 * noise(p * 50.0));
    return body * mix(0.82, 1.0, m);
  }
  if (kind == 7) return base * (0.95 + 0.07 * noise(p * 60.0) + 0.04 * noise(p * 7.0));
  if (kind == 8) { float v = noise(p * vec2(18.0, 40.0)) * 0.6 + noise(p * 4.0) * 0.4; return base * (0.7 + 0.55 * v); }
  if (kind == 10) { float w = noise(p * vec2(3.0, 1.2) + vec2(noise(p * 2.0) * 2.0, 0.0)); return base * (0.88 + 0.25 * smoothstep(0.55, 0.8, w)); }
  if (kind == 11) { float r = 0.5 + 0.5 * sin(p.x * 6.2832); return base * (0.8 + 0.3 * r); }
  if (kind == 19) {
    vec3 c = cells(p * 5.0);
    float clod = smoothstep(0.0, 0.3, c.y - c.x);
    float v = 0.78 + 0.25 * noise(p * 3.0) + 0.12 * noise(p * 17.0);
    return base * v * (0.82 + 0.18 * clod);
  }
  return base;
}

// ---- Where a point is on its face -----------------------------------------
// Walls: along the wall and up it. Slopes (roofs): along the eaves and up
// the slope, so courses run level and lips face down. Floors: the plan.
// Around an axis (towers, cones): round the axis and up.
void frameOf(vec3 N, vec3 world, out vec2 uv, out vec3 t, out vec3 b) {
  vec3 up = vec3(0.0, 0.0, 1.0);
  if (abs(N.z) > 0.985) {
    t = vec3(1.0, 0.0, 0.0); b = vec3(0.0, 1.0, 0.0);
    uv = world.xy;
    return;
  }
  t = normalize(cross(up, N));
  b = cross(N, t);
  uv = vec2(dot(world, t), dot(world, b));
  if (uMap == 1) {
    vec2 d = world.xy - uCenter.xy;
    uv.x = atan(d.y, d.x) * uRadius;
    uv.y = dot(world - uCenter, b);
  }
}
`;
