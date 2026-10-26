//! The workshop: models made with more care than the first library, to be
//! shown to Antonio piece by piece and, once a set is approved, to take
//! the place of the old ones in the library.
//!
//! What makes them better: stuffed fabric instead of rounded boxes
//! ([`soft::Soft`]: faces that swell, seats that sag, creases pulled from
//! the corners, nothing perfectly regular), the piping sewn along the
//! edges, scatter cushions with their seam and pointed corners, and turned
//! wooden legs that splay a little.

pub mod armchairs;
pub mod chairs;
pub mod hearth;
pub mod lamp;
pub mod plant;
pub mod shape;
pub mod sink;
pub mod sofas;
pub mod soft;
pub mod storage;
pub mod tables;
pub mod textiles;

use crate::build::{Builder, P3};
use crate::Params;
use soft::{knife_pillow, Soft};

/// The kinds of the workshop and the names of their models.
pub const KINDS: &[(&str, &[&str])] = &[
    (
        "fine_sofa",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_armchair",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_pouf",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_chair",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_stool",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_dining_table",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_coffee_table",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_side_table",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_sideboard",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_bookcase",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_tv_unit",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_vitrine",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_rug",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_curtain",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    (
        "fine_fireplace",
        &[
            "Nórdico cálido",
            "Mediterráneo",
            "Años 50",
            "Japandi",
            "Contemporáneo",
            "Rústica",
            "Industrial",
            "Clásica",
        ],
    ),
    ("fine_plant", &["Espatifilo", "Pilea"]),
    ("fine_sink_unit", &["Fregadero de acero"]),
    ("fine_streetlight", &["Columna de fundición"]),
];

/// Makes a workshop model, if `kind` is one.
pub fn make(kind: &str, b: &mut Builder, p: &Params) -> bool {
    match kind {
        "fine_sofa" => match p.style() {
            2 => sofas::mediterranean(b, p),
            3 => sofas::fifties(b, p),
            4 => sofas::japandi(b, p),
            5 => sofas::contemporary(b, p),
            6 => sofas::rustic(b, p),
            7 => sofas::industrial(b, p),
            8 => sofas::classic(b, p),
            _ => sofa(b, p),
        },
        "fine_armchair" => {
            // The collections whose armchair is their sofa at one place.
            let mut one = p.clone();
            one.set("places", 1.0);
            for (k, v) in [("width", 0.95), ("depth", 0.88), ("pillows", 1.0)] {
                if p.get(k, f64::NAN).is_nan() {
                    one.set(k, v);
                }
            }
            match p.style() {
                2 => armchairs::mediterranean(b, p),
                3 => armchairs::fifties(b, p),
                4 => sofas::japandi(b, &one),
                5 => sofas::contemporary(b, &one),
                6 => armchairs::friar(b, p),
                7 => sofas::industrial(b, &one),
                8 => armchairs::wingback(b, p),
                _ => armchairs::nordic(b, p),
            }
        }
        "fine_pouf" => armchairs::pouf(b, p),
        "fine_chair" => chairs::chair(b, p),
        "fine_dining_table" => tables::dining(b, p),
        "fine_coffee_table" => tables::coffee(b, p),
        "fine_side_table" => tables::side(b, p),
        "fine_stool" => chairs::stool(b, p),
        "fine_sideboard" => storage::sideboard(b, p),
        "fine_bookcase" => storage::bookcase(b, p),
        "fine_tv_unit" => storage::tv_unit(b, p),
        "fine_vitrine" => storage::vitrine(b, p),
        "fine_rug" => textiles::rug(b, p),
        "fine_curtain" => textiles::curtain(b, p),
        "fine_fireplace" => hearth::fireplace(b, p),
        "fine_plant" => plant::potted_plant(b, p),
        "fine_sink_unit" => sink::sink_unit(b, p),
        "fine_streetlight" => lamp::streetlight(b, p),
        _ => return false,
    }
    true
}

/// A turned wooden leg from the floor to `top`, standing at `at` (its top)
/// and splayed `splay` metres outwards along `dir` at its foot.
fn leg(b: &mut Builder, role: &'static str, at: [f64; 2], top: f64, dir: [f64; 2], splay: f64) {
    let start = b.mark();
    let profile = [
        (0.0115, 0.0),
        (0.0135, 0.003),
        (0.0145, 0.012),
        (0.0205, top - 0.012),
        (0.0215, top - 0.004),
        (0.019, top),
    ];
    b.lathe(role, [0.0, 0.0, 0.0], &profile, 1.0, 1.0);
    // Lean it: the foot goes out, the top stays.
    let k = splay / top.max(1e-3);
    b.transform_from(start, |q| {
        let down = top - q[2];
        [
            at[0] + q[0] + dir[0] * k * down,
            at[1] + q[1] + dir[1] * k * down,
            q[2],
        ]
    });
}

