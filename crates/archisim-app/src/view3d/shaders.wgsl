// ArchiSim 3D view.
//
// Pass 1 draws the opaque model, lit by a sun from the south-west and a
// sky light, and also writes each pixel's view-space normal and distance.
// Pass 2 draws the glass, see-through, over it.
// With a section box (Revit's), everything outside it is cut away: the
// faces seen through a cut are the inside of a closed solid, and they are
// painted as the flat cap of the cut, on the box face, in the material's
// colour.
// Pass 3 finds contour lines from those normals and distances (edges where
// the surface turns, silhouettes, and steps in depth) and draws them in ink.

struct Globals {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
    // xyz: direction towards the sun; w: unused.
    sun: vec4<f32>,
    // x: pixel step for contour detection; y: contour strength, negative
    // for thin lines (one pixel wide); z = 1
    // when something is selected (outlined); w = 1 while what is
    // selected is drawn stretched over the scene.
    edges: vec4<f32>,
    // Section box corners; clip_min.w = 1 when the box is on.
    clip_min: vec4<f32>,
    clip_max: vec4<f32>,
    // Eye position (w = 1 in perspective) and viewing direction.
    eye: vec4<f32>,
    forward: vec4<f32>,
    // The cut away walls (view3d/cutaway.rs): x the height everything is
    // cut at, y = 1 when on, z the height of the walls in front, w = 1
    // with walls in front.
    cut: vec4<f32>,
    // Walls in front: xy the horizontal direction towards the camera, z
    // where in front begins along it, w above which walls are upstairs.
    front: vec4<f32>,
    // The sun's view and projection, for the shadows (view3d/look.rs).
    sun_vp: mat4x4<f32>,
    // x = 1 with shadows; y: how far a point is moved off its face before
    // it is looked up in the shadow map; z: samples (1, 5 or 9); w = 1
    // with the shade of inner corners.
    look: vec4<f32>,
    // The line of sight through each pixel (the sky).
    inv_vp: mat4x4<f32>,
    // The light (view3d/look.rs, `Light::colours`): the sun's colour and
    // strength (w), the sky's above and at the horizon (w: strength), the
    // zenith, the haze and the far ground (w: its height).
    sun_color: vec4<f32>,
    sky_hi: vec4<f32>,
    sky_lo: vec4<f32>,
    zenith: vec4<f32>,
    haze: vec4<f32>,
    ground: vec4<f32>,
    // The grid on the ground: centre, reach and step (0: none).
    grid: vec4<f32>,
    // x = 1 with the sky; y: night (0…1); z, w: where the haze begins and
    // where it is thickest.
    atmos: vec4<f32>,
    // Lamps lit at night: position and reach; lamp_count.x of them; y: the
    // water's clock (seconds); z = 1 with the textures' relief.
    lamps: array<vec4<f32>, 16>,
    lamp_count: vec4<f32>,
    // What a cut shows: xyz a flat colour, w = 1 to use it; otherwise the
    // material's cut colour (each layer of a wall its own).
    caps: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

// The shadow map of the model's passes (the contour pass has its own
// picture at binding 0 of its group 1).
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var shadow_cmp: sampler_comparison;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    // xyz: the normal times 32767; w: the red and green of the cut colour
    // (scene.rs `Vertex::with_cap`).
    @location(1) normal: vec4<i32>,
    // sRGB colour and opacity.
    @location(2) color: vec4<f32>,
    // x: the bias plus three; y: 1 if selected (2 stretched); z: the
    // texture (0 none, 1 brick…); w: the blue of the cut colour.
    @location(3) misc: vec4<u32>,
    // The size of the texture's piece, metres.
    @location(4) tex_size: f32,
    // The element for the cut away walls: kind (0 plain, 1 wall, 2
    // object), a point of it in plan and its base.
    @location(5) cut: vec4<f32>,
    // Where the object stands (scene.rs, `Instance`): cos, sin of its
    // turn and where it goes in plan; its height and scale; its cut (x
    // below zero: the corner's); flags: selected (below zero: the
    // corner's), greyed.
    @location(6) i_place: vec4<f32>,
    @location(7) i_lift: vec4<f32>,
    @location(8) i_cut: vec4<f32>,
    @location(9) i_flags: vec4<f32>,
};

// A corner where it is, with what the shader needs of it.
struct Corner {
    pos: vec3<f32>,
    normal: vec3<f32>,
    color: vec4<f32>,
    bias: f32,
    tex: vec2<f32>,
    cut: vec4<f32>,
    picked: f32,
    // The material's cut colour, linear.
    cap: vec3<f32>,
};

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn corner(v: VertexIn) -> Corner {
    var c: Corner;
    let cs = v.i_place.x;
    let sn = v.i_place.y;
    let q = v.pos * v.i_lift.yzw;
    c.pos = vec3<f32>(
        cs * q.x - sn * q.y + v.i_place.z,
        sn * q.x + cs * q.y + v.i_place.w,
        q.z + v.i_lift.x,
    );
    let m = vec3<f32>(v.normal.xyz) / 32767.0 / v.i_lift.yzw;
    c.normal = normalize(vec3<f32>(cs * m.x - sn * m.y, sn * m.x + cs * m.y, m.z) + vec3<f32>(0.0, 0.0, 1e-9));
    var rgb = srgb_to_linear(v.color.rgb);
    if (v.i_flags.y > 0.5) {
        // Greyed by the disciplines, as a reference (as scene.rs `tone`).
        let lum = 0.3 * rgb.r + 0.59 * rgb.g + 0.11 * rgb.b;
        rgb = vec3<f32>(1.0 - (1.0 - lum) * 0.35);
    }
    c.color = vec4<f32>(rgb, v.color.a);
    c.bias = f32(v.misc.x) - 3.0;
    c.tex = vec2<f32>(f32(v.misc.z), v.tex_size);
    c.cut = select(v.cut, v.i_cut, v.i_cut.x > -0.5);
    c.picked = select(f32(v.misc.y), v.i_flags.x, v.i_flags.x > -0.5);
    let rg = u32(v.normal.w) & 0xffffu;
    var cap = srgb_to_linear(vec3<f32>(f32(rg & 255u), f32(rg >> 8u), f32(v.misc.w)) / 255.0);
    if (v.i_flags.y > 0.5) {
        let lum = 0.3 * cap.r + 0.59 * cap.g + 0.11 * cap.b;
        cap = vec3<f32>(1.0 - (1.0 - lum) * 0.35);
    }
    c.cap = cap;
    return c;
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) view_normal: vec3<f32>,
    @location(3) view_dist: f32,
    @location(4) world: vec3<f32>,
    // Foliage (bias 2): no contour lines between its own facets.
    @location(5) @interpolate(flat) soft: f32,
    @location(6) @interpolate(flat) tex: vec2<f32>,
    @location(7) @interpolate(flat) cut: vec4<f32>,
    @location(8) @interpolate(flat) picked: f32,
    @location(9) @interpolate(flat) cap: vec3<f32>,
};

@vertex
fn vs_model(input: VertexIn) -> VertexOut {
    let v = corner(input);
    var out: VertexOut;
    var vp = g.view * vec4<f32>(v.pos, 1.0);
    // A nudge along the line of sight, 0.02 % of the distance: invisible,
    // but it settles which element wins where two touch on an edge.
    vp.z = vp.z * (1.0 + min(v.bias, 1.0) * 0.0002);
    // 1: foliage; 2: what glows at night (lampshades, bulbs, flames); 3:
    // grass and flowers on the ground; 4: many small leaves (no contours).
    out.soft = select(select(select(select(0.0, 1.0, v.bias > 1.5), 2.0, v.bias > 2.5), 3.0, v.bias > 3.5), 4.0, v.bias > 4.5);
    // -1: a face between two layers of a wall, only seen through a cut.
    out.soft = select(out.soft, -1.0, v.bias < -1.5);
    // -2: the filled section of a cut object, on the face of the box.
    out.soft = select(out.soft, -2.0, v.bias < -2.5);
    out.clip = g.proj * vp;
    out.view_normal = (g.view * vec4<f32>(v.normal, 0.0)).xyz;
    out.view_dist = -vp.z;
    out.normal = v.normal;
    out.color = v.color;
    out.world = v.pos;
    out.tex = v.tex;
    out.cut = v.cut;
    out.picked = v.picked;
    out.cap = v.cap;
    return out;
}

fn boxed() -> bool {
    return g.clip_min.w > 0.5;
}

// The height above which an element is cut away by the cut away walls:
// 1e9 if it is not, -1e9 if all of it goes. As `CutRule::top_of`.
fn cut_top(c: vec4<f32>) -> f32 {
    if (g.cut.y < 0.5) {
        return 1e9;
    }
    if (c.x > 1.5) {
        if (c.w > g.cut.x + 1e-3) {
            return -1e9;
        }
        return 1e9;
    }
    if (c.x > 0.5 && g.cut.w > 0.5) {
        if (c.w > g.front.w) {
            return g.cut.x;
        }
        if (dot(g.front.xy, c.yz) > g.front.z) {
            return g.cut.z;
        }
        return 1e9;
    }
    return g.cut.x;
}

// The scene's own copy of what is being stretched (picked 1 while an
// overlay, picked 2, is drawn in its place: edges.w = 1).
fn stretched(picked: f32) -> bool {
    return g.edges.w > 0.5 && picked > 0.5 && picked < 1.5;
}

