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
    // x: pixel step for contour detection; y: contour strength.
    edges: vec4<f32>,
    // Section box corners; clip_min.w = 1 when the box is on.
    clip_min: vec4<f32>,
    clip_max: vec4<f32>,
    // Eye position (w = 1 in perspective) and viewing direction.
    eye: vec4<f32>,
    forward: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bias: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) view_normal: vec3<f32>,
    @location(3) view_dist: f32,
    @location(4) world: vec3<f32>,
    // Foliage (bias 2): no contour lines between its own facets.
    @location(5) @interpolate(flat) soft: f32,
};

@vertex
fn vs_model(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    var vp = g.view * vec4<f32>(v.pos, 1.0);
    // A nudge along the line of sight, 0.02 % of the distance: invisible,
    // but it settles which element wins where two touch on an edge.
    vp.z = vp.z * (1.0 + min(v.bias, 1.0) * 0.0002);
    out.soft = select(0.0, 1.0, v.bias > 1.5);
    out.clip = g.proj * vp;
    out.view_normal = (g.view * vec4<f32>(v.normal, 0.0)).xyz;
    out.view_dist = -vp.z;
    out.normal = v.normal;
    out.color = v.color;
    out.world = v.pos;
    return out;
}

fn boxed() -> bool {
    return g.clip_min.w > 0.5;
}

// Outside the section box (a hair of tolerance keeps faces on it).
fn outside(p: vec3<f32>) -> bool {
    let e = 1e-4;
    return any(p < g.clip_min.xyz - vec3<f32>(e)) || any(p > g.clip_max.xyz + vec3<f32>(e));
}

// Where the line of sight through `p` (inside the box) leaves it towards
// the eye: the cap of the cut. Returns the cap's normal (xyz) and its
// distance along the view (w).
fn cap(p: vec3<f32>) -> vec4<f32> {
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
            var bound = g.clip_min[k];
            var sgn = -1.0;
            if (b[k] > 0.0) {
                bound = g.clip_max[k];
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

fn shade(n_in: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    let n = normalize(n_in);
    let sun = max(dot(n, normalize(g.sun.xyz)), 0.0);
    // Sky from above, a little bounce from the ground below.
    let sky = 0.5 + 0.5 * n.z;
    let light = 0.38 + 0.22 * sky + 0.55 * sun;
    return color * light;
}

struct ModelOut {
    @location(0) color: vec4<f32>,
    @location(1) normal_dist: vec4<f32>,
};

@fragment
fn fs_model(i: VertexOut, @builtin(front_facing) front: bool) -> ModelOut {
    var out: ModelOut;
    if (boxed() && outside(i.world)) {
        discard;
    }
    if (!front) {
        // The inside of a solid shows only through a cut.
        if (!boxed()) {
            discard;
        }
        let c = cap(i.world);
        out.color = vec4<f32>(shade(c.xyz, i.color.rgb) * 0.82, 1.0);
        out.normal_dist = vec4<f32>(normalize((g.view * vec4<f32>(c.xyz, 0.0)).xyz), c.w);
        return out;
    }
    out.color = vec4<f32>(shade(i.normal, i.color.rgb), 1.0);
    out.normal_dist = vec4<f32>(normalize(i.view_normal), i.view_dist);
    if (i.soft > 0.5) {
        // Leaves: the same normal everywhere, so the contours come only
        // from the outline of the masses.
        out.normal_dist = vec4<f32>(0.0, 0.0, 1.0, i.view_dist);
    }
    return out;
}

@fragment
fn fs_glass(i: VertexOut) -> @location(0) vec4<f32> {
    if (boxed() && outside(i.world)) {
        discard;
    }
    return vec4<f32>(shade(i.normal, i.color.rgb), i.color.a);
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

// Whether two neighbouring samples belong to different surfaces.
fn crease(a: vec4<f32>, b: vec4<f32>) -> f32 {
    let a_empty = a.w <= 0.0;
    let b_empty = b.w <= 0.0;
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
    if (a.w <= 0.0 || b.w <= 0.0 || c.w <= 0.0) {
        return 0.0;
    }
    let ia = 1.0 / a.w;
    let ib = 1.0 / b.w;
    let ic = 1.0 / c.w;
    let kink = abs(ia + ib - 2.0 * ic) / ic;
    return select(0.0, 1.0, kink > 0.03);
}

@fragment
fn fs_edges(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let s = i32(g.edges.x);
    let c = sample_nd(p);
    let l = sample_nd(p - vec2<i32>(s, 0));
    let r = sample_nd(p + vec2<i32>(s, 0));
    let u = sample_nd(p - vec2<i32>(0, s));
    let d = sample_nd(p + vec2<i32>(0, s));
    var e = max(max(crease(c, r), crease(c, d)), max(crease(c, l), crease(c, u)));
    e = max(e, max(depth_step(l, c, r), depth_step(u, c, d)));
    return vec4<f32>(0.05, 0.05, 0.06, e * g.edges.y);
}
