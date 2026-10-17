// ArchiSim 3D view.
//
// Pass 1 draws the opaque model, lit by a sun from the south-west and a
// sky light, and also writes each pixel's view-space normal and distance.
// Pass 2 draws the glass, see-through, over it.
// Pass 3 finds contour lines from those normals and distances (edges where
// the surface turns, silhouettes, and steps in depth) and draws them in ink.

struct Globals {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
    // xyz: direction towards the sun; w: unused.
    sun: vec4<f32>,
    // x: pixel step for contour detection; y: contour strength.
    edges: vec4<f32>,
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
};

@vertex
fn vs_model(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    var vp = g.view * vec4<f32>(v.pos, 1.0);
    // A nudge along the line of sight, 0.02 % of the distance: invisible,
    // but it settles which element wins where two touch on an edge.
    vp.z = vp.z * (1.0 + v.bias * 0.0002);
    out.clip = g.proj * vp;
    out.view_normal = (g.view * vec4<f32>(v.normal, 0.0)).xyz;
    out.view_dist = -vp.z;
    out.normal = v.normal;
    out.color = v.color;
    return out;
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
fn fs_model(i: VertexOut) -> ModelOut {
    var out: ModelOut;
    out.color = vec4<f32>(shade(i.normal, i.color.rgb), 1.0);
    out.normal_dist = vec4<f32>(normalize(i.view_normal), i.view_dist);
    return out;
}

@fragment
fn fs_glass(i: VertexOut) -> @location(0) vec4<f32> {
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
