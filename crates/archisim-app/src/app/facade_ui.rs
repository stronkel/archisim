//! The façade of a wall type in «Tipos y materiales» (E9): what its
//! exterior finish is built of, its bond, its pieces and joints, its
//! pattern, the lintels on end and the piece over the slab edges — every
//! choice in cards drawn as the façade would look.

use archisim_core::{
    Bond, Coursing, Facade, FacadeSystem, MaterialId, Model, Pattern, SlabFront, WallType,
};
use eframe::egui::{Color32, ColorImage, Id, Ui, Vec2};

use super::types_ui::Action;
use super::widgets::SwitchExt;
use super::{props, ArchiSimApp};

const NONE: &str = "Sin aparejo";
const FRONT_NONE: &str = "Sin pieza propia";
const FRONT_SLIPS: &str = "Plaquetas";
const FRONT_SOLDIERS: &str = "Ladrillo a sardinel";

/// A picture of a bond: a patch of façade, its pieces in their colours
/// over the colour of the joints.
pub fn bond_image(c: &Coursing, piece: [u8; 3], accent: [u8; 3], joint: [u8; 3]) -> ColorImage {
    let (w, h) = (96usize, 64usize);
    // As many pieces as read well: some four across.
    let width_m = (c.length + c.joint) * 3.6;
    let k = w as f64 / width_m;
    let mut img = ColorImage::new(
        [w, h],
        vec![Color32::from_rgb(joint[0], joint[1], joint[2]); w * h],
    );
    let ch = c.course_height();
    let courses = ((h as f64 / k) / ch).ceil() as i64 + 1;
    for course in 0..courses {
        let y1 = h as f64 - course as f64 * ch * k;
        let y0 = y1 - c.height * k;
        for u in c.course(course, width_m, false) {
            let accent_here = c
                .accent_by_place((u.start + u.end) / 2.0, course)
                .unwrap_or(u.accent);
            let rgb = if accent_here { accent } else { piece };
            // A little light on the top of each piece, to read it.
            let (x0, x1) = (u.start * k, u.end * k);
            for py in (y0.max(0.0) as usize)..(y1.min(h as f64) as usize) {
                for px in (x0.max(0.0) as usize)..(x1.ceil().min(w as f64) as usize) {
                    let shade = if (py as f64) < y0 + 1.0 { 18 } else { 0 };
                    img[(px, py)] = Color32::from_rgb(
                        rgb[0].saturating_add(shade),
                        rgb[1].saturating_add(shade),
                        rgb[2].saturating_add(shade),
                    );
                }
            }
        }
    }
    img
}

fn colour(m: &Model, id: Option<MaterialId>, or: [u8; 3]) -> [u8; 3] {
    id.filter(|id| m.material(*id).is_some())
        .map_or(or, |id| m.surface_color(id))
}

const MORTAR: [u8; 3] = [196, 190, 178];

impl ArchiSimApp {
    /// The cards of a bond: `options` drawn as the façade `make` gives
    /// for each.
    fn bond_cards(
        &mut self,
        ui: &mut Ui,
        label: &str,
        options: &[String],
        current: &str,
        colours: ([u8; 3], [u8; 3], [u8; 3]),
        make: &dyn Fn(&str) -> Option<Coursing>,
    ) -> Option<String> {
        let ctx = ui.ctx().clone();
        let swatches = &mut self.swatches;
        let mut picked = None;
        super::cards::card_row(ui, label, |ui, w| {
            let mut thumb = |o: &str| {
                let c = make(o);
                let key = format!("bond:{c:?}:{colours:?}");
                let tex = swatches.image(&ctx, &key, || match c {
                    Some(c) => bond_image(&c, colours.0, colours.1, colours.2),
                    None => ColorImage::new(
                        [96, 64],
                        vec![Color32::from_rgb(colours.0[0], colours.0[1], colours.0[2]); 96 * 64],
                    ),
                });
                Some(super::thumbs::Thumb {
                    tex,
                    size: Vec2::new(96.0, 64.0),
                })
            };
            picked = super::cards::option_cards(ui, w - 8.0, options, current, &mut thumb);
        });
        picked.filter(|p| p != current)
    }

