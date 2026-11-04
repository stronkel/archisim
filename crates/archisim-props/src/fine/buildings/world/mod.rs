//! Buildings of the world (taller de edificios, tanda 6): three typologies
//! — the house between party walls of a street, the block of flats and the
//! detached house — dressed by the skin of a place. A skin ([`Skin`]) says
//! what a place builds with and how: its colours and stones, its windows
//! and what closes them, its balconies, how its ground floor meets the
//! street, how its walls end and what its roof is. The same typology in
//! Seville, Paris, Amsterdam, Brooklyn or Kyoto is the same code with
//! another skin, so a building brought from a map can take the skin of
//! where it stands (tanda 7) and still be changed by hand.
//!
//! Every material is a role of its own name ([`MATERIALS`]): a skin picks
//! from them by role, and the recipe of each typology lists them all.

pub mod facade;
pub mod plan;
pub mod roofs;
pub mod skins;
pub mod types;

pub use skins::{skin_of, Skin, SKINS};

use crate::build::Builder;

/// The materials of the world's buildings: role and material. The skins
/// choose among these roles; the recipes give each its material.
pub const MATERIALS: &[(&str, &str)] = &[
    // Renders and paints.
    ("w_white", "Cal blanca"),
    ("w_cream", "Monocapa crema"),
    ("w_yellow", "Estuco amarillo"),
    ("w_ochre", "Estuco ocre"),
    ("w_almagra", "Estuco almagra"),
    ("w_pink", "Estuco rosa"),
    ("w_salmon", "Estuco salmón"),
    ("w_blue", "Estuco azul"),
    ("w_green", "Estuco verde agua"),
    ("w_grey", "Enfoscado gris"),
    ("w_pearl", "Estuco gris perla"),
    ("w_terracotta", "Estuco terracota"),
    // Stones.
    ("s_lime", "Piedra caliza"),
    ("s_ashlar", "Sillería arenisca"),
    ("s_rubble", "Mampostería"),
    ("s_brown", "Piedra parda"),
    ("s_granite", "Piedra gris"),
    // Bricks.
    ("k_red", "Ladrillo visto"),
    ("k_dark", "Ladrillo pardo"),
    ("k_yellow", "Ladrillo de Londres"),
    ("k_grey", "Ladrillo gris"),
    // Boards.
    ("b_falu", "Madera roja de Falun"),
    ("b_white", "Tablas blancas"),
    ("b_blue", "Tablas solapadas azul"),
    ("b_green", "Tablas solapadas verde"),
    ("b_yellow", "Tablas amarillas"),
    ("b_grey", "Tablas grises"),
    ("b_dark", "Madera oscura"),
    ("b_cedar", "Madera de cedro"),
    // Concrete, panels and tiles of the walls.
    ("c_concrete", "Hormigón suave"),
    ("c_panel", "Panel prefabricado"),
    ("c_tile_white", "Cerámica blanca"),
    ("c_azulejo", "Azulejo azul"),
    // Roofs.
    ("r_curved", "Teja curva"),
    ("r_tile", "Teja plana roja"),
    ("r_slate", "Tejuela de pizarra"),
    ("r_zinc", "Zinc"),
    ("r_copper", "Cobre con pátina"),
    ("r_black", "Teja negra"),
    ("r_grey", "Teja gris"),
    ("r_shingle", "Tejuela asfáltica"),
    ("r_red_metal", "Chapa roja"),
    ("r_galv", "Acero galvanizado"),
    ("r_green_tile", "Teja vidriada verde"),
    ("r_gravel", "Grava"),
    // Carpentry and paints of the woodwork.
    ("f_white", "Lacado blanco"),
    ("f_green", "Lacado verde carruaje"),
    ("f_blue", "Lacado azul añil"),
    ("f_sky", "Lacado azul celeste"),
    ("f_greyblue", "Lacado gris azulado"),
    ("f_red", "Lacado rojo"),
    ("f_yellow", "Lacado amarillo"),
    ("f_black", "Metal negro"),
    ("f_brown", "Nogal"),
    ("f_oak", "Roble claro"),
    ("f_alu", "Aluminio plata"),
    ("f_bronze", "Aluminio bronce"),
    ("iron", "Hierro forjado"),
    // What every building has.
    ("glass", "Vidrio claro"),
    ("interior", "Interior en sombra"),
    ("curtain", "Visillo"),
    ("blind", "Persiana crema"),
    ("metal", "Acero galvanizado"),
    ("appliance", "Lacado blanco"),
    ("pot", "Terracota"),
    ("leaves", "Hoja verde"),
    ("flowers", "Buganvilla"),
    ("trunk", "Corteza de olivo"),
    ("palm", "Corteza de pino"),
    ("fruit", "Naranja"),
    ("lamp", "Luz de farola"),
    ("brass", "Latón"),
    ("sign", "Rótulo granate"),
    ("sign_b", "Plástico amarillo"),
    ("shutter", "Acero galvanizado"),
    ("awning", "Lona verde"),
    ("awning_b", "Lona crema"),
    ("line", "Línea blanca"),
    ("cloth_a", "Toalla blanca"),
    ("cloth_b", "Lino añil"),
    ("cloth_c", "Lana teja"),
    ("paving", "Losa de acera"),
    ("lawn", "Césped natural"),
    ("hedge", "Hoja verde tierna"),
    ("car_a", "Pintura roja"),
    ("car_b", "Lacado blanco"),
    ("car_c", "Granito negro"),
    ("car_glass", "Vidrio opaco gris"),
    ("tyre", "Goma negra"),
];