// What is kept of an element: the section box, lowered to its cut.
struct Keep {
    lo: vec3<f32>,
    hi: vec3<f32>,
    // Something of it is cut.
    on: bool,
};

fn keep(c: vec4<f32>) -> Keep {
    var k: Keep;
    k.lo = vec3<f32>(-1e9);
    k.hi = vec3<f32>(1e9);
    if (boxed()) {
        k.lo = g.clip_min.xyz;
        k.hi = g.clip_max.xyz;
    }
    let top = cut_top(c);
    k.hi.z = min(k.hi.z, top);
    k.on = boxed() || top < 1e8;
    return k;
}

// Outside what is kept (a hair of tolerance keeps faces on it).
fn outside(p: vec3<f32>, k: Keep) -> bool {
    let e = 1e-4;
    return any(p < k.lo - vec3<f32>(e)) || any(p > k.hi + vec3<f32>(e));
}

// Where the line of sight through `p` (inside the box) leaves it towards
// the eye: the cap of the cut. Returns the cap's normal (xyz) and its
// distance along the view (w).
fn cap(p: vec3<f32>, kept: Keep) -> vec4<f32> {
    var d = g.forward.xyz;
    if (g.eye.w > 0.5) {
        d = normalize(p - g.eye.xyz);
    }
    // Back towards the eye.
    let b = -d;
    var t = 1e9;
    var n = vec3<f32>(0.0, 0.0, 1.0);
    for (var k = 0; k < 3; k = k + 1) {
        if (abs(b[k]) > 1e-6) {
            var bound = kept.lo[k];
            var sgn = -1.0;
            if (b[k] > 0.0) {
                bound = kept.hi[k];
                sgn = 1.0;
            }
            let tk = (bound - p[k]) / b[k];
            if (tk < t) {
                t = tk;
                n = vec3<f32>(0.0);
                n[k] = sgn;
            }
        }
    }
    let q = p + b * max(t, 0.0);
    return vec4<f32>(n, dot(q - g.eye.xyz, g.forward.xyz));
}

// An illustration's light: a warm sun (its edge a little crisper than
// plain shading) and a cool sky, brighter from above; `lit` is how much
// of the sun reaches the point (0 in shadow).
fn shade(n_in: vec3<f32>, color: vec3<f32>, lit: f32) -> vec3<f32> {
    let n = normalize(n_in);
    let ndl = dot(n, normalize(g.sun.xyz));
    // The lit side wraps a little further round and its edge is crisper,
    // like a painted shadow line (the stylized range, 2026-11-02).
    let direct = (0.82 * smoothstep(-0.08, 0.42, ndl) + 0.18 * max(ndl, 0.0)) * lit;
    let up = 0.5 + 0.5 * n.z;
    let sky = mix(g.sky_lo.rgb * g.sky_lo.w, g.sky_hi.rgb * g.sky_hi.w, up) * (0.48 + 0.18 * up);
    let sun = g.sun_color.rgb * g.sun_color.w * direct;
    return color * (sky + sun);
}

// An illustration's colours: a little livelier than the materials'.
fn vivid(c: vec3<f32>) -> vec3<f32> {
    let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return max(mix(vec3<f32>(l), c, 1.22), vec3<f32>(0.0));
}

// The warm light of the lamps lit at night on a point of a face (no
// shadows). A fixed number of turns, only sums: Windows' compiler unrolls
// it.
fn lamplight(world: vec3<f32>, n_in: vec3<f32>) -> vec3<f32> {
    let n = normalize(n_in);
    var sum = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let l = g.lamps[i];
        let to = l.xyz - world;
        let d = max(length(to), 0.05);
        let fall = clamp(1.0 - d / max(l.w, 0.1), 0.0, 1.0);
        // Only faces turned to the lamp: not the far side of its walls.
        let face = max(dot(n, to / d), 0.0);
        let on = select(0.0, 1.0, f32(i) < g.lamp_count.x);
        sum = sum + on * fall * fall * face;
    }
    return vec3<f32>(1.0, 0.72, 0.42) * 1.25 * sum * g.atmos.y;
}

// The haze between the eye and a point `dist` away.
fn hazed(color: vec3<f32>, dist: f32) -> vec3<f32> {
    if (g.atmos.x < 0.5) {
        return color;
    }
    let f = smoothstep(g.atmos.z, g.atmos.w, dist) * 0.75;
    return mix(color, g.haze.rgb, f);
}

// How much of the sun reaches a point of a face looking along `n`: 1 in
// the sun, 0 in shadow, softened over a few samples. No loops (Windows'
// shader compiler).
fn sunlit(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (g.look.x < 0.5) {
        return 1.0;
    }
    let q = g.sun_vp * vec4<f32>(world + normalize(n) * g.look.y, 1.0);
    let ndc = q.xyz / q.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let d = ndc.z;
    let px = 1.0 / f32(textureDimensions(shadow_map).x);
    var sum = textureSampleCompareLevel(shadow_map, shadow_cmp, uv, d);
    var count = 1.0;
    if (g.look.z > 1.5) {
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(px, px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-px, px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(px, -px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-px, -px), d);
        count = count + 4.0;
    }
    if (g.look.z > 5.5) {
        let r = 2.0 * px;
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(r, 0.0), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-r, 0.0), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(0.0, r), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(0.0, -r), d);
        count = count + 4.0;
    }
    // Outside the map (beyond the scene), in the sun.
    let inside = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && d <= 1.0;
    return select(1.0, sum / count, inside);
}

// The level of detail of the textures' pictures at this pixel (set at the
// start of the fragment; only the pictures' textures use it).
var<private> tex_level: f32;
// Metres a pixel covers at this point, and whether the face is curved (a
// round tower), both set at the start of the fragment (derivatives only
// in uniform control flow).
var<private> tex_px: f32;
var<private> tex_curved: bool;
// The relief of the texture at this point, metres off the face (the
// stylized range: a brick's bevel, a board's lip); 0 for the others.
var<private> tex_height: f32;

// Roof coverings: their courses run along the eaves and up the slope on
// any pitch (`TextureKind::on_slopes`, and the ribbed sheet).
fn tex_on_slopes(kind: i32) -> bool {
    return kind == 11 || (kind >= 23 && kind <= 26);
}

// Where a point of a face is on its texture, metres: on faces looking up,
// across the plan (roof coverings only when flat); on curved faces, along
// x or y and up; on the others, along the face and up it (up the slope on
// a roof). As `archisim_render::textures::texture_point`.
fn tex_uv(world: vec3<f32>, n_in: vec3<f32>, kind: i32) -> vec2<f32> {
    let n = normalize(n_in);
    let a = abs(n);
    if (a.z > 0.985 || (a.z >= 0.7 && !tex_on_slopes(kind))) {
        return world.xy;
    }
    if (tex_curved) {
        if (a.x > a.y) {
            return world.yz;
        }
        return world.xz;
    }
    let t = normalize(vec3<f32>(-n.y, n.x, 0.0));
    let b = cross(n, t);
    return vec2<f32>(dot(world, t), dot(world, b));
}

// Sets what the textures need of the derivatives (call first thing in a
// fragment).
fn tex_begin(world: vec3<f32>, n_in: vec3<f32>) {
    let dw = length(fwidth(world));
    tex_px = max(dw * 0.7, 1e-5);
    tex_curved = length(fwidth(n_in)) > 0.05 * dw;
    tex_height = 0.0;
}

// The normal tilted by the texture's relief (`tex_height`), with the
// relief on (`g.lamp_count.z`); fades out where a pixel covers more than
// a few centimetres. Call in uniform control flow, after `textured`.
fn relief_normal(world: vec3<f32>, n_in: vec3<f32>) -> vec3<f32> {
    let n = normalize(n_in);
    let dpx = dpdx(world);
    let dpy = dpdy(world);
    let hx = dpdx(tex_height);
    let hy = dpdy(tex_height);
    if (g.lamp_count.z < 0.5) {
        return n;
    }
    let r1 = cross(dpy, n);
    let r2 = cross(n, dpx);
    let det = dot(dpx, r1);
    var grad = sign(det) * (hx * r1 + hy * r2);
    // No more than some 55° of tilt: a step in the relief (a lip) would
    // otherwise make a black line on the pixels across it.
    let most = 1.4 * abs(det);
    let gl = length(grad);
    if (gl > most) {
        grad = grad * (most / gl);
    }
    let fade = 1.0 - smoothstep(0.006, 0.03, tex_px);
    let bent = abs(det) * n - grad * fade;
    if (dot(bent, bent) < 1e-20) {
        return n;
    }
    return normalize(bent);
}

// Water, seen through: little when looked into, nearly opaque at a
// glancing look; the sea more than a pool.
fn water_alpha(world: vec3<f32>, n_in: vec3<f32>, kind: i32, alpha: f32) -> f32 {
    if (kind != 10 && kind != 30) {
        return alpha;
    }
    var d = g.forward.xyz;
    if (g.eye.w > 0.5) {
        d = normalize(world - g.eye.xyz);
    }
    let fres = pow(1.0 - abs(dot(normalize(n_in), d)), 3.0);
    let lo = select(0.32, 0.74, kind == 30);
    return mix(lo, 0.96, fres);
}


