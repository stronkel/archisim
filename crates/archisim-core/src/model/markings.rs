//! Road markings at will (U4, after Spain's «Norma 8.2-IC Marcas viales»):
//! lines along a path (continuous, dashed, double, continuous with dashed,
//! edge, stop and give-way lines), pedestrian crossings across a road
//! (zebra, with signals, raised), arrows, the «STOP», give-way triangle and
//! «BUS» on the pavement, hatched islands («cebreado») and parking bays
//! (disabled, loading, motorbikes). Each marking is a path drawn on the
//! plan; its paint is worked out from it ([`Marking::shapes`]) and lies on
//! whatever pavement is under it.

use super::*;

/// What a marking is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkingKind {
    /// M-2.2: a continuous line.
    Continuous,
    /// M-1.x: a dashed line (`dash` and `gap`).
    Dashed,
    /// M-2.3: two continuous lines.
    DoubleContinuous,
    /// M-3.1: continuous on the left, dashed on the right.
    ContinuousDashed,
    /// M-2.6: the edge of the carriageway.
    Edge,
    /// M-4.1: the stop line, 40 cm.
    StopLine,
    /// M-4.2: the give-way line, dashes of 80 cm.
    GiveWayLine,
    /// M-4.3: a zebra crossing, across the road from the first point to
    /// the second, `depth` along it.
    Zebra,
    /// A crossing with traffic lights: two dashed lines.
    SignalCrossing,
    /// A raised crossing: a zebra on a plateau with ramps.
    RaisedCrossing,
    /// M-5.2: arrows, at the first point pointing to the second.
    ArrowStraight,
    ArrowLeft,
    ArrowRight,
    ArrowStraightLeft,
    ArrowStraightRight,
    /// M-6.4: «STOP».
    Stop,
    /// M-6.5: the give-way triangle.
    GiveWay,
    /// «BUS».
    Bus,
    /// M-7.1: a hatched island, the path its outline.
    Chevrons,
    /// A parking bay for disabled people, the path its outline.
    DisabledBay,
    /// A loading bay: a zigzag along the path.
    LoadingBay,
    /// A bay for motorbikes, the path its outline.
    MotoBay,
}

/// How a marking is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkingPlace {
    /// Along a path of two points or more.
    Line,
    /// Two points across the road.
    Across,
    /// A point and a direction (two points).
    Symbol,
    /// An outline of three points or more.
    Area,
}

impl MarkingKind {
    pub const ALL: [MarkingKind; 22] = [
        MarkingKind::Continuous,
        MarkingKind::Dashed,
        MarkingKind::DoubleContinuous,
        MarkingKind::ContinuousDashed,
        MarkingKind::Edge,
        MarkingKind::StopLine,
        MarkingKind::GiveWayLine,
        MarkingKind::Zebra,
        MarkingKind::SignalCrossing,
        MarkingKind::RaisedCrossing,
        MarkingKind::ArrowStraight,
        MarkingKind::ArrowLeft,
        MarkingKind::ArrowRight,
        MarkingKind::ArrowStraightLeft,
        MarkingKind::ArrowStraightRight,
        MarkingKind::Stop,
        MarkingKind::GiveWay,
        MarkingKind::Bus,
        MarkingKind::Chevrons,
        MarkingKind::DisabledBay,
        MarkingKind::LoadingBay,
        MarkingKind::MotoBay,
    ];