/// The material of a role, if it is one of the world's.
pub fn material_of(role: &str) -> Option<&'static str> {
    MATERIALS.iter().find(|m| m.0 == role).map(|m| m.1)
}

/// The roles a building of the world is made of, as the skin chose them
/// for one building (its seed).
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub wall: &'static str,
    pub base: &'static str,
    pub trim: &'static str,
    pub frame: &'static str,
    pub shutter: &'static str,
    pub door: &'static str,
    pub roof: &'static str,
    pub accent: &'static str,
}

fn pick(list: &[&'static str], seed: u64, k: u64, or: &'static str) -> &'static str {
    if list.is_empty() {
        return or;
    }
    list[(super::unit(seed, k) * list.len() as f64) as usize % list.len()]
}

/// The look of one building of `skin` (`house` takes the house's walls if
/// the skin has its own).
pub fn look(skin: &Skin, seed: u64, house: bool) -> Look {
    let walls = if house && !skin.house.walls.is_empty() {
        skin.house.walls
    } else {
        skin.walls
    };
    let wall = pick(walls, seed, 1, "w_white");
    let trim = pick(skin.trims, seed, 3, "w_white");
    let trim = if trim == wall {
        // A trim the colour of the wall would not show: the next one.
        skin.trims
            .iter()
            .copied()
            .find(|t| *t != wall)
            .unwrap_or(trim)
    } else {
        trim
    };
    Look {
        wall,
        base: pick(skin.base, seed, 2, wall),
        trim,
        frame: pick(skin.frames, seed, 4, "f_white"),
        shutter: pick(skin.shutters, seed, 5, "f_green"),
        door: pick(skin.doors, seed, 6, "f_brown"),
        roof: skin.roof_mat,
        accent: pick(skin.accents, seed, 7, trim),
    }
}