// ---- Textures -------------------------------------------------------------
//
// Drawn from each point's position on its face (no images, nothing to
// unwrap): on faces looking up, across the plan; on the others, along the
// face and up it. They change the colour a little, around the material's.

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Distance to the nearest and second nearest cell centres, and the
// nearest cell's id (stones, gravel).
fn cells(p: vec2<f32>) -> vec3<f32> {
    let i = floor(p);
    var d1 = 9.0;
    var d2 = 9.0;
    var id = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let c = i + vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(hash2(c), hash2(c + 17.0)) * 0.8 + 0.1;
            let d = length(c + o - p);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id = hash2(c + 3.0);
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return vec3<f32>(d1, d2, id);
}

// How far into a joint a coordinate is (0 in the joint, 1 away from it).
fn joint(t: f32, width: f32) -> f32 {
    let f = fract(t);
    let e = min(f, 1.0 - f);
    return smoothstep(width * 0.5, width, e);
}

fn tex_lod(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> f32 {
    return 0.0;
}

// Distance from `t` to the nearest multiple of `size` (same units).
fn edist(t: f32, size: f32) -> f32 {
    let f = fract(t / size);
    return min(f, 1.0 - f) * size;
}

// 0 in a joint `half` wide each side of its line, 1 on the piece, over
// `w` (a pixel, in the texture's pieces).
fn on_piece(e: f32, half: f32, w: f32) -> f32 {
    // Where a pixel is wider than the joint, the joint is only as dark as
    // the share of the pixel it covers (no thick lines from afar).
    let s = smoothstep(half - w, half + w, e);
    return 1.0 - (1.0 - s) * clamp(half / w, 0.0, 1.0);
}

// A 0–1 value in `n` steps: a painter's few tones, not a photo's many.
fn tones(x: f32, n: f32) -> f32 {
    return floor(min(x, 0.999) * n) / max(n - 1.0, 1.0);
}

// Odd rows (row parity without a remainder of reals).
fn odd(row: f32) -> f32 {
    return step(0.25, fract(row * 0.5));
}

// From afar, the pieces melt into their mean colour and lose their
// relief: no shimmer where a piece is a few pixels (`piece` in pieces,
// `w` a pixel in pieces).
fn far_off(c: vec3<f32>, mean: vec3<f32>, piece: f32, w: f32) -> vec3<f32> {
    let f = smoothstep(piece * 0.05, piece * 0.25, w);
    tex_height = tex_height * (1.0 - f);
    return mix(c, mean, f * 0.85);
}

// Grass at `q` (metres), its colour `base`, a pixel `wm` metres: soft
// patches in three greens, darker clumps of clover and, near, little
// strokes of blades seen from above, each its own slant and tone.
fn grass_at(q: vec2<f32>, base: vec3<f32>, wm: f32) -> vec3<f32> {
    let n = noise(q * 0.3) * 0.55 + noise(q * 0.9) * 0.3 + noise(q * 2.6) * 0.15;
    var c = mix(base * 0.86, base * 0.75 + vec3<f32>(0.13, 0.19, 0.04), tones(smoothstep(0.3, 0.7, n), 3.0));
    let k3 = cells(q * 1.4);
    c = c * (1.0 - 0.12 * (1.0 - smoothstep(0.15, 0.4, k3.x)) * step(0.72, k3.z));
    let near = 1.0 - smoothstep(0.004, 0.012, wm);
    if (near <= 0.0) {
        return c;
    }
    let gq = q * 9.0;
    let i = floor(gq);
    var stroke = 0.0;
    var tone = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let cc = i + vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(hash2(cc), hash2(cc + 17.0));
            let a = (hash2(cc + 5.0) - 0.5) * 1.4;
            let d = vec2<f32>(sin(a), cos(a));
            let rel = gq - (cc + o);
            let along = clamp(dot(rel, d), -0.32, 0.32);
            let s1 = 1.0 - smoothstep(0.035, 0.07, length(rel - d * along));
            if (s1 > stroke) {
                stroke = s1;
                tone = hash2(cc + 9.0);
            }
        }
    }
    let lit = select(c * 0.78, c * 1.22 + vec3<f32>(0.03, 0.04, 0.0), tone > 0.5);
    return mix(c, lit, stroke * 0.75 * near);
}

// A row of strokes on the sea: lines a `width` of their spacing wide,
// `freq` lines a metre, moving `speed` lines a second, broken into dashes
// where a noise is over `dashes`.
fn wave_strokes(q: vec2<f32>, freq: f32, speed: f32, t: f32, width: f32, dashes: f32) -> f32 {
    let y = q.y * freq + noise(q * vec2<f32>(0.05, 0.2)) * 1.5 - t * speed;
    let li = floor(y);
    let f = fract(y);
    let line = smoothstep(0.0, width * 0.4, f) * (1.0 - smoothstep(width * 0.6, width, f));
    let dash = smoothstep(dashes, dashes + 0.12, noise(vec2<f32>(q.x * 0.4 + li * 13.0 + t * 0.25, li * 1.7)));
    return line * dash;
}

// Setts in courses half a sett high, every other course moved half a
// sett (`q` in setts).
fn setts(q: vec2<f32>, base: vec3<f32>, w: f32) -> vec3<f32> {
    let row = floor(q.y * 2.0);
    let u = q.x + 0.5 * odd(row);
    let e = min(edist(u, 1.0), edist(q.y, 0.5));
    let id = hash2(vec2<f32>(floor(u), row));
    var c = base * (0.82 + 0.26 * tones(id, 4.0)) * select(1.0, 0.85, id > 0.8) * (0.95 + 0.07 * noise(q * 4.0));
    c = mix(base * 0.6, c, on_piece(e, 0.025, w));
    tex_height = sqrt(smoothstep(0.025, 0.085, e)) * 0.075 * (tex_px / max(w, 1e-4));
    return far_off(c, base * 0.85, 0.5, w);
}

// The piece of the opus romanum's module (8 × 6 units) a point `l` of it
// falls in: x, y, width and height.
fn opus_piece(l: vec2<f32>) -> vec4<f32> {
    if (l.x < 4.0 && l.y < 4.0) {
        return vec4<f32>(0.0, 0.0, 4.0, 4.0);
    }
    if (l.x >= 4.0 && l.y < 2.0) {
        return vec4<f32>(4.0, 0.0, 4.0, 2.0);
    }
    if (l.x >= 4.0 && l.x < 6.0 && l.y < 4.0) {
        return vec4<f32>(4.0, 2.0, 2.0, 2.0);
    }
    if (l.x >= 6.0) {
        return vec4<f32>(6.0, 2.0, 2.0, 4.0);
    }
    if (l.x < 2.0) {
        return vec4<f32>(0.0, 4.0, 2.0, 2.0);
    }
    if (l.x < 4.0) {
        return vec4<f32>(2.0, 4.0, 2.0, 2.0);
    }
    return vec4<f32>(4.0, 4.0, 2.0, 2.0);
}

