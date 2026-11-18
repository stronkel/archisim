//! `archisim trafico`: the traffic for illustration (N9,
//! `docs/TERRITORIO.md` §2.12) on the streets of `archisim nudos`, with
//! traffic lights on its main crossing: half a minute of cars along the
//! lanes and through the junctions by their lane connections, stopping
//! at the lights and the stop signs, and people on the sidewalks. It
//! writes the plan with the cars and people where they are, a picture
//! with them and what it counted.

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{ArmSign, Model, NodeKind, NodeSettings};
use archisim_geom::crossroads::{self, Junction};
use archisim_geom::traffic::{usual_counts, Traffic, CAR};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// How long the traffic runs before the picture, seconds.
pub const SECONDS: f64 = 30.0;

/// The streets of `archisim nudos` with lights on every arm of their
/// first crossing.
pub fn build() -> Res<Model> {
    let mut m = crate::nudos::build()?;
    let pb = m.levels()[0].id;
    let j = crossroads::junctions(&m, pb)
        .into_iter()
        .find(|j| j.kind == NodeKind::Crossing && j.arms.len() == 4 && j.key.is_some())
        .ok_or("sin un cruce de cuatro calles")?;
    let key = j.key.clone().unwrap();
    let mut n = m
        .node(&key)
        .cloned()
        .unwrap_or_else(|| NodeSettings::new(key));
    for a in &j.arms {
        let arm = Junction::arm_ref(a);
        let mut s = n.arm(arm);
        s.sign = Some(ArmSign::Lights);
        n.arms.retain(|(x, _)| *x != arm);
        n.arms.push((arm, s));
    }
    m.set_node(n)?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    let pb = m.levels()[0].id;
    let (cars, people) = usual_counts(&m, pb);
    let mut t = Traffic::new(&m, pb, cars, people, 1);
    let mut most_waiting = 0;
    let steps = (SECONDS / 0.1) as usize;
    for _ in 0..steps {
        t.step(&m, 0.1);
        most_waiting = most_waiting.max(t.waiting());
    }
    std::fs::create_dir_all(dir)?;
    // The plan, with the cars and people over it.
    let options = archisim_plan::PlanOptions {
        scale: 500.0,
        ..Default::default()
    };
    let mut svg = archisim_plan::render_plan(&m, pb, &options);
    let mut over = String::from("<g id=\"traffic\">\n");
    for b in t.bodies(&m) {
        let (l, w) = if b.person { (0.5, 0.5) } else { (CAR.0, CAR.1) };
        let u = b.dir * (l / 2.0);
        let n = b.dir.perp_left() * (w / 2.0);
        let c = archisim_core::Point2::new(b.at[0], b.at[1]);
        let pts = [c - u - n, c + u - n, c + u + n, c - u + n];
        let d: Vec<String> = pts
            .iter()
            .map(|p| format!("{:.3},{:.3}", p.x, -p.y))
            .collect();
        let [r, g, bl] = b.colour;
        let _ = writeln!(
            over,
            r#"<polygon points="{}" fill="rgb({r},{g},{bl})" stroke="black" stroke-width="0.08"/>"#,
            d.join(" ")
        );
    }
    // The lit lamp of each traffic light.
    for l in t.lamps() {
        if l.colour.iter().map(|c| *c as u32).sum::<u32>() < 300 {
            continue;
        }
        let [r, g, bl] = l.colour;
        let _ = writeln!(
            over,
            r#"<circle cx="{:.3}" cy="{:.3}" r="0.6" fill="rgb({r},{g},{bl})" stroke="black" stroke-width="0.08"/>"#,
            l.at[0], -l.at[1]
        );
    }
    over.push_str("</g>\n");
    if let Some(end) = svg.rfind("</svg>") {
        svg.insert_str(end, &over);
    }
    std::fs::write(dir.join("trafico-planta.svg"), &svg)?;
    // A picture of the crossing with the lights, from the south-west.
    let mut view = archisim_render::View::fit(&m);
    if let Some(j) = crossroads::junctions(&m, pb)
        .into_iter()
        .find(|j| j.kind == NodeKind::Crossing && j.arms.len() == 4 && j.key.is_some())
    {
        view.target = [j.at.x, j.at.y, 0.0];
        view.eye = [j.at.x - 32.0, j.at.y - 42.0, 26.0];
        view.perspective = true;
    }
    let picture = archisim_render::render(
        &m,
        &view,
        &archisim_render::Options {
            width: 800,
            height: 500,
            samples: 1,
            sky_rays: 4,
            traffic: Some(SECONDS),
            ..Default::default()
        },
        &|_| {},
    );
    std::fs::write(
        dir.join("trafico.png"),
        archisim_export::png::encode_rgb(
            picture.width as u32,
            picture.height as u32,
            &picture.rgb,
            96.0,
        ),
    )?;
    println!(
        "Tráfico: {} coches y {} personas durante {} s; como mucho {} parados a la vez en semáforos y stop.",
        t.cars(),
        people,
        SECONDS,
        most_waiting
    );
    println!("  Planta: {}", dir.join("trafico-planta.svg").display());
    println!("  Imagen: {}", dir.join("trafico.png").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crossing has its lights and the cars stop at them.
    #[test]
    fn cars_stop_at_the_lights_of_the_crossing() {
        let m = build().unwrap();
        let pb = m.levels()[0].id;
        let (cars, people) = usual_counts(&m, pb);
        let mut t = Traffic::new(&m, pb, cars, people, 1);
        let mut most = 0;
        for _ in 0..400 {
            t.step(&m, 0.1);
            most = most.max(t.waiting());
        }
        assert!(most > 0);
        assert!(t.cars() >= cars * 3 / 4);
    }
}