    /// The group «Fachada» of a wall type.
    pub(super) fn facade_group(&mut self, ui: &mut Ui, t: &WallType, actions: &mut Vec<Action>) {
        props::group(
            ui,
            "types-wall-facade",
            "Fachada: aparejo y despiece",
            |ui| {
                props::note(
                ui,
                "La capa exterior se dibuja pieza a pieza en 3D, se cuenta en las mediciones y va al IFC como piezas del muro.",
            );
                self.facade_body(ui, t, actions);
            },
        );
    }

    fn facade_body(&mut self, ui: &mut Ui, t: &WallType, actions: &mut Vec<Action>) {
        let id = t.id;
        let m = self.ed.model();
        let Some(first) = t.layers.first() else {
            return;
        };
        let piece_rgb = m.surface_color(first.material);
        let now = t.facade;
        let joint_rgb = colour(m, now.and_then(|f| f.coursing.joint_material), MORTAR);
        let accent_rgb = colour(m, now.and_then(|f| f.coursing.accent), [96, 58, 46]);
        let colours = (piece_rgb, accent_rgb, joint_rgb);
        // What it is built of.
        let mut systems = vec![NONE.to_string()];
        systems.extend(FacadeSystem::ALL.iter().map(|s| s.name().to_string()));
        let current = now.map_or(NONE, |f| f.coursing.system.name());
        let picked = self.bond_cards(ui, "Sistema", &systems, current, colours, &|o| {
            FacadeSystem::ALL
                .iter()
                .find(|s| s.name() == o)
                .map(|s| s.coursing())
        });
        if let Some(o) = picked {
            let f = FacadeSystem::ALL
                .iter()
                .find(|s| s.name() == o)
                .map(|s| Facade {
                    coursing: Coursing {
                        accent: now.and_then(|f| f.coursing.accent),
                        ..s.coursing()
                    },
                    slab_front: now.and_then(|f| f.slab_front),
                });
            actions.push(Action::WallFacade(id, f));
        }
        let Some(f) = now else {
            return;
        };
        let c = f.coursing;
        // The bond.
        let bonds: Vec<String> = Bond::ALL.iter().map(|b| b.name().to_string()).collect();
        if let Some(o) = self.bond_cards(ui, "Aparejo", &bonds, c.bond.name(), colours, &|o| {
            Bond::ALL
                .iter()
                .find(|b| b.name() == o)
                .map(|b| Coursing { bond: *b, ..c })
        }) {
            if let Some(b) = Bond::ALL.iter().find(|b| b.name() == o) {
                let mut g = f;
                g.coursing.bond = *b;
                actions.push(Action::WallFacade(id, Some(g)));
            }
        }
        // The piece and the joints.
        let mut sizes =
            |ui: &mut Ui, name: &str, hint: &str, v: f64, set: fn(&mut Coursing, f64)| {
                props::row(ui, name, hint, |ui, w| {
                    if let Some(x) =
                        super::length_field(ui, Id::unique(("facade", id.0, name)), v, w - 8.0)
                    {
                        let mut g = f;
                        set(&mut g.coursing, x);
                        actions.push(Action::WallFacade(id, Some(g)));
                    }
                });
            };
        sizes(
            ui,
            "Largo (mm)",
            "La soga: lo que mide la pieza a lo largo",
            c.length,
            |c, x| c.length = x,
        );
        sizes(
            ui,
            "Alto (mm)",
            "El grueso de la hilada sin la junta",
            c.height,
            |c, x| c.height = x,
        );
        sizes(
            ui,
            "Tizón (mm)",
            "La testa: lo que se ve de una pieza puesta a tizón",
            c.depth,
            |c, x| c.depth = x,
        );
        sizes(ui, "Junta (mm)", "Llagas y tendeles", c.joint, |c, x| {
            c.joint = x
        });
        if let Some(j) = c.joint_material {
            if let Some(new) =
                self.material_param_row(ui, "facade-joint", "Junta (color)", "joint", j)
            {
                let mut g = f;
                g.coursing.joint_material = Some(MaterialId(new));
                actions.push(Action::WallFacade(id, Some(g)));
            }
        }
        // The pattern of a second material.
        let patterns = [
            Pattern::Plain,
            Pattern::Headers,
            Pattern::Courses(4),
            Pattern::Diamonds(3),
        ];
        let names: Vec<String> = patterns.iter().map(|p| p.name().to_string()).collect();
        let accent = c.accent.unwrap_or(first.material);
        if let Some(o) = self.bond_cards(ui, "Dibujo", &names, c.pattern.name(), colours, &|o| {
            patterns.iter().find(|p| p.name() == o).map(|p| Coursing {
                pattern: *p,
                accent: Some(accent),
                ..c
            })
        }) {
            if let Some(p) = patterns.iter().find(|p| p.name() == o) {
                let mut g = f;
                g.coursing.pattern = *p;
                actions.push(Action::WallFacade(id, Some(g)));
            }
        }
        if c.pattern != Pattern::Plain {
            if let Some(new) =
                self.material_param_row(ui, "facade-accent", "Segundo material", "brick", accent)
            {
                let mut g = f;
                g.coursing.accent = Some(MaterialId(new));
                actions.push(Action::WallFacade(id, Some(g)));
            }
        }
        let mut soldiers = c.soldier_heads;
        if ui
            .switch(
                &mut soldiers,
                "Dinteles a sardinel sobre puertas y ventanas",
            )
            .changed()
        {
            let mut g = f;
            g.coursing.soldier_heads = soldiers;
            actions.push(Action::WallFacade(id, Some(g)));
        }
        // The piece over the slab edges.
        let fronts = [FRONT_NONE, FRONT_SLIPS, FRONT_SOLDIERS].map(String::from);
        let front_now = match f.slab_front {
            None => FRONT_NONE,
            Some(s) if s.coursing.bond == Bond::Stack => FRONT_SOLDIERS,
            Some(_) => FRONT_SLIPS,
        };
        let joint = c.joint_material;
        if let Some(o) = self.bond_cards(
            ui,
            "Frente de forjado",
            &fronts,
            front_now,
            colours,
            &|o| match o {
                FRONT_SLIPS => Some(SlabFront::slips().coursing),
                FRONT_SOLDIERS => Some(SlabFront::soldiers().coursing),
                _ => None,
            },
        ) {
            let mut g = f;
            g.slab_front = match o.as_str() {
                FRONT_SLIPS => Some(SlabFront::slips()),
                FRONT_SOLDIERS => Some(SlabFront::soldiers()),
                _ => None,
            };
            if let Some(s) = g.slab_front.as_mut() {
                s.coursing.joint_material = joint;
                s.material = f.slab_front.and_then(|s| s.material);
            }
            actions.push(Action::WallFacade(id, Some(g)));
        }
        if let Some(s) = f.slab_front {
            let now = s.material.unwrap_or(first.material);
            if let Some(new) =
                self.material_param_row(ui, "facade-front", "Pieza del frente", "brick", now)
            {
                let mut g = f;
                if let Some(x) = g.slab_front.as_mut() {
                    x.material = Some(MaterialId(new));
                }
                actions.push(Action::WallFacade(id, Some(g)));
            }
            props::row(
                ui,
                "Vuelo (mm)",
                "Lo que sale el frente de la cara de la fachada",
                |ui, w| {
                    if let Some(x) = super::length_field(
                        ui,
                        Id::unique(("facade-front-out", id.0)),
                        s.projection,
                        w - 8.0,
                    ) {
                        let mut g = f;
                        if let Some(s) = g.slab_front.as_mut() {
                            s.projection = x.max(0.0);
                        }
                        actions.push(Action::WallFacade(id, Some(g)));
                    }
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bond's picture has its pieces and its joints.
    #[test]
    fn bonds_are_drawn() {
        let c = FacadeSystem::Brick.coursing();
        let img = bond_image(&c, [168, 86, 60], [90, 50, 40], MORTAR);
        let brick = img
            .pixels
            .iter()
            .filter(|p| p.r() == 168 || p.r() == 186)
            .count();
        let joint = img.pixels.iter().filter(|p| p.r() == MORTAR[0]).count();
        assert!(brick > joint && joint > 0, "{brick} {joint}");
        // Flemish with dark headers shows them.
        let fl = Coursing {
            bond: Bond::Flemish,
            accent: Some(MaterialId(1)),
            pattern: Pattern::Headers,
            ..c
        };
        let img = bond_image(&fl, [168, 86, 60], [90, 50, 40], MORTAR);
        assert!(img.pixels.iter().any(|p| p.r() == 90));
    }
}