fn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {
    let kind = i32(tex.x + 0.5);
    if (kind == 0) {
        return base;
    }
    let s = max(tex.y, 0.01);
    let p = tex_uv(world, n_in, kind) / s;
    // A pixel, in pieces.
    let w = max(tex_px / s, 1e-4);
    let t = g.lamp_count.y;
    let mortar = mix(base, vec3<f32>(0.82, 0.79, 0.73), 0.78);
    switch kind {
        // Brick: chunky bricks, four courses a brick's length, each brick
        // one of a few tones (a burnt one, a pale one), bevelled, light
        // joints.
        case 1: {
            let row = floor(p.y * 4.0);
            let u = p.x + 0.5 * odd(row);
            let e = min(edist(u, 1.0), edist(p.y, 0.25));
            let id = hash2(vec2<f32>(floor(u), row));
            var k = 0.84 + 0.24 * tones(id, 4.0);
            k = k * select(1.0, 0.7, id > 0.88) * select(1.0, 1.12, id < 0.08);
            k = k * (0.95 + 0.07 * noise(p * 7.0)) * (0.97 + 0.08 * (fract(p.y * 4.0) - 0.3));
            let c = mix(mortar, base * k, on_piece(e, 0.025, w));
            tex_height = sqrt(smoothstep(0.025, 0.075, e)) * 0.048 * s;
            return far_off(c, mix(base * 0.95, mortar, 0.25), 0.25, w);
        }
        // Stone (rubble, "mampostería"): rounded stones, pillowy, a light
        // mortar between them.
        case 2: {
            let k3 = cells(p * vec2<f32>(1.25, 1.9));
            let e = (k3.y - k3.x) * 0.4;
            var c = base * (0.82 + 0.28 * tones(k3.z, 5.0)) * (0.94 + 0.08 * noise(p * 7.0));
            c = mix(c, base * 0.86 + vec3<f32>(0.06, 0.05, 0.02), select(0.0, 0.5, k3.z > 0.8));
            let j = mix(base, vec3<f32>(0.86, 0.84, 0.79), 0.6);
            c = mix(j, c, on_piece(e, 0.025, w));
            tex_height = sqrt(smoothstep(0.025, 0.15, e)) * 0.09 * s;
            return far_off(c, mix(base * 0.95, j, 0.15), 0.6, w);
        }
        // Tiles and slabs: squares a piece wide, thin light joints, each
        // its tone.
        case 3: {
            let e = min(edist(p.x, 1.0), edist(p.y, 1.0));
            let id = hash2(floor(p));
            var c = base * (0.93 + 0.1 * tones(id, 3.0)) * (0.96 + 0.06 * noise(p * 1.8) + 0.03 * noise(p * 30.0));
            let j = mix(base, vec3<f32>(0.93, 0.91, 0.87), 0.7);
            c = mix(j, c, on_piece(e, 0.008, w));
            tex_height = smoothstep(0.008, 0.02, e) * 0.01 * s;
            return far_off(c, base * 0.97, 1.0, w);
        }
        // Wood: grain.
        case 4: {
            let gr = sin(p.y * 40.0 + noise(p * vec2<f32>(1.5, 8.0)) * 6.0);
            return base * (0.86 + 0.1 * gr + 0.08 * noise(p * vec2<f32>(0.5, 30.0)));
        }
        // Boards (decking, "tarima"): a board a piece wide and eight
        // long, a few tones, painted grain, dark joints.
        case 5: {
            let row = floor(p.y);
            let along = p.x / 8.0 + hash2(vec2<f32>(row, 1.0));
            let id = hash2(vec2<f32>(floor(along), row));
            let gr = noise(vec2<f32>(p.x * 0.17, p.y * 4.2) + id * 10.0);
            var c = base * (0.8 + 0.3 * tones(id, 4.0)) * (0.9 + 0.14 * smoothstep(0.3, 0.7, gr));
            let e = min(edist(p.y, 1.0), edist(along, 1.0) * 8.0);
            c = mix(base * 0.45, c, on_piece(e, 0.015, w));
            tex_height = smoothstep(0.015, 0.045, e) * 0.02 * s;
            return far_off(c, base * 0.93, 1.0, w);
        }
        // Board-marked concrete: panels, their joints and tie holes.
        case 6: {
            let q = vec2<f32>(p.x, p.y * 2.0);
            let m = joint(q.x, 0.008) * joint(q.y, 0.012);
            let f = fract(q);
            let hole = smoothstep(0.012, 0.022, min(length(f - vec2<f32>(0.15, 0.25)), length(f - vec2<f32>(0.85, 0.25))));
            let id = hash2(floor(q));
            let body = base * (0.94 + 0.06 * tones(id, 3.0) + 0.08 * noise(p * 12.0) + 0.03 * noise(p * 50.0));
            return body * mix(0.82, 1.0, m) * mix(0.55, 1.0, hole);
        }
        // Render ("estuco"): broad soft patches, as painted, a fine grain.
        case 7: {
            let n = noise(p * 0.45) * 0.55 + noise(p * 1.35) * 0.3 + noise(p * 3.5) * 0.15;
            tex_height = noise(p * 22.0) * 0.003 * s;
            return base * (0.95 + 0.08 * smoothstep(0.35, 0.65, n)) * (0.985 + 0.03 * noise(p * 45.0));
        }
        // Grass: soft painted patches, darker clumps of clover and little
        // strokes of blades seen from above.
        case 8: {
            return grass_at(p * s, base, w * s);
        }
        // Gravel ("grava"): fine stones in three tones, domed.
        case 9: {
            let k3 = cells(p * 3.0);
            let dome = 1.0 - smoothstep(0.0, 0.6, k3.x);
            var c = mix(base * 0.78, base * 0.9 + vec3<f32>(0.05, 0.04, 0.03), tones(k3.z, 3.0) * 0.7) * (0.8 + 0.3 * dome);
            c = mix(base * 0.55, c, smoothstep(0.0, 0.12, k3.y - k3.x));
            tex_height = dome * 0.02 * s;
            return far_off(c, base * 0.8, 0.3, w);
        }
        // Water of a pool: clear, with the bright web of caustics wandering
        // over it.
        case 10: {
            let q = p * 1.6 + vec2<f32>(noise(p * 0.9 + t * 0.25), noise(p * 0.9 - t * 0.2 + 5.0)) * 0.9;
            let k3 = cells(q + vec2<f32>(t * 0.08, t * 0.05));
            let web = 1.0 - smoothstep(0.0, 0.09, k3.y - k3.x);
            tex_height = (sin(p.x * 7.0 + t * 2.0) + sin(p.y * 6.0 - t * 1.7)) * 0.004 * s;
            return mix(base, vec3<f32>(0.72, 0.95, 1.0), web * 0.85) * (0.96 + 0.06 * noise(p * 2.0 + t * 0.3));
        }
        // Sheet metal: corrugations along the face (down the slope on a
        // roof).
        case 11: {
            let r = 0.5 + 0.5 * sin(p.x * 6.2832);
            tex_height = r * 0.25 * s;
            return base * (0.8 + 0.3 * r);
        }
        // Linen: threads both ways and slubs along them, faint up close,
        // a soft mottle from afar.
        case 12: {
            let warp = 0.5 + 0.5 * sin(p.x * 150.0);
            let weft = 0.5 + 0.5 * sin(p.y * 150.0);
            let slub = noise(vec2<f32>(p.x * 4.0, p.y * 60.0)) * 0.6 + noise(vec2<f32>(p.x * 60.0, p.y * 4.0)) * 0.4;
            return base * (0.9 + 0.035 * (warp + weft) + 0.08 * slub + 0.04 * noise(p * 3.0));
        }
        // Bouclé: little loops, each catching the light.
        case 13: {
            let c = cells(p * 14.0);
            let loop_ = 1.0 - smoothstep(0.0, 0.45, c.x);
            return base * (0.84 + 0.2 * loop_ + 0.06 * c.z);
        }
        // Knit: rows of V-shaped stitches.
        case 14: {
            let q = vec2<f32>(p.x * 10.0, p.y * 7.0);
            let col = fract(q.x);
            let v = abs(col - 0.5) * 2.0;
            let row = fract(q.y + v * 0.5);
            let stitch = smoothstep(0.0, 0.35, row) * (1.0 - smoothstep(0.65, 1.0, row));
            let gap = smoothstep(0.0, 0.12, min(col, 1.0 - col));
            return base * (0.78 + 0.22 * stitch * gap + 0.05 * noise(p * 2.0));
        }
        // Velvet: lighter where the face turns away from the eye, a soft
        // pile.
        case 15: {
            var d = g.forward.xyz;
            if (g.eye.w > 0.5) {
                d = normalize(world - g.eye.xyz);
            }
            let rim = 1.0 - abs(dot(normalize(n_in), d));
            let pile = noise(p * 12.0) * 0.5 + noise(p * 3.0) * 0.5;
            return base * (0.82 + 0.45 * rim * rim + 0.07 * pile);
        }
        // Wicker: strands over and under in squares.
        case 16: {
            let q = p * 7.0;
            let c = floor(q);
            let f = fract(q);
            let across = (c.x + c.y) % 2.0;
            let tt = select(f.x, f.y, across > 0.5);
            let strand = sin(tt * 3.1416);
            let gap = smoothstep(0.0, 0.08, min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)));
            return base * (0.55 + 0.5 * strand * gap + 0.05 * hash2(c));
        }
        // Terrazzo: chips of light, dark and warm stone in the ground.
        case 17: {
            let c = cells(p * 9.0);
            let chip = 1.0 - smoothstep(0.18, 0.24, c.x);
            var tint = vec3<f32>(0.32, 0.3, 0.3);
            if (c.z > 0.66) {
                tint = vec3<f32>(0.75, 0.45, 0.35);
            } else if (c.z > 0.33) {
                tint = vec3<f32>(0.95, 0.93, 0.9);
            }
            let ground = base * (0.96 + 0.05 * noise(p * 20.0));
            return mix(ground, tint, chip * 0.85);
        }
        // Marble: grey veins wandering across a white ground.
        case 18: {
            let wv = p.x * 1.3 + p.y * 0.7 + noise(p * 1.5) * 3.0 + noise(p * 5.0) * 0.8;
            let vein = 1.0 - smoothstep(0.0, 0.06, abs(sin(wv * 3.1416)));
            let fine = 1.0 - smoothstep(0.0, 0.03, abs(sin((wv + noise(p * 9.0)) * 9.0)));
            return base * (0.97 + 0.03 * noise(p * 4.0)) * (1.0 - 0.32 * vein - 0.12 * fine);
        }
        // Soil: a mottle with clods and, here and there, a light stone.
        case 19: {
            let c = cells(p * 5.0);
            let clod = smoothstep(0.0, 0.3, c.y - c.x);
            let v = 0.78 + 0.25 * noise(p * 3.0) + 0.12 * noise(p * 17.0);
            let stone = (1.0 - smoothstep(0.1, 0.16, c.x)) * step(0.85, c.z);
            return mix(base * v * (0.82 + 0.18 * clod), base * 1.3 + vec3<f32>(0.08, 0.08, 0.07), stone);
        }
        // Fallen leaves: one in each cell, ochre, rust or brown, over the
        // darker ground.
        case 20: {
            let c = cells(p * 2.5);
            let leaf = 1.0 - smoothstep(0.26, 0.33, c.x);
            var tint = vec3<f32>(0.5, 0.31, 0.14);
            if (c.z > 0.66) {
                tint = vec3<f32>(0.66, 0.47, 0.18);
            } else if (c.z > 0.33) {
                tint = vec3<f32>(0.4, 0.2, 0.1);
            }
            let ground = base * (0.72 + 0.25 * noise(p * 6.0));
            return mix(ground, tint, leaf * 0.85);
        }
        // Ashlar ("sillería"): big squared blocks, courses half a block
        // high, each course its block length; chamfered.
        case 21: {
            let row = floor(p.y * 2.0);
            let bw = 0.8 + 0.4 * hash2(vec2<f32>(row, 7.0));
            let u = p.x + hash2(vec2<f32>(row, 3.0)) * bw;
            let e = min(edist(u, bw), edist(p.y, 0.5));
            let id = hash2(vec2<f32>(floor(u / bw), row));
            var c = base * (0.88 + 0.16 * tones(id, 4.0)) * (0.93 + 0.1 * noise(p * 3.6) + 0.04 * noise(p * 24.0));
            c = select(c, c * 0.92 + vec3<f32>(0.02, 0.015, 0.0), hash2(vec2<f32>(floor(u / bw) + 5.0, row)) > 0.75);
            let j = mix(base, vec3<f32>(0.9, 0.87, 0.8), 0.6);
            c = mix(j, c, on_piece(e, 0.01, w));
            tex_height = smoothstep(0.01, 0.06, e) * 0.035 * s;
            return far_off(c, base * 0.95, 0.5, w);
        }
        // Clapboard ("tablas solapadas"): a board a piece wide, leaning out
        // over the one below, a shadow under its lip, butt joints.
        case 22: {
            let f = fract(p.y);
            let row = floor(p.y);
            let along = p.x / 20.0 + hash2(vec2<f32>(row, 2.0));
            let id = hash2(vec2<f32>(floor(along), row));
            var c = base * (0.95 + 0.07 * tones(id, 3.0)) * (0.97 + 0.05 * noise(vec2<f32>(p.x * 0.32, p.y * 11.2)));
            c = c * (1.0 - 0.32 * smoothstep(0.82, 1.0, f)) * (0.92 + 0.08 * smoothstep(0.0, 0.25, f));
            c = mix(c * 0.7, c, on_piece(edist(along, 1.0) * 20.0, 0.02, w));
            tex_height = (1.0 - f) * 0.11 * smoothstep(0.0, 0.06, f) * s;
            return far_off(c, base * 0.93, 1.0, w);
        }
        // Shingles ("tejuela"): rows a piece high, shingles a piece and a
        // bit wide, staggered; each its tone; a shadow under each row.
        case 23: {
            let f = fract(p.y);
            let row = floor(p.y);
            let u = p.x / 1.2 + hash2(vec2<f32>(row, 9.0));
            let id = hash2(vec2<f32>(floor(u), row));
            var c = base * (0.8 + 0.32 * tones(id, 4.0)) * (0.95 + 0.08 * noise(vec2<f32>(p.x * 0.9, p.y * 5.4)));
            c = select(c, c * 1.15 + vec3<f32>(0.02), id > 0.85);
            c = c * (1.0 - 0.38 * smoothstep(0.78, 1.0, f));
            c = mix(base * 0.55, c, on_piece(edist(u, 1.0) * 1.2, 0.035, w));
            tex_height = (1.0 - f) * 0.12 * smoothstep(0.0, 0.06, f) * s;
            return far_off(c, base * 0.88, 1.0, w);
        }
        // Fish scales ("escamas"): rounded lower edges.
        case 24: {
            var row = floor(p.y);
            var f = fract(p.y);
            var u = p.x / 1.5 + 0.5 * odd(row);
            var x = fract(u) * 2.0 - 1.0;
            var bottom = 0.45 * (1.0 - sqrt(max(0.0, 1.0 - x * x)));
            if (f < bottom) {
                // Still the scale of the row below.
                row = row - 1.0;
                f = f + 1.0;
                u = p.x / 1.5 + 0.5 * odd(row);
                x = fract(u) * 2.0 - 1.0;
                bottom = 0.45 * (1.0 - sqrt(max(0.0, 1.0 - x * x)));
            }
            let id = hash2(vec2<f32>(floor(u), row));
            let edge = smoothstep(0.0, 0.06, f - bottom);
            var c = base * (0.82 + 0.28 * tones(id, 4.0)) * mix(0.55, 1.0, edge);
            c = c * (1.0 - 0.3 * smoothstep(0.85, 1.4, f));
            tex_height = (1.4 - f) * 0.12 * edge * s;
            return far_off(c, base * 0.85, 1.0, w);
        }
        // Barrel tiles ("teja curva"): channels and caps down the slope,
        // a lip at each course; some weathered.
        case 25: {
            let course = floor(p.y / 1.8);
            let f = fract(p.y / 1.8);
            let r = sin(fract(p.x + 0.5) * 3.14159);
            let id = hash2(vec2<f32>(floor(p.x + 0.5), course));
            var c = base * (0.84 + 0.26 * tones(id, 4.0));
            c = mix(c, base * 0.6 + vec3<f32>(0.1, 0.07, 0.04), smoothstep(0.6, 0.95, noise(p * 0.18)) * 0.45);
            c = c * (0.72 + 0.36 * r) * (1.0 - 0.35 * smoothstep(0.85, 1.0, f));
            tex_height = (r * 0.16 + (1.0 - f) * 0.055 * smoothstep(0.0, 0.04, f)) * s;
            return far_off(c, base * 0.86, 1.0, w);
        }
        // Standing seam ("chapa de junta alzada"): panels a piece wide,
        // their seams catching the light, a patina in patches.
        case 26: {
            let e = edist(p.x, 1.0);
            let id = hash2(vec2<f32>(floor(p.x), 1.0));
            var c = base * (0.95 + 0.08 * tones(id, 3.0));
            let pat = smoothstep(0.45, 0.8, noise(p * vec2<f32>(0.36, 0.16)) * 0.7 + noise(p * 1.8) * 0.3);
            c = mix(c, base * 1.3 + vec3<f32>(0.03, 0.05, 0.04), pat * 0.6);
            let rib = 1.0 - smoothstep(0.0, max(0.03, w), e);
            c = mix(c, c * 1.18, rib) * (1.0 - 0.18 * (1.0 - smoothstep(0.03, 0.09, e)) * (1.0 - rib));
            tex_height = rib * 0.05 * s;
            return far_off(c, base * 1.02, 1.0, w);
        }
        // Setts ("adoquín"): rounded stones in courses.
        case 27: {
            return setts(p, base, w);
        }
        // Asphalt: worn patches and light speckles.
        case 28: {
            let n = noise(p * 0.25) * 0.6 + noise(p * 1.3) * 0.4;
            var c = base * (0.96 + 0.12 * tones(smoothstep(0.35, 0.8, n), 3.0));
            let k3 = cells(p * 30.0);
            c = mix(c, c * 1.35, (1.0 - smoothstep(0.05, 0.12, k3.x)) * step(0.7, k3.z) * (1.0 - smoothstep(0.02, 0.06, w)));
            return c;
        }
        // Rock in strata: soft horizontal layers of sandstone of different
        // thickness and tone, each a little proud of the one above (light
        // on its top edge, shade under it), runs of weathering, a few deep
        // crevices.
        case 29: {
            var z = p.y + noise(vec2<f32>(p.x * 0.14, 2.0)) * 1.6 + noise(vec2<f32>(p.x * 0.6, 7.0)) * 0.27;
            z = z + 0.8 * sin(z * 0.5);
            let band = floor(z);
            let f = fract(z);
            var c = base * (1.0 - 0.18 * tones(hash2(vec2<f32>(band, 5.0)), 4.0));
            c = c * (1.0 + 0.14 * smoothstep(0.82, 0.97, f)) * (1.0 - 0.22 * (1.0 - smoothstep(0.0, 0.18, f)));
            c = c * (0.9 + 0.13 * noise(vec2<f32>(p.x * 1.3, p.y * 0.1)));
            c = c * (1.0 - 0.35 * step(0.55, hash2(vec2<f32>(band, 9.0))) * (1.0 - smoothstep(0.0, 0.07, f)));
            tex_height = f * 0.12 * (1.0 - smoothstep(0.94, 1.0, f)) * s;
            return far_off(c, base * 0.92, 1.0, w);
        }
        // The sea: broad tones, and strokes of light blue to white that
        // drift towards the shore and slide along it, in dashes.
        case 30: {
            let q = p + vec2<f32>(noise(p * 0.08 + t * 0.01) * 6.0, 0.0);
            let wv = noise(p * vec2<f32>(0.1, 0.25) + t * vec2<f32>(0.02, 0.01)) * 0.7 + noise(p * 0.5 - t * 0.03) * 0.3;
            let light = base * 1.3 + vec3<f32>(0.12, 0.22, 0.24);
            var c = mix(base, mix(base, light, 0.3), smoothstep(0.4, 0.7, wv));
            let deep = wave_strokes(q * 1.1 + 11.0, 0.28, 0.04, t, 0.3, 0.4);
            let mid = wave_strokes(q, 0.36, 0.06, t, 0.22, 0.48);
            let crest = wave_strokes(q * 0.8 + 37.0, 0.3, 0.08, t, 0.12, 0.62);
            let near = 1.0 - smoothstep(0.03, 0.1, w);
            c = mix(c, mix(base, light, 0.55), deep * 0.55);
            c = mix(c, mix(base, light, 0.85), mid * 0.75);
            c = mix(c, vec3<f32>(0.93, 0.97, 1.0), crest * 0.8 * near);
            tex_height = (deep * 0.5 + mid + crest) * 0.015 * s;
            return c;
        }
        // Setts laid diagonally: courses half a sett high, turned 45°.
        case 31: {
            let q = vec2<f32>(p.x + p.y, p.y - p.x);
            return setts(q, base, w);
        }
        // Setts in fans: each fan a circle of arcs, half a fan's width
        // across (sixteen setts wide); the fans in rows 3/8 of a width
        // apart, every other row moved half a fan; the lower fans lie over
        // the upper ones, so each shows its upper part, bounded by the arcs
        // of the fans below it.
        case 32: {
            let fw = 16.0;
            let rad = 8.0;
            let fh = 6.0;
            var centre = vec2<f32>(0.0);
            var frow = 0.0;
            var found = false;
            var below = 99.0;
            let r0 = floor((p.y - rad) / fh);
            for (var k = 0; k < 4; k = k + 1) {
                let r = r0 + f32(k);
                let off = odd(r) * 0.5 * fw;
                let cx = floor((p.x - off) / fw + 0.5) * fw + off;
                let cc = vec2<f32>(cx, r * fh);
                let dist = length(p - cc);
                if (!found) {
                    if (dist < rad) {
                        centre = cc;
                        frow = r;
                        found = true;
                    } else {
                        below = min(below, dist - rad);
                    }
                }
            }
            let d = p - centre;
            let rr = length(d);
            let band = floor(rr);
            let along = atan2(d.x, d.y) * (band + 0.5) + hash2(vec2<f32>(band, frow));
            let e = min(min(edist(rr, 1.0), edist(along, 1.0) * max(rr, 1.0) / max(band + 0.5, 0.5)), below);
            let id = hash2(vec2<f32>(floor(along), band + frow * 31.0 + centre.x));
            var c = base * (0.8 + 0.28 * tones(id, 4.0)) * select(1.0, 0.85, id > 0.8);
            c = mix(base * 0.6, c, on_piece(e, 0.05, w));
            tex_height = sqrt(smoothstep(0.05, 0.15, e)) * 0.1 * s;
            return far_off(c, base * 0.85, 1.0, w);
        }
        // Cobbles ("empedrado"): rounded stones packed close, dark gaps.
        case 33: {
            let k3 = cells(p);
            let e = (k3.y - k3.x) * 0.5;
            let round_ = 1.0 - smoothstep(0.05, 0.6, k3.x);
            var c = base * (0.78 + 0.32 * tones(k3.z, 5.0));
            c = select(c, c * 0.8 + vec3<f32>(0.07, 0.06, 0.04), k3.z > 0.82);
            c = mix(base * 0.42, c * (0.72 + 0.4 * round_), on_piece(e, 0.056, w));
            tex_height = sqrt(round_) * 0.19 * s * on_piece(e, 0.056, w);
            return far_off(c, base * 0.75, 1.0, w);
        }
        // Old cobbles: bigger, uneven, worn smooth on top, earth and a
        // little moss in the joints.
        case 34: {
            let q = p + vec2<f32>(noise(p * 0.31), noise(p * 0.31 + 4.0)) * 0.35;
            let k3 = cells(q * vec2<f32>(1.0, 1.3));
            let e = (k3.y - k3.x) * 0.5;
            var c = base * (0.74 + 0.36 * tones(k3.z, 5.0));
            c = select(c, c * 0.85 + vec3<f32>(0.06, 0.05, 0.03), k3.z > 0.75);
            c = c * (0.88 + 0.2 * (1.0 - smoothstep(0.0, 0.45, k3.x)));
            let joint_c = mix(base * 0.45 + vec3<f32>(0.02, 0.015, 0.0), vec3<f32>(0.05, 0.11, 0.025), smoothstep(0.55, 0.75, noise(p * 0.5)) * 0.7);
            let half = 0.0625 * (0.7 + 0.6 * noise(p * 0.7));
            c = mix(joint_c, c, on_piece(e, half, w));
            tex_height = sqrt(smoothstep(half, half + 0.17, e)) * 0.125 * s;
            return far_off(c, base * 0.8, 1.0, w);
        }
        // Opus romanum: squares and oblongs of four sizes in a module of
        // 8 × 6 units that repeats.
        case 35: {
            let m = floor(p / vec2<f32>(8.0, 6.0));
            let l = p - m * vec2<f32>(8.0, 6.0);
            let r = opus_piece(l);
            let a = l - r.xy;
            let b = r.xy + r.zw - l;
            let e = min(min(a.x, a.y), min(b.x, b.y));
            let id = hash2(m * 7.0 + vec2<f32>(r.x * 3.0 + r.y, r.z));
            var c = base * (0.86 + 0.2 * tones(id, 4.0)) * (0.95 + 0.07 * noise(p * 0.75) + 0.03 * noise(p * 6.0));
            c = select(c, c * 0.88 + vec3<f32>(0.03, 0.02, 0.0), id > 0.7);
            c = mix(mix(base, vec3<f32>(0.94, 0.91, 0.84), 0.7), c, on_piece(e, 0.027, w));
            tex_height = smoothstep(0.027, 0.08, e) * 0.053 * s;
            return far_off(c, base * 0.95, 1.0, w);
        }
        // Flagstone ("laja"): irregular flat stones of several colours,
        // wide mortar joints.
        case 36: {
            let k3 = cells(p + vec2<f32>(noise(p * 0.7), noise(p * 0.7 + 3.0)) * 0.25);
            let e = (k3.y - k3.x) * 0.5;
            var c = base * (0.82 + 0.26 * tones(k3.z, 5.0));
            c = select(c, c * 0.85 + vec3<f32>(0.1, 0.05, 0.02), fract(k3.z * 3.7) > 0.7);
            c = c * (0.94 + 0.08 * noise(p * 1.5) + 0.04 * noise(p * 12.5));
            c = mix(mix(base, vec3<f32>(0.85, 0.82, 0.76), 0.7), c, on_piece(e, 0.025, w));
            tex_height = smoothstep(0.025, 0.065, e) * 0.024 * s;
            return far_off(c, base * 0.92, 1.0, w);
        }
        // Flagstones in grass: stepping stones, grass between them.
        case 37: {
            let k3 = cells(p);
            let e = (k3.y - k3.x) * 0.5;
            let stone = smoothstep(0.17 - w, 0.19 + w, e);
            let c = base * (0.84 + 0.24 * tones(k3.z, 4.0)) * (0.94 + 0.08 * noise(p * 2.1));
            let gr = grass_at(p * s, vec3<f32>(0.18, 0.42, 0.07), w * s);
            tex_height = stone * 0.03 * s;
            return mix(gr, c, stone);
        }
        // Marble slabs: veined slabs a piece square, thin joints.
        case 38: {
            let cell = floor(p);
            let id = hash2(cell);
            let q = p * 0.6 + id * 7.0;
            let wv = q.x * 1.3 + q.y * 0.7 + noise(q * 1.5) * 3.0 + noise(q * 5.0) * 0.8;
            let vein = 1.0 - smoothstep(0.0, 0.06, abs(sin(wv * 3.1416)));
            let fine = 1.0 - smoothstep(0.0, 0.03, abs(sin((wv + noise(q * 9.0)) * 9.0)));
            var c = base * (0.97 + 0.04 * tones(id, 3.0)) * (1.0 - 0.32 * vein - 0.12 * fine);
            c = mix(c * 0.8, c, on_piece(min(edist(p.x, 1.0), edist(p.y, 1.0)), 0.003, w));
            return far_off(c, base * 0.94, 1.0, w);
        }
        // Pebbles: small oval stones, white, grey and ochre, domed.
        case 39: {
            let k3 = cells(p * vec2<f32>(1.0, 1.4));
            let dome = 1.0 - smoothstep(0.0, 0.55, k3.x);
            var tint = base;
            if (k3.z > 0.66) {
                tint = base * 0.4 + vec3<f32>(0.5, 0.48, 0.44);
            } else if (k3.z < 0.33) {
                tint = base * 0.7 + vec3<f32>(0.12, 0.1, 0.06);
            }
            let gap = smoothstep(0.03, 0.09, k3.y - k3.x);
            let c = mix(base * 0.3, tint * (0.75 + 0.35 * dome), gap);
            tex_height = dome * 0.13 * gap * s;
            return far_off(c, base * 0.75, 1.0, w);
        }
        // Metro tiles: glazed oblongs twice as long as high, staggered, a
        // soft bevel and a glint along the top of each, thin light joints.
        case 40: {
            let row = floor(p.y * 2.0);
            let u = p.x + 0.5 * odd(row);
            let e = min(edist(u, 1.0), edist(p.y, 0.5));
            let id = hash2(vec2<f32>(floor(u), row));
            let fy = fract(p.y * 2.0);
            var c = base * (0.96 + 0.06 * tones(id, 3.0));
            c = mix(c, vec3<f32>(1.0), smoothstep(0.7, 0.86, fy) * (1.0 - smoothstep(0.86, 0.95, fy)) * 0.25);
            c = mix(mix(base, vec3<f32>(0.85, 0.84, 0.81), 0.7), c, on_piece(e, 0.0075, w));
            tex_height = sqrt(smoothstep(0.0075, 0.04, e)) * 0.02 * s;
            return far_off(c, base * 0.97, 0.5, w);
        }
        // Patterned tiles: on each a star in the middle, quarter circles at
        // the corners, a ring and four petals between, on a cream ground.
        case 41: {
            let f = fract(p) - 0.5;
            let a = abs(f);
            let r = length(f);
            let star = 1.0 - smoothstep(0.17, 0.18, min(a.x + a.y, max(a.x, a.y) * 1.25));
            let ring = smoothstep(0.27, 0.28, r) * (1.0 - smoothstep(0.31, 0.32, r));
            let corner = 1.0 - smoothstep(0.2, 0.21, length(a - 0.5));
            let petals = (1.0 - smoothstep(0.06, 0.07, length(a - vec2<f32>(0.33, 0.0)))) + (1.0 - smoothstep(0.06, 0.07, length(a - vec2<f32>(0.0, 0.33))));
            var c = mix(vec3<f32>(0.85, 0.78, 0.6), base, clamp(corner + ring + petals, 0.0, 1.0));
            c = mix(c, base * 0.45 + vec3<f32>(0.35, 0.22, 0.02), star);
            c = c * (0.97 + 0.04 * noise(p * 8.0));
            let e = min(edist(p.x, 1.0), edist(p.y, 1.0));
            c = mix(vec3<f32>(0.8, 0.77, 0.7), c, on_piece(e, 0.0075, w));
            tex_height = smoothstep(0.0075, 0.03, e) * 0.015 * s;
            return far_off(c, mix(base, vec3<f32>(0.85, 0.78, 0.6), 0.5), 1.0, w);
        }
        // An earth path: trodden earth in soft patches, little stones, a
        // few cracks.
        case 42: {
            let n = noise(p * 0.6) * 0.6 + noise(p * 2.2) * 0.4;
            var c = mix(base * 0.88, base * 1.15 + vec3<f32>(0.03, 0.025, 0.015), tones(smoothstep(0.3, 0.75, n), 3.0) * 0.6);
            let k3 = cells(p * 7.0);
            let stone = (1.0 - smoothstep(0.1, 0.17, k3.x)) * step(0.72, k3.z);
            c = mix(c, base * 0.7 + vec3<f32>(0.3, 0.28, 0.24), stone * 0.8);
            let k4 = cells(p * 1.3);
            c = c * (1.0 - 0.25 * (1.0 - smoothstep(0.0, 0.025, k4.y - k4.x)) * step(0.5, noise(p * 0.7)));
            tex_height = stone * 0.008 * s;
            return c;
        }
        // Albero: fine yellow sand, softly raked.
        case 43: {
            let n = noise(p * 0.8) * 0.5 + noise(p * 3.0) * 0.3 + noise(p * 60.0) * 0.2;
            var c = base * (0.92 + 0.12 * tones(smoothstep(0.3, 0.75, n), 4.0));
            let k3 = cells(p * 25.0);
            c = mix(c, c * 0.8, (1.0 - smoothstep(0.05, 0.12, k3.x)) * step(0.8, k3.z) * (1.0 - smoothstep(0.004, 0.012, w)));
            return c;
        }
        // Furrows: ridges a piece apart, lit on one side and shaded on the
        // other, a row of small shoots on some.
        case 44: {
            let f = fract(p.y);
            let row = floor(p.y);
            let ridge = sin(f * 3.14159);
            var c = mix(base * 0.55, base * 1.1, ridge) * (0.9 + 0.14 * noise(p * vec2<f32>(1.05, 4.2)) + 0.06 * noise(p * 17.5));
            let i = floor(p.x / 0.25);
            let l = vec2<f32>(fract(p.x / 0.25) - 0.5 + (hash2(vec2<f32>(i, row)) - 0.5) * 0.3, (f - 0.5) * 4.0);
            let sown = step(0.3, hash2(vec2<f32>(row, 1.0))) * step(0.15, hash2(vec2<f32>(i, row + 7.0)));
            let shoot = (1.0 - smoothstep(0.16, 0.22, length(l * vec2<f32>(1.0, 0.8)))) * sown;
            c = mix(c, vec3<f32>(0.2, 0.42, 0.06) * (0.8 + 0.4 * hash2(vec2<f32>(i, row))), shoot);
            tex_height = (ridge * 0.07 + shoot * 0.015) * s;
            return far_off(c, base * 0.85, 1.0, w);
        }
        // A kitchen garden: round green plants along the ridges of earth.
        case 45: {
            let f = fract(p.y);
            let row = floor(p.y);
            let ridge = sin(f * 3.14159);
            var c = mix(base * 0.6, base, ridge) * (0.92 + 0.1 * noise(p * 2.4));
            let sh = hash2(vec2<f32>(row, 3.0));
            let i = floor(p.x / 0.5 + sh);
            let l = vec2<f32>(fract(p.x / 0.5 + sh) - 0.5, (f - 0.5) * 2.0);
            let size = 0.32 + 0.1 * hash2(vec2<f32>(i, row));
            let plant = 1.0 - smoothstep(size - 0.03, size, length(l));
            let green = mix(vec3<f32>(0.07, 0.2, 0.03), vec3<f32>(0.16, 0.38, 0.06), noise(p * 24.0)) * (0.85 + 0.25 * (0.5 - l.y));
            c = mix(c, green, plant);
            tex_height = plant * (size - length(l)) * 0.15 * s;
            return far_off(c, mix(base * 0.8, vec3<f32>(0.1, 0.27, 0.04), 0.4), 1.0, w);
        }
        // Cracked earth: dry plates curling up a little, dark cracks.
        case 46: {
            let k3 = cells(p);
            let e = (k3.y - k3.x) * 0.5;
            let curl = smoothstep(0.0, 0.5, k3.x);
            var c = base * (0.9 + 0.14 * tones(k3.z, 3.0)) * (0.95 + 0.06 * noise(p * 2.8)) * (1.0 + 0.08 * curl);
            c = mix(base * 0.45, c, on_piece(e, 0.017, w));
            tex_height = curl * 0.017 * s;
            return far_off(c, base * 0.9, 1.0, w);
        }
        default: {
            return base;
        }
    }
}