    pub fn name(self) -> &'static str {
        use MarkingKind::*;
        match self {
            Continuous => "Línea continua",
            Dashed => "Línea discontinua",
            DoubleContinuous => "Línea continua doble",
            ContinuousDashed => "Continua y discontinua",
            Edge => "Línea de borde",
            StopLine => "Línea de detención (STOP)",
            GiveWayLine => "Línea de ceda el paso",
            Zebra => "Paso de peatones (cebra)",
            SignalCrossing => "Paso con semáforo",
            RaisedCrossing => "Paso sobreelevado",
            ArrowStraight => "Flecha de frente",
            ArrowLeft => "Flecha a la izquierda",
            ArrowRight => "Flecha a la derecha",
            ArrowStraightLeft => "Flecha de frente e izquierda",
            ArrowStraightRight => "Flecha de frente y derecha",
            Stop => "STOP",
            GiveWay => "Triángulo de ceda el paso",
            Bus => "BUS",
            Chevrons => "Cebreado",
            DisabledBay => "Plaza de movilidad reducida",
            LoadingBay => "Carga y descarga",
            MotoBay => "Plaza de motos",
        }
    }

    pub fn code(self) -> &'static str {
        use MarkingKind::*;
        match self {
            Continuous => "continuous",
            Dashed => "dashed",
            DoubleContinuous => "double",
            ContinuousDashed => "continuous_dashed",
            Edge => "edge",
            StopLine => "stop_line",
            GiveWayLine => "give_way_line",
            Zebra => "zebra",
            SignalCrossing => "signal_crossing",
            RaisedCrossing => "raised_crossing",
            ArrowStraight => "arrow",
            ArrowLeft => "arrow_left",
            ArrowRight => "arrow_right",
            ArrowStraightLeft => "arrow_straight_left",
            ArrowStraightRight => "arrow_straight_right",
            Stop => "stop",
            GiveWay => "give_way",
            Bus => "bus",
            Chevrons => "chevrons",
            DisabledBay => "disabled_bay",
            LoadingBay => "loading_bay",
            MotoBay => "moto_bay",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    pub fn place(self) -> MarkingPlace {
        use MarkingKind::*;
        match self {
            Continuous | Dashed | DoubleContinuous | ContinuousDashed | Edge | StopLine
            | GiveWayLine | LoadingBay => MarkingPlace::Line,
            Zebra | SignalCrossing | RaisedCrossing => MarkingPlace::Across,
            ArrowStraight | ArrowLeft | ArrowRight | ArrowStraightLeft | ArrowStraightRight
            | Stop | GiveWay | Bus => MarkingPlace::Symbol,
            Chevrons | DisabledBay | MotoBay => MarkingPlace::Area,
        }
    }

    /// The group it is shown in.
    pub fn group(self) -> &'static str {
        match self.place() {
            MarkingPlace::Line => "Líneas",
            MarkingPlace::Across => "Pasos de peatones",
            MarkingPlace::Symbol => "Flechas y símbolos",
            MarkingPlace::Area => "Cebreados y plazas",
        }
    }

    /// Its usual line width, dash and gap.
    pub fn defaults(self) -> (f64, f64, f64) {
        use MarkingKind::*;
        match self {
            Dashed => (0.10, 2.0, 5.5),
            ContinuousDashed => (0.10, 2.0, 5.5),
            Edge => (0.15, 0.0, 0.0),
            StopLine => (0.40, 0.0, 0.0),
            GiveWayLine => (0.40, 0.8, 0.4),
            Zebra | RaisedCrossing => (0.50, 0.5, 0.5),
            SignalCrossing => (0.40, 0.4, 0.4),
            Chevrons => (0.15, 0.6, 1.0),
            LoadingBay => (0.15, 0.0, 0.0),
            _ => (0.10, 0.0, 0.0),
        }
    }

    /// Whether it is painted yellow by default (loading bays) or blue
    /// (disabled bays); white otherwise.
    pub fn colour(self) -> [u8; 3] {
        match self {
            MarkingKind::LoadingBay => [236, 196, 40],
            MarkingKind::DisabledBay => [40, 96, 176],
            _ => [245, 245, 240],
        }
    }
}

/// A road marking.
#[derive(Debug, Clone, PartialEq)]
pub struct Marking {
    pub id: MarkingId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub kind: MarkingKind,
    pub path: Vec<Point2>,
    /// Width of its lines (or of a zebra's bars).
    pub width: f64,
    /// Length of its dashes and of the gaps between them.
    pub dash: f64,
    pub gap: f64,
    /// A crossing's depth along the road (its width for people).
    pub depth: f64,
    pub material: MaterialId,
}

/// Data needed to make a marking.
#[derive(Debug, Clone, PartialEq)]
pub struct NewMarking {
    pub level: LevelId,
    pub kind: MarkingKind,
    pub path: Vec<Point2>,
    pub width: f64,
    pub dash: f64,
    pub gap: f64,
    pub depth: f64,
    pub material: MaterialId,
}

impl NewMarking {
    /// A marking of `kind` along `path` with its usual sizes.
    pub fn usual(
        level: LevelId,
        kind: MarkingKind,
        path: Vec<Point2>,
        material: MaterialId,
    ) -> Self {
        let (width, dash, gap) = kind.defaults();
        NewMarking {
            level,
            kind,
            path,
            width,
            dash,
            gap,
            depth: 4.0,
            material,
        }
    }
}

