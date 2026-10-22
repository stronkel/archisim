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
    // x: texture (0 none, 1 brick…), y: size of its piece in metres.
    @location(4) tex: vec2<f32>,
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
    @location(6) @interpolate(flat) tex: vec2<f32>,
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
    out.tex = v.tex;
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

fn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {
    let kind = i32(tex.x + 0.5);
    if (kind == 0) {
        return base;
    }
    let n = abs(normalize(n_in));
    // Along the face (horizontal) and up it; on faces looking up, the plan.
    var uv = world.xy;
    if (n.z < 0.7) {
        if (n.x > n.y) {
            uv = world.yz;
        } else {
            uv = world.xz;
        }
    }
    let s = max(tex.y, 0.01);
    let p = uv / s;
    let mortar = mix(base, vec3<f32>(0.78, 0.76, 0.72), 0.75);
    switch kind {
        // Brick: courses a quarter of the brick's length high, every other
        // one moved half a brick; each brick a little different.
        case 1: {
            let row = floor(p.y * 3.8);
            let q = vec2<f32>(p.x + 0.5 * (row % 2.0), p.y * 3.8);
            let id = hash2(vec2<f32>(floor(q.x), row));
            let body = base * (0.82 + 0.3 * id) * (0.93 + 0.1 * noise(p * 9.0));
            let m = joint(q.x, 0.06) * joint(q.y, 0.2);
            return mix(mortar, body, m);
        }
        // Stone: irregular pieces with dark joints.
        case 2: {
            let c = cells(p * 1.6);
            let edge = smoothstep(0.02, 0.09, c.y - c.x);
            let body = base * (0.75 + 0.45 * c.z) * (0.9 + 0.15 * noise(p * 6.0));
            return mix(base * 0.45, body, edge);
        }
        // Tiles: squares with light joints.
        case 3: {
            let id = hash2(floor(p));
            let body = base * (0.95 + 0.08 * id);
            let m = joint(p.x, 0.03) * joint(p.y, 0.03);
            return mix(mortar, body, m);
        }
        // Wood: grain.
        case 4: {
            let g = sin(p.y * 40.0 + noise(p * vec2<f32>(1.5, 8.0)) * 6.0);
            return base * (0.86 + 0.1 * g + 0.08 * noise(p * vec2<f32>(0.5, 30.0)));
        }
        // Boards: strips a board wide, joined end to end in turns, with
        // their grain.
        case 5: {
            let row = floor(p.y);
            let along = p.x / 8.0 + hash2(vec2<f32>(row, 1.0));
            let id = hash2(vec2<f32>(floor(along), row));
            let g = sin(p.y * 25.0 + noise(vec2<f32>(p.x * 0.6, p.y * 5.0)) * 5.0);
            let body = base * (0.82 + 0.25 * id) * (0.93 + 0.06 * g);
            let m = joint(p.y, 0.06) * joint(along, 0.01);
            return mix(base * 0.4, body, m);
        }
        // Board-marked concrete: panels, their joints and tie holes.
        case 6: {
            let q = vec2<f32>(p.x, p.y * 2.0);
            let m = joint(q.x, 0.008) * joint(q.y, 0.012);
            let f = fract(q);
            let hole = smoothstep(0.012, 0.022, min(length(f - vec2<f32>(0.15, 0.25)), length(f - vec2<f32>(0.85, 0.25))));
            let body = base * (0.94 + 0.1 * noise(p * 12.0) + 0.04 * noise(p * 50.0));
            return body * mix(0.82, 1.0, m) * mix(0.55, 1.0, hole);
        }
        // Render: a fine grain.
        case 7: {
            return base * (0.95 + 0.07 * noise(p * 60.0) + 0.04 * noise(p * 7.0));
        }
        // Grass: blades of light and dark green.
        case 8: {
            let v = noise(p * vec2<f32>(18.0, 40.0)) * 0.6 + noise(p * 4.0) * 0.4;
            return base * (0.7 + 0.55 * v);
        }
        // Gravel: little stones.
        case 9: {
            let c = cells(p * 3.0);
            let body = base * (0.7 + 0.5 * c.z);
            return mix(base * 0.5, body, smoothstep(0.0, 0.25, c.y - c.x));
        }
        // Water: ripples.
        case 10: {
            let w = noise(p * vec2<f32>(3.0, 1.2) + vec2<f32>(noise(p * 2.0) * 2.0, 0.0));
            return base * (0.88 + 0.25 * smoothstep(0.55, 0.8, w));
        }
        // Sheet metal: ribs along the face.
        case 11: {
            let r = 0.5 + 0.5 * sin(p.x * 6.2832);
            return base * (0.8 + 0.3 * r);
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
    out.color = vec4<f32>(shade(i.normal, textured(i.color.rgb, i.world, i.normal, i.tex)), 1.0);
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