struct ModelOut {
    @location(0) color: vec4<f32>,
    @location(1) normal_dist: vec4<f32>,
};

@fragment
fn fs_model(i: VertexOut, @builtin(front_facing) front: bool) -> ModelOut {
    var out: ModelOut;
    tex_begin(i.world, i.normal);
    tex_level = tex_lod(i.world, i.normal, i.tex);
    // The texture and its relief, worked out here, where every pixel still
    // runs together (derivatives).
    let base = textured(i.color.rgb, i.world, i.normal, i.tex);
    let bent = relief_normal(i.world, i.normal);
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
    // Selected: the distance is written negative, for the contour pass
    // to outline it.
    let mark = select(1.0, -1.0, i.picked > 0.5);
    if (front && i.soft < -0.5 && i.soft > -1.5) {
        // Between two layers: never seen from the outside.
        discard;
    }
    if (!front || i.soft < -1.5) {
        // The inside of a solid shows only through a cut.
        if (!kept.on) {
            discard;
        }
        let c = cap(i.world, kept);
        // Solid, as a poché: the material's cut colour (each layer of a
        // wall its own) or the one flat colour, barely shaded so that the
        // faces of the cut still read.
        let flat_fill = g.caps.w > 0.5;
        let fill = select(i.cap, g.caps.xyz, flat_fill);
        let lit = 0.92 + 0.08 * max(dot(c.xyz, normalize(vec3<f32>(0.3, -0.4, 0.86))), 0.0);
        out.color = vec4<f32>(fill * lit, 1.0);
        var n = normalize((g.view * vec4<f32>(c.xyz, 0.0)).xyz);
        if (!flat_fill) {
            // A thin line between layers, as in plan: the normal written
            // for the contour pass leans a different way for each cut
            // colour, so two layers side by side make a crease.
            let h = fract(sin(dot(i.cap, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.547) * 6.2831853;
            n = normalize(n + 0.9 * vec3<f32>(cos(h), sin(h), 0.0));
        }
        out.normal_dist = vec4<f32>(n, c.w * mark);
        return out;
    }
    let lit = sunlit(i.world, i.normal);
    var color = vivid(shade(bent, base, lit) + base * lamplight(i.world, bent));
    if (i.soft > 1.5 && i.soft < 2.5) {
        // A lampshade or a bulb at night: it glows.
        let glow = (base * 0.6 + vec3<f32>(0.55, 0.42, 0.25)) * 1.6;
        color = mix(color, glow, g.atmos.y);
    }
    out.color = vec4<f32>(hazed(color, i.view_dist), 1.0);
    out.normal_dist = vec4<f32>(normalize(i.view_normal), i.view_dist * mark);
    if (i.soft > 0.5 && i.soft < 1.5) {
        // Leaves: the same normal everywhere, so the contours come only
        // from the outline of the masses.
        out.normal_dist = vec4<f32>(0.0, 0.0, 1.0, i.view_dist * mark);
    }
    if (i.soft > 3.5) {
        // Many small leaves: marked (a normal straight back), and the
        // contour pass draws no line next to them.
        out.normal_dist = vec4<f32>(0.0, 0.0, -1.0, i.view_dist * mark);
    }
    return out;
}

@fragment
fn fs_glass(i: VertexOut) -> @location(0) vec4<f32> {
    // Water's texture (moving) and its ripples, first (derivatives).
    tex_begin(i.world, i.normal);
    tex_level = tex_lod(i.world, i.normal, i.tex);
    let base = textured(i.color.rgb, i.world, i.normal, i.tex);
    let bent = relief_normal(i.world, i.normal);
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
    let c = vivid(shade(bent, base, 1.0)) + base * lamplight(i.world, bent);
    let alpha = water_alpha(i.world, i.normal, i32(i.tex.x + 0.5), i.color.a);
    return vec4<f32>(hazed(c, i.view_dist), alpha);
}

// ---- Shadow map -------------------------------------------------------------
//
// The scene's depth seen from the sun. What the cut away walls take away
// casts no shadow.

struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) cut: vec4<f32>,
    @location(2) @interpolate(flat) picked: f32,
};

