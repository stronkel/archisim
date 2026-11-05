//! A linear 3D frame solver (stiffness method): bars of two nodes with six
//! degrees of freedom at each end (three movements, three turns), end
//! releases (hinges), supports, loads at the nodes and loads spread over
//! part of a bar. It gives, for each load case, the movements of the
//! nodes, the forces at the ends of each bar, its internal forces along it
//! and the reactions.
//!
//! Local axes of a bar: `x` from its first node to the second; `z` the
//! component of the bar's `up` vector square to `x` (for a beam, up; for
//! a column, the direction of its section's depth); `y = z × x`. Bending
//! about `y` (`My`, with the shear `Vz`) is the strong axis of a beam
//! whose depth is along `z`.
//!
//! Units: metres, kN, kN·m.

pub type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn unit(a: V3) -> V3 {
    let n = norm(a);
    if n > 0.0 {
        scale(a, 1.0 / n)
    } else {
        a
    }
}

/// The properties of a bar's section and material (kN, m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// Area, m².
    pub a: f64,
    /// Second moment about local `y` (strong axis of a beam), m⁴.
    pub iy: f64,
    /// Second moment about local `z`, m⁴.
    pub iz: f64,
    /// Torsion constant, m⁴.
    pub j: f64,
    /// Young's modulus, kN/m².
    pub e: f64,
    /// Shear modulus, kN/m².
    pub g: f64,
}

/// A bar between two nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub i: usize,
    pub j: usize,
    pub section: Section,
    /// Any vector not along the bar that gives its local `z`.
    pub up: V3,
    /// Hinges: the bending turns free at the first and the second end.
    pub hinge_i: bool,
    pub hinge_j: bool,
}

/// What is held at a supported node: movements x, y, z and turns x, y, z.
pub type Fixity = [bool; 6];

pub const FIXED: Fixity = [true; 6];
pub const PINNED: Fixity = [true, true, true, false, false, false];
/// Held only up and down.
pub const ROLLER: Fixity = [false, false, true, false, false, false];

/// A load of one case.
#[derive(Debug, Clone, PartialEq)]
pub enum Load {
    /// At a node: forces x, y, z and moments x, y, z (global), kN and kN·m.
    Node { node: usize, f: [f64; 6] },
    /// Over a bar from `from` to `to` metres along it: force per metre in
    /// global axes (−z is down), kN/m.
    Spread {
        bar: usize,
        from: f64,
        to: f64,
        w: V3,
    },
    /// A force at `at` metres along a bar, global, kN.
    Point { bar: usize, at: f64, f: V3 },
}

/// The frame: nodes, bars and supports.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub nodes: Vec<V3>,
    pub bars: Vec<Bar>,
    pub supports: Vec<(usize, Fixity)>,
    /// Elastic springs at nodes, per degree of freedom (kN/m, kN·m/rad).
    pub springs: Vec<(usize, [f64; 6])>,
}

/// What the solver found for one case.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    /// Movements (m) and turns (rad) of each node, global.
    pub displacements: Vec<[f64; 6]>,
    /// The forces each bar takes at its ends, in its local axes: at the
    /// first end `[N, Vy, Vz, T, My, Mz]` acting on the bar, and the same
    /// at the second end.
    pub end_forces: Vec<[f64; 12]>,
    /// Reactions at the supports, global: `(node, [Fx, Fy, Fz, Mx, My, Mz])`.
    pub reactions: Vec<(usize, [f64; 6])>,
    /// Degrees of freedom that had nothing holding them: `(node, dof)`;
    /// the solver held them with a soft spring.
    pub loose: Vec<(usize, usize)>,
}