/// A sofa of plump box cushions, sewn with piping, on turned legs.
///
/// Front towards `+y`, the back along `y = 0`, centred on `x = 0`, as the
/// library's sofa. Values: `width`, `depth`, `seat` (height of the seat),
/// `height` (of the back), `places`, `pillows` (scatter cushions, 0 to 4).
pub fn sofa(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.2).max(0.8);
    let d = p.get("depth", 0.92).max(0.6);
    let seat = p.get("seat", 0.43).max(0.3);
    let h = p.get("height", 0.82).max(seat + 0.15);
    let places = p.get("places", 3.0).round().clamp(1.0, 6.0) as usize;
    let x0 = -w / 2.0;
    let arm = (w * 0.085).clamp(0.15, 0.22);
    let foot = 0.13;
    let cushion = (seat - foot - 0.1).clamp(0.1, 0.16);
    let base_top = seat - cushion;
    let arm_h = (seat + 0.2).min(h - 0.04);
    let back = 0.2;

    // Legs: four turned legs splayed out a little, and a hidden pair
    // under the middle of a long sofa.
    let inset = 0.075;
    for (x, y, dx, dy) in [
        (x0 + inset, inset, -1.0f64, -1.0f64),
        (-x0 - inset, inset, 1.0, -1.0),
        (x0 + inset, d - inset, -1.0, 1.0),
        (-x0 - inset, d - inset, 1.0, 1.0),
    ] {
        let l = (dx * dx + dy * dy).sqrt();
        leg(b, "frame", [x, y], foot, [dx / l, dy / l], 0.018);
    }
    if w > 2.3 {
        for y in [inset, d - inset] {
            b.cyl("frame", [0.0, y, 0.0], [0.0, y, foot], 0.018);
        }
    }

    // The frame: the base under the seat, the back and the two arms, all
    // upholstered tight (little swell).
    let mut base = Soft::new(
        [x0 + arm - 0.02, 0.05, foot],
        [-x0 - arm + 0.02, d, base_top],
        0.03,
    );
    base.side = 0.006;
    base.crown = 0.003;
    base.wobble = 0.001;
    base.seed = 1;
    base.step = 0.25;
    base.add(b, "mat");

    let mut frame = Soft::new(
        [x0 + arm - 0.02, 0.0, foot],
        [-x0 - arm + 0.02, back, h - 0.1],
        0.05,
    );
    frame.crown = 0.01;
    frame.side = 0.008;
    frame.wobble = 0.0012;
    frame.seed = 2;
    frame.step = 0.2;
    frame.add(b, "mat");

    for (i, (a0, a1)) in [(x0, x0 + arm), (-x0 - arm, -x0)].into_iter().enumerate() {
        let mut a = Soft::new([a0, 0.0, foot], [a1, d, arm_h], (arm * 0.42).min(0.075));
        a.crown = 0.012;
        a.side = 0.014;
        a.creases = 0.004;
        a.wobble = 0.0015;
        a.seed = 3 + i as u32;
        a.add(b, "mat");
    }

    // Seat cushions, one per place, a little over the front of the base,
    // and the back cushions leaning on the back.
    let inner = w - 2.0 * arm;
    let pw = inner / places as f64;
    let gap = 0.004;
    for i in 0..places {
        let xa = x0 + arm + i as f64 * pw + gap;
        let xc = xa + pw - 2.0 * gap;
        let mut s = Soft::new([xa, back - 0.02, base_top], [xc, d + 0.015, seat], 0.045);
        s.crown = 0.024;
        s.side = 0.013;
        s.sag = 0.03;
        s.creases = 0.008;
        s.wobble = 0.0022;
        s.welt = 0.0065;
        s.seed = 10 + i as u32;
        s.add(b, "cushion");

        let start = b.mark();
        let (z0, z1) = (seat - 0.03, h + 0.03);
        let mut k = Soft::new([xa, back - 0.01, z0], [xc, back + 0.22, z1], 0.07);
        k.crown = 0.055;
        k.side = 0.016;
        k.creases = 0.01;
        k.wobble = 0.003;
        k.welt = 0.0065;
        k.seed = 20 + i as u32;
        k.add(b, "cushion");
        // Leant back about its foot against the back.
        let tilt = 13f64.to_radians();
        let (st, ct) = tilt.sin_cos();
        b.transform_from(start, |q| {
            let (dy, dz) = (q[1] - back, q[2] - z0);
            [q[0], back + dy * ct - dz * st, z0 + dy * st + dz * ct]
        });
    }

    // Scatter cushions, at the ends first, leaning on the back cushions
    // and turned a little each, as left after sitting.
    let pillows = p.get("pillows", 2.0).round().clamp(0.0, 4.0) as usize;
    let size = (pw * 0.75).clamp(0.38, 0.48);
    let at_end = x0 + arm + size * 0.62;
    let spots: Vec<f64> = match pillows {
        0 => vec![],
        1 => vec![at_end],
        2 => vec![at_end, -at_end],
        3 => vec![at_end, at_end + size * 0.85, -at_end],
        _ => vec![at_end, at_end + size * 0.85, -at_end - size * 0.85, -at_end],
    };
    for (i, x) in spots.into_iter().enumerate() {
        let yaw: f64 = [-0.16, 0.08, -0.06, 0.15][i];
        let lean = (62f64 + 4.0 * i as f64).to_radians();
        let s = if i == 1 || i == 2 { size * 0.9 } else { size };
        // Its face looks forward and up, turned by `yaw` about the vertical.
        let (sy, cy) = yaw.sin_cos();
        let (sl, cl) = lean.sin_cos();
        let az: P3 = [-sy * sl, cy * sl, cl];
        let ax: P3 = [cy, sy, 0.0];
        let c: P3 = [x, back + 0.3, seat + s * 0.46];
        knife_pillow(b, "pillow", c, ax, az, s, s, 0.2, 40 + i as u32);
    }
}