@vertex
fn vs_shadow(input: VertexIn) -> ShadowOut {
    let v = corner(input);
    var out: ShadowOut;
    out.clip = g.sun_vp * vec4<f32>(v.pos, 1.0);
    out.world = v.pos;
    out.cut = v.cut;
    out.picked = v.picked;
    return out;
}

@fragment
fn fs_shadow(i: ShadowOut) {
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
}

// ---- Contours -------------------------------------------------------------

@group(1) @binding(0) var normal_dist: texture_2d<f32>;

@vertex
fn vs_fullscreen(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // One triangle covering the whole view.
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn sample_nd(p: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(normal_dist));
    return textureLoad(normal_dist, clamp(p, vec2<i32>(0), size - 1), 0);
}

// Nothing drawn there (the distance of a selected element is negative).
fn empty(a: vec4<f32>) -> bool {
    return abs(a.w) < 1e-4;
}

// A sample of something selected (its distance written negative).
fn picked_at(a: vec4<f32>) -> bool {
    return a.w < 0.0 && !empty(a);
}

// A sample of many small leaves (no contour lines next to them).
fn fine(a: vec4<f32>) -> bool {
    return !empty(a) && a.z < -0.99;
}

// Whether two neighbouring samples belong to different surfaces.
fn crease(a: vec4<f32>, b: vec4<f32>) -> f32 {
    if (fine(a) || fine(b)) {
        return 0.0;
    }
    let a_empty = empty(a);
    let b_empty = empty(b);
    if (a_empty && b_empty) {
        return 0.0;
    }
    if (a_empty != b_empty) {
        return 1.0;
    }
    return select(0.0, 1.0, dot(a.xyz, b.xyz) < 0.94);
}

