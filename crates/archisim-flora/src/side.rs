//! The plant seen from the side, for elevations, sections and
//! axonometries, as architects draw trees there: its crown's outline (or
//! cloud, teeth, masses of foliage, washes…, the same styles as in plan)
//! and its trunk standing from the ground, not the net of the 3D mesh's
//! facets.
//!
//! The plant is turned so that what the viewer sees lies on the plan
//! drawing's axes: across the view on `x`, up on `y`, towards the viewer
//! on `z`. Then the plan's own drawing draws it.

use crate::grow::{Clump, Frond, Plant, P3};
use crate::plan::{PlanOptions, Shape};
use crate::{Look, PlanStyle};

/// How a plant is drawn seen from the side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SideOptions {
    pub style: PlanStyle,
    pub look: Look,
    /// Opacity of the colour, from 0 to 1.
    pub opacity: f64,
    /// Metres of the plant per mm of paper (the scale: 0.1 at 1:100).
    pub m_per_mm: f64,
}

impl Plant {
    /// The plant seen looking along `toward` (level; its length does not
    /// matter): each point of the shapes is `[across, up]` in metres from
    /// the foot of the plant, `across` growing to the viewer's right.
    pub fn side(&self, toward: [f64; 2], o: &SideOptions) -> Vec<Shape> {
        let turned = self.turned(toward);
        let w = (turned.radius * 2.0).max(1e-3);
        turned.draw(
            &PlanOptions {
                style: o.style,
                look: o.look,
                opacity: o.opacity,
                crown_mm: w / o.m_per_mm.max(1e-6),
            },
            true,
        )
    }

    /// The plant turned for a side view (see the module).
    fn turned(&self, toward: [f64; 2]) -> Plant {
        let l = toward[0].hypot(toward[1]);
        let d = if l > 1e-12 {
            [toward[0] / l, toward[1] / l]
        } else {
            [0.0, 1.0]
        };
        // The viewer's right.
        let r = [d[1], -d[0]];
        let at = |p: P3| -> P3 {
            [
                p[0] * r[0] + p[1] * r[1],
                p[2],
                -(p[0] * d[0] + p[1] * d[1]),
            ]
        };
        let branches = self
            .branches
            .iter()
            .map(|b| {
                let mut b = b.clone();
                b.points = b.points.iter().map(|&p| at(p)).collect();
                b
            })
            .collect();
        let clumps: Vec<Clump> = self
            .clumps
            .iter()
            .map(|c| Clump {
                center: at(c.center),
                axis: at(c.axis),
                ..c.clone()
            })
            .collect();
        let fronds: Vec<Frond> = self
            .fronds
            .iter()
            .map(|f| Frond {
                rachis: f.rachis.iter().map(|&p| at(p)).collect(),
                side: at(f.side),
                ..f.clone()
            })
            .collect();
        // The crown's middle seen from the side: the middle of its leaves
        // (or fronds), or halfway up the plant.
        let mut sum = [0.0, 0.0];
        let mut n = 0.0;
        for c in &clumps {
            sum[0] += c.center[0];
            sum[1] += c.center[1];
            n += 1.0;
        }
        for f in &fronds {
            for p in &f.rachis {
                sum[0] += p[0];
                sum[1] += p[1];
                n += 1.0;
            }
        }
        let center = if n > 0.0 {
            [sum[0] / n, sum[1] / n]
        } else {
            [0.0, self.height / 2.0]
        };
        // How far the crown reaches from its middle.
        let mut radius: f64 = 0.0;
        for c in &clumps {
            let dd = (c.center[0] - center[0]).hypot(c.center[1] - center[1]);
            radius = radius.max(dd + c.radius * c.stretch.max(1.0));
        }
        for f in &fronds {
            for p in &f.rachis {
                radius = radius.max((p[0] - center[0]).hypot(p[1] - center[1]) + f.width);
            }
        }
        if radius <= 0.0 {
            radius = (self.height / 2.0).max(self.species.width / 2.0);
        }
        Plant {
            species: self.species,
            branches,
            clumps,
            fronds,
            center,
            height: self.height,
            radius,
        }
    }
}