/// Gives the triangles of role `from` to role `to` (the pieces shared
/// with the other buildings draw in fixed roles: «tiles», «wall»…).
pub fn recolour(b: &mut Builder, from: &'static str, to: &'static str) {
    if from == to {
        return;
    }
    if let Some(tris) = b.parts.remove(from) {
        b.parts.entry(to).or_default().extend(tris);
        if b.fine.remove(from) {
            b.fine.insert(to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, make, Params};

    const KINDS: [&str; 3] = [
        "fine_world_townhouse",
        "fine_world_block",
        "fine_world_house",
    ];

    /// Every typology in every skin is made, with a sensible size, its
    /// roles among the world's materials and nothing in the air.
    #[test]
    fn every_skin_is_made() {
        let mut bad = Vec::new();
        for (i, sk) in SKINS.iter().enumerate() {
            for kind in KINDS {
                for variant in [1.0, 2.0] {
                    let mut p = Params::default();
                    p.set("skin", i as f64);
                    p.set("variant", variant);
                    let m = make(kind, &p).unwrap();
                    let n = m.triangle_count();
                    if !(1_000..=250_000).contains(&n) {
                        bad.push(format!("{} {kind}: {n} triangles", sk.key));
                    }
                    for (role, _) in &m.parts {
                        if material_of(role).is_none() {
                            bad.push(format!("{} {kind}: role «{role}» has no material", sk.key));
                        }
                    }
                    let loose = check::loose_pieces(&m, check::anchors_for(kind), 0.004);
                    if !loose.is_empty() {
                        let l = &loose[0];
                        bad.push(format!(
                            "{} {kind} v{variant}: {} loose, {} {:.3} off at x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
                            sk.key,
                            loose.len(),
                            l.role,
                            l.gap,
                            l.lo[0],
                            l.hi[0],
                            l.lo[1],
                            l.hi[1],
                            l.lo[2],
                            l.hi[2]
                        ));
                    }
                }
            }
        }
        bad.dedup();
        for b in &bad {
            println!("{b}");
        }
        assert!(bad.is_empty(), "{} problems", bad.len());
    }

    /// Buildings over plans that are not rectangles (an L, a corner with
    /// a chamfer, a slanted plot), in every skin and typology, full and
    /// light: made, with their materials, nothing in the air, and the
    /// light ones much lighter.
    #[test]
    fn plans_of_any_shape() {
        use plan::{Plan, Side};
        use types::{dress, Typology};
        let plans = [
            Plan {
                poly: vec![
                    [0.0, 0.0],
                    [16.0, 0.0],
                    [16.0, 7.0],
                    [7.0, 7.0],
                    [7.0, 18.0],
                    [0.0, 18.0],
                ],
                sides: vec![
                    Side::Free,
                    Side::Street,
                    Side::Free,
                    Side::Street,
                    Side::Free,
                    Side::Party,
                ],
            },
            Plan {
                poly: vec![
                    [0.0, 0.0],
                    [20.0, 0.0],
                    [20.0, 12.0],
                    [17.0, 15.0],
                    [0.0, 15.0],
                ],
                sides: vec![
                    Side::Party,
                    Side::Street,
                    Side::Street,
                    Side::Street,
                    Side::Party,
                ],
            },
            Plan {
                poly: vec![[0.0, 0.0], [12.0, 2.0], [11.0, 13.0], [-1.0, 11.0]],
                sides: vec![Side::Back, Side::Party, Side::Street, Side::Party],
            },
        ];
        let mut bad = Vec::new();
        for (k, plan) in plans.iter().enumerate() {
            for (i, sk) in SKINS.iter().enumerate() {
                for t in [Typology::Townhouse, Typology::Block, Typology::House] {
                    let full = dress(plan, t, i, 0, 3, false);
                    let light = dress(plan, t, i, 0, 3, true);
                    for m in [&full, &light] {
                        for (role, _) in &m.parts {
                            if material_of(role).is_none() {
                                bad.push(format!("plan {k} {} {t:?}: role «{role}»", sk.key));
                            }
                        }
                        let loose =
                            check::loose_pieces(m, check::anchors_for("fine_world_block"), 0.004);
                        if let Some(l) = loose.first() {
                            bad.push(format!(
                                "plan {k} {} {t:?}: {} loose, {} {:.3} off at x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
                                sk.key, loose.len(), l.role, l.gap, l.lo[0], l.hi[0], l.lo[1], l.hi[1], l.lo[2], l.hi[2]
                            ));
                        }
                    }
                    let (a, e) = (full.triangle_count(), light.triangle_count());
                    if e * 10 > a * 6 {
                        bad.push(format!("plan {k} {} {t:?}: light {e} of {a}", sk.key));
                    }
                }
            }
        }
        for b in &bad {
            println!("{b}");
        }
        assert!(bad.is_empty(), "{} problems", bad.len());
    }

    /// Every role of a skin is one of the world's materials.
    #[test]
    fn skins_use_known_materials() {
        for sk in SKINS {
            let lists = [
                sk.walls,
                sk.base,
                sk.trims,
                sk.accents,
                sk.frames,
                sk.shutters,
                sk.doors,
                sk.block.walls,
                sk.house.walls,
            ];
            for role in lists.iter().flat_map(|l| l.iter()).chain([&sk.roof_mat]) {
                assert!(material_of(role).is_some(), "{}: {role}", sk.key);
            }
            if !sk.house.roof_mat.is_empty() {
                assert!(material_of(sk.house.roof_mat).is_some(), "{}", sk.key);
            }
        }
    }
}