/// Local axes of a bar: the rows of the rotation (x, y, z).
pub fn axes(frame: &Frame, b: &Bar) -> [V3; 3] {
    let x = unit(sub(frame.nodes[b.j], frame.nodes[b.i]));
    let mut up = b.up;
    if norm(cross(x, up)) < 1e-6 * norm(up).max(1e-12) {
        up = if x[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
    }
    let y = unit(cross(up, x));
    let z = cross(x, y);
    [x, y, z]
}

pub fn length(frame: &Frame, b: &Bar) -> f64 {
    norm(sub(frame.nodes[b.j], frame.nodes[b.i]))
}

/// The local stiffness of a bar (12 × 12), before its releases.
fn local_stiffness(s: &Section, l: f64) -> [[f64; 12]; 12] {
    let mut k = [[0.0; 12]; 12];
    let ea = s.e * s.a / l;
    let gj = s.g * s.j / l;
    let (eiy, eiz) = (s.e * s.iy, s.e * s.iz);
    let l2 = l * l;
    let l3 = l2 * l;
    // Axial.
    k[0][0] = ea;
    k[0][6] = -ea;
    k[6][0] = -ea;
    k[6][6] = ea;
    // Torsion.
    k[3][3] = gj;
    k[3][9] = -gj;
    k[9][3] = -gj;
    k[9][9] = gj;
    // Bending in x–y (about z): v (1, 7) and θz (5, 11).
    let a = 12.0 * eiz / l3;
    let b = 6.0 * eiz / l2;
    let c = 4.0 * eiz / l;
    let d = 2.0 * eiz / l;
    for (r, cc, v) in [
        (1, 1, a),
        (1, 5, b),
        (1, 7, -a),
        (1, 11, b),
        (5, 5, c),
        (5, 7, -b),
        (5, 11, d),
        (7, 7, a),
        (7, 11, -b),
        (11, 11, c),
    ] {
        k[r][cc] = v;
        k[cc][r] = v;
    }
    // Bending in x–z (about y): w (2, 8) and θy (4, 10); signs flip.
    let a = 12.0 * eiy / l3;
    let b = 6.0 * eiy / l2;
    let c = 4.0 * eiy / l;
    let d = 2.0 * eiy / l;
    for (r, cc, v) in [
        (2, 2, a),
        (2, 4, -b),
        (2, 8, -a),
        (2, 10, -b),
        (4, 4, c),
        (4, 8, b),
        (4, 10, d),
        (8, 8, a),
        (8, 10, b),
        (10, 10, c),
    ] {
        k[r][cc] = v;
        k[cc][r] = v;
    }
    k
}

/// The equivalent nodal loads (local, fixed ends) of a local load per
/// metre `w` over `[a, b]` of a bar of length `l`, and of a point load.
fn fixed_end(l: f64, a: f64, b: f64, w: V3) -> [f64; 12] {
    // Gauss–Legendre with 4 points: exact for the cubic shape functions
    // times a constant.
    const G: [(f64, f64); 4] = [
        (-0.861_136_311_594_052_6, 0.347_854_845_137_453_9),
        (-0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
        (0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
        (0.861_136_311_594_052_6, 0.347_854_845_137_453_9),
    ];
    let mut f = [0.0; 12];
    let (a, b) = (a.clamp(0.0, l), b.clamp(0.0, l));
    if b <= a {
        return f;
    }
    let half = (b - a) / 2.0;
    for (xi, wt) in G {
        let x = a + half * (xi + 1.0);
        let n = shape(l, x);
        let k = wt * half;
        add_point(&mut f, &n, w, k);
    }
    f
}

/// The Hermite and linear shape functions at `x`: (n1, n2) linear, and
/// (h1, h2, h3, h4) for a transverse movement and the end turns.
fn shape(l: f64, x: f64) -> ([f64; 2], [f64; 4]) {
    let s = x / l;
    let lin = [1.0 - s, s];
    let h = [
        1.0 - 3.0 * s * s + 2.0 * s * s * s,
        x * (1.0 - s) * (1.0 - s),
        3.0 * s * s - 2.0 * s * s * s,
        x * (s * s - s),
    ];
    (lin, h)
}

fn add_point(f: &mut [f64; 12], n: &([f64; 2], [f64; 4]), w: V3, k: f64) {
    let (lin, h) = n;
    // Axial.
    f[0] += lin[0] * w[0] * k;
    f[6] += lin[1] * w[0] * k;
    // In y: v and θz.
    f[1] += h[0] * w[1] * k;
    f[5] += h[1] * w[1] * k;
    f[7] += h[2] * w[1] * k;
    f[11] += h[3] * w[1] * k;
    // In z: w and θy (θy = −dw/dx).
    f[2] += h[0] * w[2] * k;
    f[4] -= h[1] * w[2] * k;
    f[8] += h[2] * w[2] * k;
    f[10] -= h[3] * w[2] * k;
}

/// Released local degrees of freedom of a bar.
fn released(b: &Bar) -> Vec<usize> {
    let mut r = Vec::new();
    if b.hinge_i {
        r.extend([4, 5]);
    }
    if b.hinge_j {
        r.extend([10, 11]);
    }
    r
}

/// Condenses the released degrees of freedom out of a stiffness and its
/// load (Gauss elimination on them), leaving zeros in their rows.
fn condense(k: &mut [[f64; 12]; 12], f: &mut [f64; 12], rel: &[usize]) {
    for &r in rel {
        let p = k[r][r];
        if p.abs() < 1e-30 {
            continue;
        }
        for i in 0..12 {
            if i == r {
                continue;
            }
            let m = k[i][r] / p;
            if m == 0.0 {
                continue;
            }
            for j in 0..12 {
                k[i][j] -= m * k[r][j];
            }
            f[i] -= m * f[r];
        }
        for j in 0..12 {
            k[r][j] = 0.0;
            k[j][r] = 0.0;
        }
        f[r] = 0.0;
    }
}

fn to_local(ax: &[V3; 3], v: V3) -> V3 {
    [dot(ax[0], v), dot(ax[1], v), dot(ax[2], v)]
}

fn to_global(ax: &[V3; 3], v: V3) -> V3 {
    [
        ax[0][0] * v[0] + ax[1][0] * v[1] + ax[2][0] * v[2],
        ax[0][1] * v[0] + ax[1][1] * v[1] + ax[2][1] * v[2],
        ax[0][2] * v[0] + ax[1][2] * v[1] + ax[2][2] * v[2],
    ]
}

/// The skyline (profile) store of a symmetric matrix with LDLᵀ.
struct Skyline {
    /// First column with a value in each row (lower part, by rows).
    first: Vec<usize>,
    /// Start of each row in `data`; the row runs from `first` to the
    /// diagonal.
    start: Vec<usize>,
    data: Vec<f64>,
}

impl Skyline {
    fn new(first: Vec<usize>) -> Self {
        let mut start = Vec::with_capacity(first.len() + 1);
        let mut n = 0;
        for (i, f) in first.iter().enumerate() {
            start.push(n);
            n += i - f + 1;
        }
        start.push(n);
        Skyline {
            first,
            start,
            data: vec![0.0; n],
        }
    }

    fn at(&mut self, i: usize, j: usize) -> &mut f64 {
        let (i, j) = if j > i { (j, i) } else { (i, j) };
        debug_assert!(j >= self.first[i]);
        let s = self.start[i];
        &mut self.data[s + j - self.first[i]]
    }

    /// Factors in place into LDLᵀ; a pivot that vanishes (a mechanism)
    /// gets a soft spring and is reported.
    fn factor(&mut self, soft: f64) -> Vec<usize> {
        let n = self.first.len();
        let mut loose = Vec::new();
        for i in 0..n {
            let fi = self.first[i];
            let si = self.start[i];
            // Row i of L: L[i][j] for j in fi..i.
            for j in fi..i {
                let fj = self.first[j];
                let lo = fi.max(fj);
                let mut sum = self.data[si + j - fi];
                let sj = self.start[j];
                for k in lo..j {
                    sum -= self.data[si + k - fi]
                        * self.data[sj + k - fj]
                        * self.data[self.start[k] + k - self.first[k]];
                }
                let djj = self.data[sj + j - fj];
                self.data[si + j - fi] = if djj != 0.0 { sum / djj } else { 0.0 };
            }
            let mut d = self.data[si + i - fi];
            let orig = d;
            for k in fi..i {
                let lik = self.data[si + k - fi];
                d -= lik * lik * self.data[self.start[k] + k - self.first[k]];
            }
            if d <= soft * 1e-3 || d <= orig.abs() * 1e-12 {
                loose.push(i);
                d = soft;
            }
            self.data[si + i - fi] = d;
        }
        loose
    }

    fn solve(&self, b: &mut [f64]) {
        let n = self.first.len();
        // L y = b.
        for i in 0..n {
            let fi = self.first[i];
            let si = self.start[i];
            let mut s = b[i];
            for k in fi..i {
                s -= self.data[si + k - fi] * b[k];
            }
            b[i] = s;
        }
        // D z = y.
        for (i, bi) in b.iter_mut().enumerate() {
            *bi /= self.data[self.start[i] + i - self.first[i]];
        }
        // Lᵀ x = z.
        for i in (0..n).rev() {
            let fi = self.first[i];
            let si = self.start[i];
            let xi = b[i];
            for k in fi..i {
                b[k] -= self.data[si + k - fi] * xi;
            }
        }
    }
}

/// A node order that keeps the matrix narrow: reverse Cuthill–McKee.
fn rcm(n: usize, bars: &[Bar]) -> Vec<usize> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for b in bars {
        if b.i != b.j {
            adj[b.i].push(b.j);
            adj[b.j].push(b.i);
        }
    }
    for a in &mut adj {
        a.sort_unstable();
        a.dedup();
    }
    let mut order = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    let mut by_degree: Vec<usize> = (0..n).collect();
    by_degree.sort_by_key(|&i| adj[i].len());
    for &s in &by_degree {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let mut queue = std::collections::VecDeque::from([s]);
        while let Some(v) = queue.pop_front() {
            order.push(v);
            let mut next: Vec<usize> = adj[v].iter().copied().filter(|&u| !seen[u]).collect();
            next.sort_by_key(|&u| adj[u].len());
            for u in next {
                seen[u] = true;
                queue.push_back(u);
            }
        }
    }
    order.reverse();
    let mut pos = vec![0; n];
    for (k, &v) in order.iter().enumerate() {
        pos[v] = k;
    }
    pos
}

/// The solved frame: its stiffness factored once, for any number of
/// cases.
pub struct Solver<'a> {
    frame: &'a Frame,
    /// Equation of each node's degree of freedom (`None`: held).
    eq: Vec<[Option<usize>; 6]>,
    k: Skyline,
    loose: Vec<(usize, usize)>,
    /// Each bar's condensed local stiffness and axes.
    bars: Vec<([[f64; 12]; 12], [V3; 3], f64)>,
}

impl<'a> Solver<'a> {
    pub fn new(frame: &'a Frame) -> Solver<'a> {
        let n = frame.nodes.len();
        let mut held = vec![[false; 6]; n];
        for (node, f) in &frame.supports {
            if *node < n {
                for d in 0..6 {
                    held[*node][d] |= f[d];
                }
            }
        }
        let pos = rcm(n, &frame.bars);
        let mut by_pos: Vec<usize> = (0..n).collect();
        by_pos.sort_by_key(|&i| pos[i]);
        let mut eq = vec![[None; 6]; n];
        let mut m = 0;
        for &node in &by_pos {
            for d in 0..6 {
                if !held[node][d] {
                    eq[node][d] = Some(m);
                    m += 1;
                }
            }
        }
        // The profile.
        let mut first: Vec<usize> = (0..m).collect();
        let dofs = |node: usize, eq: &Vec<[Option<usize>; 6]>| -> Vec<usize> {
            eq[node].iter().flatten().copied().collect()
        };
        for b in &frame.bars {
            let mut all = dofs(b.i, &eq);
            all.extend(dofs(b.j, &eq));
            if let Some(lo) = all.iter().min().copied() {
                for &d in &all {
                    first[d] = first[d].min(lo);
                }
            }
        }
        let mut k = Skyline::new(first);
        let mut bars = Vec::with_capacity(frame.bars.len());
        let mut biggest: f64 = 0.0;
        for b in &frame.bars {
            let ax = axes(frame, b);
            let l = length(frame, b);
            let mut kl = local_stiffness(&b.section, l.max(1e-6));
            let mut dummy = [0.0; 12];
            condense(&mut kl, &mut dummy, &released(b));
            // Global: K = Tᵀ k T, T = blocks of the axes.
            let t = |r: usize, c: usize| -> f64 {
                // Row r of T (local), column c (global).
                if r / 3 != c / 3 {
                    0.0
                } else {
                    ax[r % 3][c % 3]
                }
            };
            let mut kg = [[0.0; 12]; 12];
            let mut tmp = [[0.0; 12]; 12];
            for i in 0..12 {
                for j in 0..12 {
                    let mut s = 0.0;
                    for p in 0..12 {
                        let tv = t(p, j);
                        if tv != 0.0 {
                            s += kl[i][p] * tv;
                        }
                    }
                    tmp[i][j] = s;
                }
            }
            for i in 0..12 {
                for j in 0..12 {
                    let mut s = 0.0;
                    for p in 0..12 {
                        let tv = t(p, i);
                        if tv != 0.0 {
                            s += tv * tmp[p][j];
                        }
                    }
                    kg[i][j] = s;
                }
            }
            let nodes = [b.i, b.j];
            for a in 0..12 {
                let Some(ea) = eq[nodes[a / 6]][a % 6] else {
                    continue;
                };
                for c in 0..=a {
                    let Some(ec) = eq[nodes[c / 6]][c % 6] else {
                        continue;
                    };
                    if ea == ec && a != c {
                        // Both ends on one node.
                        *k.at(ea, ec) += 2.0 * kg[a][c];
                    } else {
                        *k.at(ea, ec) += kg[a][c];
                    }
                    if a == c {
                        biggest = biggest.max(kg[a][a].abs());
                    }
                }
            }
            bars.push((kl, ax, l));
        }
        for (node, s) in &frame.springs {
            for d in 0..6 {
                if let Some(e) = eq.get(*node).and_then(|q| q[d]) {
                    *k.at(e, e) += s[d];
                }
            }
        }
        let soft = (biggest * 1e-9).max(1e-6);
        let loose_eq = k.factor(soft);
        let mut loose = Vec::new();
        for (node, q) in eq.iter().enumerate() {
            for (d, e) in q.iter().enumerate() {
                if e.is_some_and(|e| loose_eq.contains(&e)) {
                    loose.push((node, d));
                }
            }
        }
        Solver {
            frame,
            eq,
            k,
            loose,
            bars,
        }
    }

    /// Solves one case.
    pub fn solve(&self, loads: &[Load]) -> Solution {
        let frame = self.frame;
        let n = frame.nodes.len();
        let m = self.k.first.len();
        let mut f = vec![0.0; m];
        // Fixed-end loads of each bar, local (after condensation).
        let mut fixed: Vec<[f64; 12]> = vec![[0.0; 12]; frame.bars.len()];
        let mut nodal = vec![[0.0; 6]; n];
        for load in loads {
            match load {
                Load::Node { node, f: v } => {
                    if *node < n {
                        for d in 0..6 {
                            nodal[*node][d] += v[d];
                        }
                    }
                }
                Load::Spread { bar, from, to, w } => {
                    if let Some((_, ax, l)) = self.bars.get(*bar) {
                        let wl = to_local(ax, *w);
                        let fe = fixed_end(*l, *from, *to, wl);
                        for d in 0..12 {
                            fixed[*bar][d] += fe[d];
                        }
                    }
                }
                Load::Point { bar, at, f: v } => {
                    if let Some((_, ax, l)) = self.bars.get(*bar) {
                        let pl = to_local(ax, *v);
                        let mut fe = [0.0; 12];
                        let x = at.clamp(0.0, *l);
                        add_point(&mut fe, &shape(*l, x), pl, 1.0);
                        for d in 0..12 {
                            fixed[*bar][d] += fe[d];
                        }
                    }
                }
            }
        }
        // Condense the fixed-end loads with each bar's releases.
        for (bi, b) in frame.bars.iter().enumerate() {
            let rel = released(b);
            if rel.is_empty() {
                continue;
            }
            let l = self.bars[bi].2;
            let mut kl = local_stiffness(&b.section, l.max(1e-6));
            condense(&mut kl, &mut fixed[bi], &rel);
        }
        for (node, v) in nodal.iter().enumerate() {
            for d in 0..6 {
                if let Some(e) = self.eq[node][d] {
                    f[e] += v[d];
                }
            }
        }
        for (bi, b) in frame.bars.iter().enumerate() {
            let ax = &self.bars[bi].1;
            let fe = &fixed[bi];
            let nodes = [b.i, b.j];
            for end in 0..2 {
                let force = to_global(ax, [fe[end * 6], fe[end * 6 + 1], fe[end * 6 + 2]]);
                let moment = to_global(ax, [fe[end * 6 + 3], fe[end * 6 + 4], fe[end * 6 + 5]]);
                for d in 0..3 {
                    if let Some(e) = self.eq[nodes[end]][d] {
                        f[e] += force[d];
                    }
                    if let Some(e) = self.eq[nodes[end]][d + 3] {
                        f[e] += moment[d];
                    }
                }
            }
        }
        self.k.solve(&mut f);
        let mut u = vec![[0.0; 6]; n];
        for node in 0..n {
            for d in 0..6 {
                if let Some(e) = self.eq[node][d] {
                    u[node][d] = f[e];
                }
            }
        }
        // End forces: k (T u) − fixed.
        let mut end_forces = Vec::with_capacity(frame.bars.len());
        let mut node_force = vec![[0.0; 6]; n];
        for (bi, b) in frame.bars.iter().enumerate() {
            let (kl, ax, _) = &self.bars[bi];
            let mut ul = [0.0; 12];
            for (end, node) in [b.i, b.j].into_iter().enumerate() {
                let t = to_local(ax, [u[node][0], u[node][1], u[node][2]]);
                let r = to_local(ax, [u[node][3], u[node][4], u[node][5]]);
                ul[end * 6..end * 6 + 3].copy_from_slice(&t);
                ul[end * 6 + 3..end * 6 + 6].copy_from_slice(&r);
            }
            let mut ef = [0.0; 12];
            for i in 0..12 {
                let mut s = 0.0;
                for j in 0..12 {
                    s += kl[i][j] * ul[j];
                }
                ef[i] = s - fixed[bi][i];
            }
            for (end, node) in [b.i, b.j].into_iter().enumerate() {
                let fg = to_global(ax, [ef[end * 6], ef[end * 6 + 1], ef[end * 6 + 2]]);
                let mg = to_global(ax, [ef[end * 6 + 3], ef[end * 6 + 4], ef[end * 6 + 5]]);
                for d in 0..3 {
                    node_force[node][d] += fg[d];
                    node_force[node][d + 3] += mg[d];
                }
            }
            end_forces.push(ef);
        }
        let mut reactions = Vec::new();
        for (node, fx) in &frame.supports {
            let mut r = [0.0; 6];
            for d in 0..6 {
                if fx[d] {
                    r[d] = node_force[*node][d] - nodal[*node][d];
                }
            }
            reactions.push((*node, r));
        }
        Solution {
            displacements: u,
            end_forces,
            reactions,
            loose: self.loose.clone(),
        }
    }

    /// The loads over a bar of a case, local, for its internal forces.
    pub fn bar_loads(&self, loads: &[Load], bar: usize) -> Vec<(f64, f64, V3)> {
        let Some((_, ax, l)) = self.bars.get(bar) else {
            return Vec::new();
        };
        loads
            .iter()
            .filter_map(|ld| match ld {
                Load::Spread {
                    bar: b,
                    from,
                    to,
                    w,
                } if *b == bar => Some((from.clamp(0.0, *l), to.clamp(0.0, *l), to_local(ax, *w))),
                Load::Point { bar: b, at, f } if *b == bar => {
                    let x = at.clamp(0.0, *l);
                    // A point as a zero-length spread of total force.
                    Some((x, x, to_local(ax, *f)))
                }
                _ => None,
            })
            .collect()
    }

    pub fn bar_length(&self, bar: usize) -> f64 {
        self.bars.get(bar).map_or(0.0, |b| b.2)
    }

    pub fn bar_axes(&self, bar: usize) -> [V3; 3] {
        self.bars.get(bar).map_or([[0.0; 3]; 3], |b| b.1)
    }
}

/// The internal forces of a bar at `x` metres from its first end, from its
/// end forces and its loads (local; a zero-length load is a point force):
/// `[N, Vy, Vz, T, My, Mz]`, with the sign of the forces the part beyond
/// `x` does on the part before it.
pub fn internal(end_i: &[f64; 12], loads: &[(f64, f64, V3)], x: f64) -> [f64; 6] {
    // Forces on the part [0, x]: end i forces plus loads before x; the
    // section's forces balance them.
    let mut f = [end_i[0], end_i[1], end_i[2]];
    let mut m = [end_i[3], end_i[4], end_i[5]];
    // Moment about the section at x of the end force at 0: r = (−x, 0, 0).
    let rm = |m: &mut [f64; 3], r: f64, fy: f64, fz: f64| {
        // r × F with r = (r, 0, 0): (0, −r·Fz, r·Fy).
        m[1] += -r * fz;
        m[2] += r * fy;
    };
    rm(&mut m, -x, end_i[1], end_i[2]);
    for &(a, b, w) in loads {
        if a >= x {
            continue;
        }
        let (s, e, total) = if (b - a).abs() < 1e-12 {
            (a, a, w)
        } else {
            let e = b.min(x);
            let len = e - a;
            (a, e, [w[0] * len, w[1] * len, w[2] * len])
        };
        let c = (s + e) / 2.0;
        f[0] += total[0];
        f[1] += total[1];
        f[2] += total[2];
        rm(&mut m, c - x, total[1], total[2]);
    }
    // The section's internal force is minus what acts on [0, x]: the sign
    // convention of the results: tension positive N, and moments that sag
    // a beam positive about its strong axis.
    [-f[0], f[1], f[2], -m[0], m[1], -m[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steel_box() -> Section {
        // 0,1 × 0,2 m solid steel.
        let (b, h) = (0.1, 0.2);
        Section {
            a: b * h,
            iy: b * h * h * h / 12.0,
            iz: h * b * b * b / 12.0,
            j: 0.0002,
            e: 210e6,
            g: 81e6,
        }
    }

    fn beam(l: f64, hinge: (bool, bool)) -> Frame {
        Frame {
            nodes: vec![[0.0, 0.0, 0.0], [l, 0.0, 0.0]],
            bars: vec![Bar {
                i: 0,
                j: 1,
                section: steel_box(),
                up: [0.0, 0.0, 1.0],
                hinge_i: hinge.0,
                hinge_j: hinge.1,
            }],
            supports: Vec::new(),
            springs: Vec::new(),
        }
    }

    #[test]
    fn a_cantilever_bends_as_in_the_books() {
        let mut f = beam(3.0, (false, false));
        f.supports.push((0, FIXED));
        let s = Solver::new(&f);
        let p = 10.0;
        let sol = s.solve(&[Load::Node {
            node: 1,
            f: [0.0, 0.0, -p, 0.0, 0.0, 0.0],
        }]);
        let sec = steel_box();
        let expect = p * 27.0 / (3.0 * sec.e * sec.iy);
        assert!(
            (sol.displacements[1][2] + expect).abs() < 1e-9,
            "{:?}",
            sol.displacements[1]
        );
        // The reaction holds it: 10 kN up and 30 kN·m.
        let r = sol.reactions[0].1;
        assert!((r[2] - p).abs() < 1e-6);
        assert!((r[4].abs() - 30.0).abs() < 1e-6, "{r:?}");
        assert!(sol.loose.is_empty());
        // The moment at the root is 30 kN·m, hogging (negative).
        let m0 = internal(&sol.end_forces[0], &[], 0.0);
        assert!((m0[4] + 30.0).abs() < 1e-6, "{m0:?}");
    }

    #[test]
    fn a_simple_beam_under_a_spread_load() {
        // Pinned at one end, a roller at the other; torsion held.
        let mut f = beam(6.0, (false, false));
        f.supports.push((0, [true, true, true, true, false, false]));
        f.supports
            .push((1, [false, true, true, false, false, false]));
        let s = Solver::new(&f);
        let w = 5.0;
        let loads = [Load::Spread {
            bar: 0,
            from: 0.0,
            to: 6.0,
            w: [0.0, 0.0, -w],
        }];
        let sol = s.solve(&loads);
        let bl = s.bar_loads(&loads, 0);
        let mid = internal(&sol.end_forces[0], &bl, 3.0);
        assert!((mid[4] - w * 36.0 / 8.0).abs() < 1e-6, "{mid:?}");
        let v0 = internal(&sol.end_forces[0], &bl, 1e-9);
        assert!((v0[2].abs() - 15.0).abs() < 1e-6, "{v0:?}");
        // The mid-span deflection: 5 w L⁴ / 384 EI (from the nodes and the
        // shape: the solver gives end turns; check with a split beam).
        let mut f2 = Frame {
            nodes: vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [6.0, 0.0, 0.0]],
            bars: vec![
                Bar {
                    i: 0,
                    j: 1,
                    section: steel_box(),
                    up: [0.0, 0.0, 1.0],
                    hinge_i: false,
                    hinge_j: false,
                },
                Bar {
                    i: 1,
                    j: 2,
                    section: steel_box(),
                    up: [0.0, 0.0, 1.0],
                    hinge_i: false,
                    hinge_j: false,
                },
            ],
            supports: vec![
                (0, [true, true, true, true, false, false]),
                (2, [false, true, true, false, false, false]),
            ],
            springs: Vec::new(),
        };
        f2.springs.clear();
        let s2 = Solver::new(&f2);
        let sol2 = s2.solve(&[
            Load::Spread {
                bar: 0,
                from: 0.0,
                to: 3.0,
                w: [0.0, 0.0, -w],
            },
            Load::Spread {
                bar: 1,
                from: 0.0,
                to: 3.0,
                w: [0.0, 0.0, -w],
            },
        ]);
        let sec = steel_box();
        let expect = 5.0 * w * 6f64.powi(4) / (384.0 * sec.e * sec.iy);
        assert!(
            (sol2.displacements[1][2] + expect).abs() < 1e-9 * 1e3,
            "{} {expect}",
            sol2.displacements[1][2]
        );
    }

    #[test]
    fn a_hinge_frees_the_moment() {
        // Fixed at one end, hinged at the other end on a pin: a propped
        // cantilever under w: reaction at the pin 3 w L / 8.
        let mut f = beam(4.0, (false, true));
        f.supports.push((0, FIXED));
        f.supports.push((1, [true, true, true, true, false, false]));
        let s = Solver::new(&f);
        let w = 2.0;
        let sol = s.solve(&[Load::Spread {
            bar: 0,
            from: 0.0,
            to: 4.0,
            w: [0.0, 0.0, -w],
        }]);
        let r1 = sol.reactions.iter().find(|r| r.0 == 1).unwrap().1;
        assert!((r1[2] - 3.0 * w * 4.0 / 8.0).abs() < 1e-6, "{r1:?}");
        // The moment at the hinge is nothing.
        assert!(sol.end_forces[0][10].abs() < 1e-6 && sol.end_forces[0][11].abs() < 1e-6);
    }

    #[test]
    fn a_portal_sways_and_balances() {
        // Two columns 3 m, a beam 5 m, fixed bases; 10 kN sideways at the
        // top, 20 kN/m on the beam.
        let col = Section {
            a: 0.09,
            iy: 0.3f64.powi(4) / 12.0,
            iz: 0.3f64.powi(4) / 12.0,
            j: 0.141 * 0.3f64.powi(4),
            e: 27e6,
            g: 11e6,
        };
        let f = Frame {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 3.0],
                [5.0, 0.0, 3.0],
                [5.0, 0.0, 0.0],
            ],
            bars: vec![
                Bar {
                    i: 0,
                    j: 1,
                    section: col,
                    up: [1.0, 0.0, 0.0],
                    hinge_i: false,
                    hinge_j: false,
                },
                Bar {
                    i: 1,
                    j: 2,
                    section: col,
                    up: [0.0, 0.0, 1.0],
                    hinge_i: false,
                    hinge_j: false,
                },
                Bar {
                    i: 3,
                    j: 2,
                    section: col,
                    up: [1.0, 0.0, 0.0],
                    hinge_i: false,
                    hinge_j: false,
                },
            ],
            supports: vec![(0, FIXED), (3, FIXED)],
            springs: Vec::new(),
        };
        let s = Solver::new(&f);
        let loads = [
            Load::Node {
                node: 1,
                f: [10.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
            Load::Spread {
                bar: 1,
                from: 0.0,
                to: 5.0,
                w: [0.0, 0.0, -20.0],
            },
        ];
        let sol = s.solve(&loads);
        let sum = sol.reactions.iter().fold([0.0; 6], |mut a, (_, r)| {
            for d in 0..6 {
                a[d] += r[d];
            }
            a
        });
        assert!((sum[0] + 10.0).abs() < 1e-6, "{sum:?}");
        assert!((sum[2] - 100.0).abs() < 1e-6, "{sum:?}");
        assert!(sol.displacements[1][0] > 0.0 && sol.displacements[2][0] > 0.0);
        assert!(sol.loose.is_empty());
        // Symmetric vertical load alone: equal vertical reactions.
        let sol = s.solve(&loads[1..]);
        assert!((sol.reactions[0].1[2] - 50.0).abs() < 1e-6);
    }

    #[test]
    fn a_mechanism_is_held_and_told() {
        // A bar hinged at both ends on two pins turns freely about itself
        // and about its supports: reported, not a crash.
        let mut f = beam(2.0, (true, true));
        f.supports.push((0, PINNED));
        f.supports.push((1, PINNED));
        let s = Solver::new(&f);
        let sol = s.solve(&[Load::Point {
            bar: 0,
            at: 1.0,
            f: [0.0, 0.0, -4.0],
        }]);
        assert!(!sol.loose.is_empty());
        let r: f64 = sol.reactions.iter().map(|r| r.1[2]).sum();
        assert!((r - 4.0).abs() < 1e-6, "{r}");
    }
}