// Whether the distance steps between three samples in a row: inverse
// distance changes linearly across a flat surface, so a kink in it is a
// step from one surface to another behind it.
fn depth_step(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (empty(a) || empty(b) || empty(c)) {
        return 0.0;
    }
    if (fine(a) || fine(b) || fine(c)) {
        return 0.0;
    }
    let ia = 1.0 / abs(a.w);
    let ib = 1.0 / abs(b.w);
    let ic = 1.0 / abs(c.w);
    let kink = abs(ia + ib - 2.0 * ic) / ic;
    return select(0.0, 1.0, kink > 0.03);
}

// A step in depth seen from the nearer of its two surfaces only: the
// line of a thin-line step is drawn on one pixel, not on both sides.
fn near_step(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (depth_step(a, c, b) < 0.5) {
        return 0.0;
    }
    let ia = 1.0 / abs(a.w);
    let ib = 1.0 / abs(b.w);
    let ic = 1.0 / abs(c.w);
    return select(0.0, 1.0, 2.0 * ic > ia + ib);
}

// Where the line of sight through a pixel meets the ground (w = 1), or
// its direction (w = 0) when it looks above the horizon.
fn sight(pos: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(normal_dist));
    let ndc = vec2<f32>(pos.x / size.x * 2.0 - 1.0, 1.0 - pos.y / size.y * 2.0);
    let near = g.inv_vp * vec4<f32>(ndc, 0.0, 1.0);
    let far = g.inv_vp * vec4<f32>(ndc, 1.0, 1.0);
    let a = near.xyz / near.w;
    let b = far.xyz / far.w;
    let d = normalize(b - a);
    if (d.z < -1e-4) {
        let t = (g.ground.w - a.z) / d.z;
        if (t > 0.0) {
            return vec4<f32>(a + d * t, 1.0);
        }
    }
    return vec4<f32>(d, 0.0);
}