fn len_of(path: &[Point2]) -> f64 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Dashes of `dash` every `dash + gap` along a path, as polygons from `a`
/// to `b` to its left.
fn dashes(path: &[Point2], a: f64, b: f64, dash: f64, gap: f64) -> Vec<Vec<Point2>> {
    let len = len_of(path);
    let mut out = Vec::new();
    if dash <= EPSILON {
        return out;
    }
    let mut t = 0.0;
    while t < len - 1e-6 {
        let e = (t + dash).min(len);
        out.push(super::urban::quad(path, t, e, a, b));
        t += dash + gap.max(0.05);
    }
    out
}

/// Where a line crosses a polygon's inside: (from, to) along `a → b`.
fn inside_spans(poly: &[Point2], a: Point2, b: Point2) -> Vec<(f64, f64)> {
    let d = b - a;
    let mut ts = Vec::new();
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let e = q - p;
        let den = d.cross(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (p - a).cross(e) / den;
        let u = (p - a).cross(d) / den;
        if (0.0..1.0).contains(&u) {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0)))
        .filter(|(x, y)| y - x > 1e-6)
        .collect()
}

/// A shape drawn in a frame: `x` along the direction, `y` to its left.
fn placed(origin: Point2, u: Vec2, pts: &[(f64, f64)]) -> Vec<Point2> {
    let n = u.perp_left();
    pts.iter().map(|&(x, y)| origin + u * x + n * y).collect()
}

