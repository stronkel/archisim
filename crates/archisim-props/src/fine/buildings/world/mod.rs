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

pub mod civic;
pub mod facade;
pub mod landmarks;
pub mod plan;
pub mod roofs;
pub mod skins;
pub mod state;
pub mod towers;
pub mod types;

pub use skins::{skin_of, Skin, SKINS};

use crate::build::Builder;

/// The materials of the world's buildings: role, material, its kind and
/// its colour. The skins choose among these roles; the recipes give each
/// its material, and a project brought from a map makes the ones it lacks
/// (`Model::ensure_world_materials`).
pub const MATERIALS: &[(&str, &str, &str, [u8; 3])] = &[
    ("g_reflect", "Vidrio reflectante", "glass", [104, 140, 166]),
    ("g_opaque", "Vidrio opaco gris", "other", [64, 74, 84]),
    ("w_faded", "Revoco envejecido", "stone", [198, 190, 172]),
    ("w_damp", "Mancha de humedad", "stone", [146, 136, 116]),
    ("water", "Agua de fuente", "water", [128, 192, 222]),
    ("court", "Pista verde", "other", [52, 112, 82]),
    ("sand", "Arena de playa", "other", [222, 204, 160]),
    ("s_mares", "Piedra de marés", "stone", [212, 178, 128]),
    ("r_thatch", "Cubierta de paja", "other", [190, 164, 104]),
    ("w_white", "Cal blanca", "stone", [238, 234, 226]),
    ("w_cream", "Monocapa crema", "render", [226, 208, 172]),
    ("w_yellow", "Estuco amarillo", "render", [239, 207, 106]),
    ("w_ochre", "Estuco ocre", "render", [204, 148, 76]),
    ("w_almagra", "Estuco almagra", "render", [226, 164, 140]),
    ("w_pink", "Estuco rosa", "render", [226, 174, 164]),
    ("w_salmon", "Estuco salmón", "render", [232, 164, 124]),
    ("w_blue", "Estuco azul", "render", [148, 180, 206]),
    ("w_green", "Estuco verde agua", "render", [160, 200, 180]),
    ("w_grey", "Enfoscado gris", "render", [168, 164, 156]),
    ("w_pearl", "Estuco gris perla", "render", [208, 204, 196]),
    ("w_terracotta", "Estuco terracota", "render", [190, 110, 76]),
    ("s_lime", "Piedra caliza", "stone", [198, 184, 156]),
    ("s_ashlar", "Sillería arenisca", "stone", [205, 191, 163]),
    ("s_rubble", "Mampostería", "stone", [180, 176, 166]),
    ("s_brown", "Piedra parda", "stone", [116, 84, 68]),
    ("s_granite", "Piedra gris", "stone", [152, 150, 144]),
    ("k_red", "Ladrillo visto", "masonry", [176, 98, 72]),
    ("k_dark", "Ladrillo pardo", "masonry", [120, 74, 58]),
    (
        "k_yellow",
        "Ladrillo de Londres",
        "masonry",
        [190, 164, 118],
    ),
    ("k_grey", "Ladrillo gris", "masonry", [128, 128, 126]),
    ("b_falu", "Madera roja de Falun", "wood", [140, 46, 38]),
    ("b_white", "Tablas blancas", "wood", [236, 234, 228]),
    ("b_blue", "Tablas solapadas azul", "wood", [125, 159, 208]),
    ("b_green", "Tablas solapadas verde", "wood", [169, 211, 181]),
    ("b_yellow", "Tablas amarillas", "wood", [228, 200, 126]),
    ("b_grey", "Tablas grises", "wood", [152, 156, 154]),
    ("b_dark", "Madera oscura", "wood", [66, 50, 40]),
    ("b_cedar", "Madera de cedro", "wood", [172, 114, 74]),
    ("c_concrete", "Hormigón suave", "concrete", [201, 196, 187]),
    ("c_panel", "Panel prefabricado", "stone", [190, 186, 178]),
    (
        "c_tile_white",
        "Cerámica blanca",
        "ceramic",
        [240, 238, 232],
    ),
    ("c_azulejo", "Azulejo azul", "stone", [106, 144, 186]),
    ("r_curved", "Teja curva", "ceramic", [188, 108, 71]),
    ("r_tile", "Teja plana roja", "stone", [166, 74, 52]),
    ("r_slate", "Tejuela de pizarra", "stone", [94, 100, 110]),
    ("r_zinc", "Zinc", "metal", [141, 151, 163]),
    ("r_copper", "Cobre con pátina", "metal", [94, 154, 128]),
    ("r_black", "Teja negra", "stone", [52, 54, 58]),
    ("r_grey", "Teja gris", "stone", [106, 110, 114]),
    ("r_shingle", "Tejuela asfáltica", "stone", [94, 88, 86]),
    ("r_red_metal", "Chapa roja", "metal", [142, 48, 42]),
    ("r_galv", "Acero galvanizado", "metal", [170, 175, 178]),
    (
        "r_green_tile",
        "Teja vidriada verde",
        "stone",
        [62, 112, 80],
    ),
    ("r_gravel", "Grava", "gravel", [201, 193, 178]),
    ("f_white", "Lacado blanco", "wood", [234, 230, 222]),
    ("f_green", "Lacado verde carruaje", "wood", [46, 84, 62]),
    ("f_blue", "Lacado azul añil", "wood", [64, 94, 140]),
    ("f_sky", "Lacado azul celeste", "wood", [150, 194, 204]),
    ("f_greyblue", "Lacado gris azulado", "wood", [120, 140, 152]),
    ("f_red", "Lacado rojo", "wood", [152, 48, 42]),
    ("f_yellow", "Lacado amarillo", "wood", [222, 180, 62]),
    ("f_black", "Metal negro", "metal", [46, 46, 48]),
    ("f_brown", "Nogal", "wood", [120, 82, 56]),
    ("f_oak", "Roble claro", "wood", [206, 170, 122]),
    ("f_alu", "Aluminio plata", "metal", [178, 178, 172]),
    ("f_bronze", "Aluminio bronce", "metal", [118, 94, 70]),
    ("iron", "Hierro forjado", "metal", [54, 52, 50]),
    ("glass", "Vidrio claro", "glass", [206, 224, 228]),
    ("interior", "Interior en sombra", "other", [58, 58, 62]),
    ("curtain", "Visillo", "glass", [246, 244, 238]),
    ("blind", "Persiana crema", "metal", [214, 204, 178]),
    ("metal", "Acero galvanizado", "metal", [170, 175, 178]),
    ("appliance", "Lacado blanco", "wood", [234, 230, 222]),
    ("pot", "Terracota", "ceramic", [196, 112, 78]),
    ("leaves", "Hoja verde", "vegetation", [76, 132, 76]),
    ("flowers", "Buganvilla", "vegetation", [200, 44, 128]),
    ("trunk", "Corteza de olivo", "wood", [112, 102, 88]),
    ("palm", "Corteza de pino", "wood", [104, 80, 58]),
    ("fruit", "Naranja", "other", [236, 140, 40]),
    ("lamp", "Luz de farola", "other", [255, 226, 150]),
    ("brass", "Latón", "metal", [196, 160, 88]),
    ("sign", "Rótulo granate", "other", [122, 31, 43]),
    ("sign_b", "Plástico amarillo", "other", [235, 190, 45]),
    ("shutter", "Acero galvanizado", "metal", [170, 175, 178]),
    ("awning", "Lona verde", "fabric", [58, 112, 76]),
    ("awning_b", "Lona crema", "fabric", [236, 226, 200]),
    ("line", "Línea blanca", "other", [244, 244, 240]),
    ("cloth_a", "Toalla blanca", "fabric", [242, 240, 234]),
    ("cloth_b", "Lino añil", "fabric", [58, 86, 140]),
    ("cloth_c", "Lana teja", "fabric", [190, 104, 72]),
    ("paving", "Losa de acera", "stone", [214, 208, 196]),
    ("lawn", "Césped natural", "vegetation", [72, 136, 64]),
    ("hedge", "Hoja verde tierna", "vegetation", [122, 166, 84]),
    ("car_a", "Pintura roja", "other", [185, 45, 40]),
    ("car_b", "Lacado blanco", "wood", [234, 230, 222]),
    ("car_c", "Granito negro", "stone", [44, 44, 46]),
    ("car_glass", "Vidrio opaco gris", "other", [64, 74, 84]),
    ("tyre", "Goma negra", "other", [38, 38, 40]),
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
                    if !(1_000..=400_000).contains(&n) {
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

    /// Every typology of every place as built today, abandoned and being
    /// built: made, with their materials and nothing in the air.
    #[test]
    fn eras_and_states() {
        let kinds = [
            "fine_world_townhouse",
            "fine_world_block",
            "fine_world_house",
            "fine_world_tower",
            "fine_world_courtyard",
            "fine_world_row",
        ];
        let mut bad = Vec::new();
        for (i, sk) in SKINS.iter().enumerate() {
            for kind in kinds {
                for (era, state) in [(1.0, 0.0), (0.0, 1.0), (0.0, 2.0), (1.0, 1.0)] {
                    let mut p = Params::default();
                    p.set("skin", i as f64);
                    p.set("era", era);
                    p.set("state", state);
                    let m = make(kind, &p).unwrap();
                    let n = m.triangle_count();
                    let what = format!("{} {kind} era {era} state {state}", sk.key);
                    // A tower of forty floors, near, has many.
                    let most = if kind == "fine_world_tower" {
                        500_000
                    } else {
                        400_000
                    };
                    if !(300..=most).contains(&n) {
                        bad.push(format!("{what}: {n} triangles"));
                    }
                    for (role, _) in &m.parts {
                        if material_of(role).is_none() {
                            bad.push(format!("{what}: role «{role}» has no material"));
                        }
                    }
                    let loose = check::loose_pieces(&m, check::anchors_for(kind), 0.004);
                    if let Some(l) = loose.first() {
                        bad.push(format!(
                            "{what}: {} loose, {} {:.3} off at x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
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
        for b in &bad {
            println!("{b}");
        }
        assert!(bad.is_empty(), "{} problems", bad.len());
    }

    /// Every facility of every place, full and light: made, with their
    /// materials and nothing in the air.
    #[test]
    fn facilities_of_every_place() {
        let mut bad = Vec::new();
        for (i, sk) in SKINS.iter().enumerate() {
            for prog in civic::PROGRAMMES {
                for (lod, era, state) in [
                    (0.0, 0.0, 0.0),
                    (1.0, 0.0, 0.0),
                    (0.0, 1.0, 0.0),
                    (0.0, 0.0, 1.0),
                    (0.0, 0.0, 2.0),
                ] {
                    let mut p = Params::default();
                    p.set("skin", i as f64);
                    p.set("programme", prog.index() as f64);
                    p.set("lod", lod);
                    p.set("era", era);
                    p.set("state", state);
                    let kind = "fine_world_facility";
                    let m = make(kind, &p).unwrap();
                    let n = m.triangle_count();
                    let what = format!(
                        "{} {} lod {lod} era {era} state {state}",
                        sk.key,
                        prog.name()
                    );
                    if !(200..=400_000).contains(&n) {
                        bad.push(format!("{what}: {n} triangles"));
                    }
                    for (role, _) in &m.parts {
                        if material_of(role).is_none() {
                            bad.push(format!("{what}: role «{role}» has no material"));
                        }
                    }
                    let loose = check::loose_pieces(&m, check::anchors_for(kind), 0.004);
                    if let Some(l) = loose.first() {
                        bad.push(format!(
                            "{what}: {} loose, {} {:.3} off at x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
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
        bad.sort();
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
                for t in [
                    Typology::Townhouse,
                    Typology::Block,
                    Typology::House,
                    Typology::Tower,
                    Typology::Courtyard,
                ] {
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