// The sky and the ground fading into the haze, with the grid on the
// ground near what is looked at; `fade` is how much the grid shows.
fn backdrop(at: vec4<f32>, line: f32) -> vec3<f32> {
    if (at.w < 0.5) {
        let up = clamp(at.z, 0.0, 1.0);
        return mix(g.haze.rgb, g.zenith.rgb, pow(up, 0.55));
    }
    let dist = length(at.xyz - g.eye.xyz);
    var c = mix(g.ground.rgb, g.haze.rgb, smoothstep(g.atmos.z * 0.5, g.atmos.w * 1.5, dist));
    c = mix(c, vec3<f32>(0.32, 0.36, 0.44) * (1.0 - 0.8 * g.atmos.y), line);
    return c;
}

@fragment
fn fs_edges(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let s = i32(g.edges.x);
    let c = sample_nd(p);
    // The grid's lines, measured before anything is decided (derivatives
    // only in uniform control flow).
    let at = sight(pos.xy);
    var line = 0.0;
    if (g.grid.w > 0.0) {
        let q = at.xy / g.grid.w;
        let w = max(fwidth(q), vec2<f32>(1e-4));
        let dl = abs(fract(q - 0.5) - 0.5) / w;
        let minor = 1.0 - clamp(min(dl.x, dl.y) - 0.5, 0.0, 1.0);
        let q2 = q * 0.5;
        let dl2 = abs(fract(q2 - 0.5) - 0.5) / max(fwidth(q2), vec2<f32>(1e-4));
        let major = 1.0 - clamp(min(dl2.x, dl2.y) - 0.5, 0.0, 1.0);
        let off = abs(at.xy - g.grid.xy);
        let inside = 1.0 - smoothstep(g.grid.z * 0.8, g.grid.z, max(off.x, off.y));
        line = max(minor * 0.22, major * 0.45) * inside * at.w;
    }
    if (g.atmos.x > 0.5 && empty(c)) {
        return vec4<f32>(backdrop(at, line), 1.0);
    }
    let l = sample_nd(p - vec2<i32>(s, 0));
    let r = sample_nd(p + vec2<i32>(s, 0));
    let u = sample_nd(p - vec2<i32>(0, s));
    let d = sample_nd(p + vec2<i32>(0, s));
    var e = 0.0;
    if (g.edges.y < 0.0) {
        // Thin lines: an edge is marked on one side only (towards the
        // right and down for a crease, on the nearer surface for a step in
        // depth), so it is one pixel wide instead of two.
        e = max(crease(c, r), crease(c, d));
        e = max(e, max(near_step(l, c, r), near_step(u, c, d)));
    } else {
        e = max(max(crease(c, r), crease(c, d)), max(crease(c, l), crease(c, u)));
        e = max(e, max(depth_step(l, c, r), depth_step(u, c, d)));
    }
    // The outline of what is selected, a little thicker than the
    // contours, in its own colour: its faces keep theirs.
    // No loops here: the shader compiler of Windows (FXC) refuses to
    // unroll them.
    if (g.edges.z > 0.5) {
        let inside = picked_at(c);
        let o = s;
        let t = 2 * s;
        var ring = false;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, 0))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, 0))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(0, o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(0, o))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(t, 0))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(t, 0))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(0, t))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(0, t))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, o))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, -o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, -o))) != inside;
        if (ring) {
            return vec4<f32>(0.98, 0.55, 0.05, 1.0);
        }
    }
    var shade_corner = 0.0;
    if (g.look.w > 0.5) {
        shade_corner = corner_shade(p, c, 3 * s);
    }
    // The ink: darker at night, or it would be lighter than the scene.
    // Thin lines are the surface's own colour darkened (a little over
    // half way to the ink), finer than black.
    let ink = vec3<f32>(0.05, 0.05, 0.08) * (1.0 - 0.9 * g.atmos.y);
    let strength = select(1.0, 0.6, g.edges.y < 0.0);
    return vec4<f32>(ink, max(e * strength, shade_corner));
}

// How far the surface through `c` bends inwards between two samples on
// either side: 0 on a flat face (1 / distance changes linearly across it)
// or a corner turned outwards, more in an inner corner. A big step (one
// thing in front of another) is not a corner.
fn bend(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (empty(a) || empty(b) || empty(c)) {
        return 0.0;
    }
    let ic = 1.0 / abs(c.w);
    let k = ((1.0 / abs(a.w) + 1.0 / abs(b.w)) * 0.5 - ic) / ic;
    return clamp(k * 25.0, 0.0, 1.0) * (1.0 - smoothstep(0.06, 0.15, abs(k)));
}

// The shade of inner corners (where a wall meets the floor, under a
// sofa…), from the distances around the point. No loops.
fn corner_shade(p: vec2<i32>, c: vec4<f32>, r: i32) -> f32 {
    var k = bend(sample_nd(p - vec2<i32>(r, 0)), c, sample_nd(p + vec2<i32>(r, 0)));
    k = k + bend(sample_nd(p - vec2<i32>(0, r)), c, sample_nd(p + vec2<i32>(0, r)));
    k = k + bend(sample_nd(p - vec2<i32>(r, r)), c, sample_nd(p + vec2<i32>(r, r)));
    k = k + bend(sample_nd(p - vec2<i32>(r, -r)), c, sample_nd(p + vec2<i32>(r, -r)));
    let t = 2 * r;
    k = k + bend(sample_nd(p - vec2<i32>(t, 0)), c, sample_nd(p + vec2<i32>(t, 0)));
    k = k + bend(sample_nd(p - vec2<i32>(0, t)), c, sample_nd(p + vec2<i32>(0, t)));
    return k / 6.0 * 0.4;
}