/// The letters of «STOP» and «BUS», 2,4 m long along the road and 0,8 m
/// across, made of bars (x along the road from the letter's foot, y
/// across from its left edge).
fn letter(c: char) -> Vec<[(f64, f64); 4]> {
    let (h, w, t) = (2.4, 0.8, 0.16);
    let bar = |x0: f64, x1: f64, y0: f64, y1: f64| [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    match c {
        'S' => vec![
            bar(h - t, h, 0.0, w),
            bar(h / 2.0, h, w - t, w),
            bar(h / 2.0 - t / 2.0, h / 2.0 + t / 2.0, 0.0, w),
            bar(0.0, h / 2.0, 0.0, t),
            bar(0.0, t, 0.0, w),
        ],
        'T' => vec![
            bar(h - t, h, 0.0, w),
            bar(0.0, h, w / 2.0 - t / 2.0, w / 2.0 + t / 2.0),
        ],
        'O' => vec![
            bar(0.0, t, 0.0, w),
            bar(h - t, h, 0.0, w),
            bar(0.0, h, 0.0, t),
            bar(0.0, h, w - t, w),
        ],
        'P' => vec![
            bar(0.0, h, w - t, w),
            bar(h - t, h, 0.0, w),
            bar(h / 2.0, h, 0.0, t),
            bar(h / 2.0 - t / 2.0, h / 2.0 + t / 2.0, 0.0, w),
        ],
        'B' => vec![
            bar(0.0, h, w - t, w),
            bar(h - t, h, 0.0, w),
            bar(h / 2.0 - t / 2.0, h / 2.0 + t / 2.0, 0.0, w),
            bar(0.0, t, 0.0, w),
            bar(0.0, h, 0.0, t),
        ],
        'U' => vec![
            bar(0.0, h, w - t, w),
            bar(0.0, h, 0.0, t),
            bar(0.0, t, 0.0, w),
        ],
        _ => Vec::new(),
    }
}

/// An arrow 5 m long (M-5.2, urban), its tip at `x = 5`: the shaft and the
/// heads it has.
fn arrow(kind: MarkingKind) -> Vec<Vec<(f64, f64)>> {
    let shaft = |x0: f64, x1: f64| vec![(x0, -0.075), (x1, -0.075), (x1, 0.075), (x0, 0.075)];
    let head = |x: f64| vec![(x - 1.2, -0.45), (x, 0.0), (x - 1.2, 0.45)];
    // A turning arm: from the shaft at 2 m out to the side (`s` 1 left,
    // −1 right), with its head.
    let side = |s: f64| {
        let (a, b) = ((2.0, 0.0), (3.2, 1.0 * s));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = (dx / l, dy / l);
        let (nx, ny) = (-uy, ux);
        let w = 0.075;
        let arm = vec![
            (a.0 - nx * w, a.1 - ny * w),
            (b.0 - nx * w, b.1 - ny * w),
            (b.0 + nx * w, b.1 + ny * w),
            (a.0 + nx * w, a.1 + ny * w),
        ];
        let hw = 0.4;
        let tip = (b.0 + ux * 1.0, b.1 + uy * 1.0);
        let head = vec![
            (b.0 - nx * hw, b.1 - ny * hw),
            tip,
            (b.0 + nx * hw, b.1 + ny * hw),
        ];
        vec![arm, head]
    };
    use MarkingKind::*;
    let mut out = Vec::new();
    match kind {
        ArrowStraight => {
            out.push(shaft(0.0, 3.8));
            out.push(head(5.0));
        }
        ArrowLeft | ArrowRight => {
            let s = if kind == ArrowLeft { 1.0 } else { -1.0 };
            out.push(shaft(0.0, 2.2));
            out.extend(side(s));
        }
        ArrowStraightLeft | ArrowStraightRight => {
            let s = if kind == ArrowStraightLeft { 1.0 } else { -1.0 };
            out.push(shaft(0.0, 3.8));
            out.push(head(5.0));
            out.extend(side(s));
        }
        _ => {}
    }
    // Counter-clockwise, whatever side.
    out.into_iter()
        .map(|mut p| {
            let a: f64 = (0..p.len())
                .map(|i| {
                    let (x0, y0) = p[i];
                    let (x1, y1) = p[(i + 1) % p.len()];
                    x0 * y1 - x1 * y0
                })
                .sum();
            if a < 0.0 {
                p.reverse();
            }
            p
        })
        .collect()
}

impl Marking {
    /// Its paint, as polygons on the plan.
    pub fn shapes(&self) -> Vec<Vec<Point2>> {
        use MarkingKind::*;
        let p = &self.path;
        if p.len() < 2 {
            return Vec::new();
        }
        let w = self.width.max(0.02);
        let h = w / 2.0;
        let len = len_of(p);
        match self.kind {
            Continuous | Edge | StopLine => vec![strip(p, -h, h)],
            Dashed => dashes(p, -h, h, self.dash, self.gap),
            GiveWayLine => dashes(p, -h, h, self.dash, self.gap),
            DoubleContinuous => vec![strip(p, 0.05, 0.05 + w), strip(p, -0.05 - w, -0.05)],
            ContinuousDashed => {
                let mut v = vec![strip(p, 0.05, 0.05 + w)];
                v.extend(dashes(p, -0.05 - w, -0.05, self.dash, self.gap));
                v
            }
            LoadingBay => {
                // A zigzag along it, 1 m to the side.
                let mut out = Vec::new();
                let n = (len / 1.0).ceil().max(1.0) as usize;
                let mut pts = Vec::new();
                for k in 0..=n {
                    let s = len * k as f64 / n as f64;
                    let (q, u) = super::urban::axis_point(p, s);
                    pts.push(q + u.perp_left() * if k % 2 == 0 { 0.0 } else { -1.0 });
                }
                for wseg in pts.windows(2) {
                    out.push(strip(wseg, -h, h));
                }
                out
            }
            Zebra | RaisedCrossing | SignalCrossing => {
                let (a, b) = (p[0], p[p.len() - 1]);
                let across = b - a;
                let Some(v) = across.normalized() else {
                    return Vec::new();
                };
                // Along the road: to the left of «across».
                let along = v.perp_left() * -1.0;
                let d = self.depth.max(0.5);
                let width = across.length();
                if self.kind == SignalCrossing {
                    let mut out = Vec::new();
                    for off in [-d / 2.0, d / 2.0] {
                        let line = [a + along * off, b + along * off];
                        out.extend(dashes(&line, -h, h, self.dash, self.gap));
                    }
                    return out;
                }
                // Bars along the road, 0,5 m every metre across it.
                let mut out = Vec::new();
                let step = self.width + self.gap.max(0.1);
                let mut t = (width - ((width / step).floor() * step - self.gap)) / 2.0;
                while t + self.width <= width + 1e-6 {
                    let c0 = a + v * t;
                    let c1 = a + v * (t + self.width);
                    out.push(vec![
                        c0 - along * (d / 2.0),
                        c1 - along * (d / 2.0),
                        c1 + along * (d / 2.0),
                        c0 + along * (d / 2.0),
                    ]);
                    t += step;
                }
                out
            }
            ArrowStraight | ArrowLeft | ArrowRight | ArrowStraightLeft | ArrowStraightRight => {
                let u = (p[1] - p[0]).normalized().unwrap_or(Vec2::new(1.0, 0.0));
                arrow(self.kind)
                    .iter()
                    .map(|shape| placed(p[0], u, shape))
                    .collect()
            }
            Stop | Bus => {
                // Read from the driver's seat: the letters go across, the
                // first on the left, each long along the road.
                let u = (p[1] - p[0]).normalized().unwrap_or(Vec2::new(1.0, 0.0));
                let word: &[char] = if self.kind == Stop {
                    &['S', 'T', 'O', 'P']
                } else {
                    &['B', 'U', 'S']
                };
                let (lw, sp) = (0.8, 0.25);
                let total = word.len() as f64 * lw + (word.len() - 1) as f64 * sp;
                let mut out = Vec::new();
                for (i, c) in word.iter().enumerate() {
                    // Its left edge, seen driving along u.
                    let left = total / 2.0 - i as f64 * (lw + sp);
                    for bar in letter(*c) {
                        let pts: Vec<(f64, f64)> =
                            bar.iter().map(|&(x, y)| (x, left - y)).collect();
                        let mut poly = placed(p[0], u, &pts);
                        if signed_area(&poly) < 0.0 {
                            poly.reverse();
                        }
                        out.push(poly);
                    }
                }
                out
            }
            GiveWay => {
                // A triangle 3,6 m long and 1,2 m wide pointing to the
                // driver: its outline, 0,2 m thick.
                let u = (p[1] - p[0]).normalized().unwrap_or(Vec2::new(1.0, 0.0));
                let tri = [(3.6, -0.6), (3.6, 0.6), (0.0, 0.0)];
                let pts = placed(p[0], u, &tri);
                let mut closed = pts.clone();
                closed.push(pts[0]);
                closed.push(pts[1]);
                (0..3)
                    .map(|i| strip(&closed[i..i + 2], -0.2, 0.0))
                    .collect()
            }
            Chevrons | DisabledBay | MotoBay => {
                let mut poly = p.clone();
                if signed_area(&poly) < 0.0 {
                    poly.reverse();
                }
                let mut closed = poly.clone();
                closed.push(poly[0]);
                let mut out: Vec<Vec<Point2>> =
                    closed.windows(2).map(|s| strip(s, -w, 0.0)).collect();
                match self.kind {
                    Chevrons => {
                        // Bars at 45°, `dash` wide every `gap`.
                        let (lo, hi) = poly.iter().fold(
                            (
                                Point2::new(f64::MAX, f64::MAX),
                                Point2::new(f64::MIN, f64::MIN),
                            ),
                            |(lo, hi), q| {
                                (
                                    Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                                    Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                                )
                            },
                        );
                        let dir = Vec2::new(1.0, 1.0)
                            .normalized()
                            .unwrap_or(Vec2::new(1.0, 0.0));
                        let n = dir.perp_left();
                        let span = (hi - lo).length();
                        let c = lo + (hi - lo) * 0.5;
                        let step = self.dash + self.gap.max(0.2);
                        let mut k = -span;
                        while k <= span {
                            let a = c + n * k - dir * span;
                            let b = c + n * k + dir * span;
                            for (t0, t1) in inside_spans(&poly, a, b) {
                                let (q0, q1) = (a + (b - a) * t0, a + (b - a) * t1);
                                let seg = [q0, q1];
                                let bar = strip(&seg, -self.dash / 2.0, self.dash / 2.0);
                                out.push(bar);
                            }
                            k += step;
                        }
                    }
                    MotoBay => {
                        // Divisions every metre along its longest side.
                        let (i, _) = (0..poly.len())
                            .map(|i| (i, poly[i].distance(poly[(i + 1) % poly.len()])))
                            .fold((0, 0.0), |b, x| if x.1 > b.1 { x } else { b });
                        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                        let l = a.distance(b);
                        if let Some(u) = (b - a).normalized() {
                            let n = u.perp_left();
                            let mut t = 1.0;
                            while t < l - 0.5 {
                                let q = a + u * t;
                                let far = q + n * 10.0;
                                for (t0, t1) in inside_spans(&poly, q - n * 0.01, far) {
                                    let q0 = (q - n * 0.01) + (far - (q - n * 0.01)) * t0;
                                    let q1 = (q - n * 0.01) + (far - (q - n * 0.01)) * t1;
                                    out.push(strip(&[q0, q1], -w / 2.0, w / 2.0));
                                }
                                t += 1.0;
                            }
                        }
                    }
                    _ => {}
                }
                out.retain(|x| x.len() >= 3);
                out
            }
        }
    }

    /// Where its colour fills all of it (a disabled bay is painted blue
    /// inside its white outline): its outline.
    pub fn fill(&self) -> Option<Vec<Point2>> {
        (self.kind == MarkingKind::DisabledBay && self.path.len() >= 3).then(|| self.path.clone())
    }

    /// A raised crossing's plateau and ramps: (plateau, ramp on each side).
    pub fn raised_plateau(&self) -> Option<(Vec<Point2>, [Vec<Point2>; 2])> {
        if self.kind != MarkingKind::RaisedCrossing || self.path.len() < 2 {
            return None;
        }
        let (a, b) = (self.path[0], self.path[self.path.len() - 1]);
        let v = (b - a).normalized()?;
        let along = v.perp_left() * -1.0;
        let d = self.depth.max(0.5) / 2.0 + 0.3;
        let r = RAMP;
        let plateau = vec![a - along * d, b - along * d, b + along * d, a + along * d];
        let ramp0 = vec![
            a - along * (d + r),
            b - along * (d + r),
            b - along * d,
            a - along * d,
        ];
        let ramp1 = vec![
            a + along * d,
            b + along * d,
            b + along * (d + r),
            a + along * (d + r),
        ];
        Some((plateau, [ramp0, ramp1]))
    }

    /// Its numbers as a text, for IFC: `width dash gap depth`.
    pub fn encode(&self) -> String {
        format!("{} {} {} {}", self.width, self.dash, self.gap, self.depth)
    }

    fn spec(&self) -> NewMarking {
        NewMarking {
            level: self.level,
            kind: self.kind,
            path: self.path.clone(),
            width: self.width,
            dash: self.dash,
            gap: self.gap,
            depth: self.depth,
            material: self.material,
        }
    }
}

/// The length of a raised crossing's ramps.
pub const RAMP: f64 = 1.5;
/// The height of a raised crossing over the carriageway.
pub const PLATEAU: f64 = 0.12;

impl Road {
    /// Where the segment `a → b` crosses its axis: the distance along it.
    pub fn station_across(&self, a: Point2, b: Point2) -> Option<f64> {
        let mut s = 0.0;
        for w in self.axis.windows(2) {
            let (p, q) = (w[0], w[1]);
            let (r, d) = (q - p, b - a);
            let den = r.cross(d);
            if den.abs() > 1e-12 {
                let t = (a - p).cross(d) / den;
                let u = (a - p).cross(r) / den;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    return Some(s + t * p.distance(q));
                }
            }
            s += p.distance(q);
        }
        None
    }
}

impl Model {
    fn validate_marking(&self, m: &NewMarking) -> Result<(), ModelError> {
        if self.level(m.level).is_none() {
            return Err(ModelError::UnknownLevel(m.level));
        }
        if self.material(m.material).is_none() {
            return Err(ModelError::UnknownMaterial(m.material));
        }
        let need = match m.kind.place() {
            MarkingPlace::Area => 3,
            _ => 2,
        };
        let ok = m.path.len() >= need
            && m.path.iter().all(|p| p.x.is_finite() && p.y.is_finite())
            && len_of(&m.path) > 0.05
            && [m.width, m.dash, m.gap, m.depth]
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0)
            && m.width > 0.0;
        if !ok {
            return Err(ModelError::InvalidShape(
                "a marking needs a path and a width",
            ));
        }
        Ok(())
    }

    pub fn add_marking(&mut self, spec: NewMarking) -> Result<MarkingId, ModelError> {
        self.validate_marking(&spec)?;
        let id = MarkingId(self.take_id());
        self.markings.insert(
            id,
            Marking {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                kind: spec.kind,
                path: spec.path,
                width: spec.width,
                dash: spec.dash,
                gap: spec.gap,
                depth: spec.depth,
                material: spec.material,
            },
        );
        Ok(id)
    }

    pub fn edit_marking(
        &mut self,
        id: MarkingId,
        edit: impl FnOnce(&mut Marking),
    ) -> Result<(), ModelError> {
        let mut m = self
            .markings
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut m);
        self.validate_marking(&m.spec())?;
        self.markings.insert(id, m);
        Ok(())
    }

    pub fn remove_marking(&mut self, id: MarkingId) -> Result<Marking, ModelError> {
        self.markings
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn marking(&self, id: MarkingId) -> Option<&Marking> {
        self.markings.get(&id)
    }

    pub fn markings(&self) -> impl Iterator<Item = &Marking> {
        self.markings.values()
    }

    pub fn markings_on_level(&self, level: LevelId) -> impl Iterator<Item = &Marking> {
        self.markings.values().filter(move |m| m.level == level)
    }

    /// The paint of a colour: a project material «Pintura vial …» of that
    /// colour, made if missing.
    pub fn marking_material(&mut self, rgb: [u8; 3]) -> MaterialId {
        let name = match rgb {
            [236, 196, 40] => "Pintura vial amarilla",
            [40, 96, 176] => "Pintura vial azul",
            _ => "Pintura vial blanca",
        };
        if let Some(m) = self.materials().find(|m| m.name == name) {
            return m.id;
        }
        self.add_material(name, "paint", rgb, CutPattern::Solid)
    }

    /// The top of the pavement at `p` on a level, over the level: the
    /// roundabout or the road it is on (the nearest axis), or 0.
    pub fn pavement_top_at(&self, level: LevelId, p: Point2) -> f64 {
        if let Some(c) = self
            .roundabouts_on_level(level)
            .find(|c| c.centre.distance(p) <= c.radius)
        {
            return c.top_offset;
        }
        self.roads_on_level(level)
            .map(|r| {
                let s = r.station_of(p);
                (r.axis_at(s).0.distance(p), r)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(0.0, |(_, r)| r.top_under(p))
    }

    pub(super) fn restore_markings(
        &mut self,
        list: Vec<Marking>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for m in list {
            claim(m.id.0, Some(&m.guid))
                .and_then(|_| self.validate_marking(&m.spec()))
                .map_err(|error| RestoreError {
                    element: format!("marking {}", m.id.0),
                    error,
                })?;
            self.markings.insert(m.id, m);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(kind: MarkingKind, path: Vec<Point2>) -> Marking {
        let s = NewMarking::usual(LevelId(0), kind, path, MaterialId(0));
        Marking {
            id: MarkingId(1),
            guid: GlobalId::new(),
            level: s.level,
            kind: s.kind,
            path: s.path,
            width: s.width,
            dash: s.dash,
            gap: s.gap,
            depth: s.depth,
            material: s.material,
        }
    }

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn every_kind_paints_something() {
        for k in MarkingKind::ALL {
            let path = match k.place() {
                MarkingPlace::Area => vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 2.5), p(0.0, 2.5)],
                MarkingPlace::Across => vec![p(0.0, -3.0), p(0.0, 3.0)],
                _ => vec![p(0.0, 0.0), p(20.0, 0.0)],
            };
            let m = mark(k, path);
            let shapes = m.shapes();
            assert!(!shapes.is_empty(), "{}", k.name());
            for s in &shapes {
                assert!(
                    s.len() >= 3 && signed_area(s).abs() > 1e-4,
                    "{}: {s:?}",
                    k.name()
                );
            }
            assert_eq!(MarkingKind::from_code(k.code()), Some(k));
        }
    }

    #[test]
    fn a_zebra_has_bars_every_metre() {
        // Across a road 6 m wide, 4 m deep: six bars of 0,5 m.
        let m = mark(MarkingKind::Zebra, vec![p(0.0, -3.0), p(0.0, 3.0)]);
        let bars = m.shapes();
        assert_eq!(bars.len(), 6, "{bars:?}");
        let area: f64 = bars.iter().map(|b| signed_area(b).abs()).sum();
        assert!((area - 6.0 * 0.5 * 4.0).abs() < 1e-6, "{area}");
        // A dashed line of 20 m: dashes of 2 m every 7,5 m.
        let d = mark(MarkingKind::Dashed, vec![p(0.0, 0.0), p(20.0, 0.0)]);
        assert_eq!(d.shapes().len(), 3);
        // A raised crossing: a plateau and its two ramps.
        let r = mark(MarkingKind::RaisedCrossing, vec![p(0.0, -3.0), p(0.0, 3.0)]);
        let (plateau, ramps) = r.raised_plateau().unwrap();
        assert!((signed_area(&plateau).abs() - 6.0 * 4.6).abs() < 1e-6);
        assert!((signed_area(&ramps[0]).abs() - 6.0 * RAMP).abs() < 1e-6);
    }
}
