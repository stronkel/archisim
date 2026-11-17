//! What changed between two versions of a model, element by element: what
//! the 3D view needs to make again only what was touched (N0, third round,
//! `docs/TERRITORIO.md` §4.3).

use std::collections::{BTreeMap, BTreeSet};

use super::Model;

/// What changed from one version of a model to another, for what is
/// drawn in 3D.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// The ids of the elements added, removed or changed.
    pub ids: BTreeSet<u64>,
    /// Something every element may depend on changed (types, materials,
    /// levels, families, appearances…): everything is to be made again.
    pub global: bool,
}

impl Changes {
    /// Nothing that is drawn changed.
    pub fn is_empty(&self) -> bool {
        !self.global && self.ids.is_empty()
    }
}

/// The ids whose entries differ between two maps (added, removed or
/// changed), walking both in their order.
fn diff_map<K: Ord + Copy, V: PartialEq>(
    a: &BTreeMap<K, V>,
    b: &BTreeMap<K, V>,
    id: impl Fn(K) -> u64,
    out: &mut BTreeSet<u64>,
) {
    let mut x = a.iter().peekable();
    let mut y = b.iter().peekable();
    loop {
        match (x.peek(), y.peek()) {
            (None, None) => break,
            (Some((k, _)), None) => {
                out.insert(id(**k));
                x.next();
            }
            (None, Some((k, _))) => {
                out.insert(id(**k));
                y.next();
            }
            (Some((ka, va)), Some((kb, vb))) => match ka.cmp(kb) {
                std::cmp::Ordering::Less => {
                    out.insert(id(**ka));
                    x.next();
                }
                std::cmp::Ordering::Greater => {
                    out.insert(id(**kb));
                    y.next();
                }
                std::cmp::Ordering::Equal => {
                    if va != vb {
                        out.insert(id(**ka));
                    }
                    x.next();
                    y.next();
                }
            },
        }
    }
}

impl Model {
    /// What changed since `before` for the 3D view. Every field of the
    /// model is named here (adding one does not compile until it is said
    /// whether it is an element's, something every element may depend on,
    /// or not drawn in 3D).
    pub fn changes_since(&self, before: &Model) -> Changes {
        let Model {
            // Not drawn in 3D.
            project_name: _,
            project_guid: _,
            site_guid: _,
            building_guid: _,
            graphics: _,
            next_id: _,
            top_id: _,
            dimensions: _,
            elevation_marks: _,
            sections: _,
            presentation_styles: _,
            plan_views: _,
            level_styles: _,
            view_templates: _,
            ref_planes: _,
            cameras: _,
            title_blocks: _,
            default_title_block: _,
            sun: _,
            prices: _,
            budget: _,
            plot_area: _,
            room_usages: _,
            code: _,
            draw_phase: _,
            draw_layers: _,
            structural_site: _,
            structural_loads: _,
            electrical_site: _,
            water_site: _,
            thermal_site: _,
            ventilation_site: _,
            renewables_site: _,
            plan_template: _,
            drafts: _,
            info: _,
            sheets: _,
            draft_views: _,
            level_underlays: _,
            viewports: _,
            tags: _,
            tables: _,
            device_circuits: _,
            // Each element's.
            walls,
            slabs,
            openings,
            columns,
            beams,
            phases,
            disciplines,
            param_values,
            grids,
            footings,
            roofs,
            wall_roofs,
            stairs,
            terrains,
            curtain_walls,
            railings,
            objects,
            masses,
            edges,
            face_paints,
            plots,
            roads,
            roundabouts,
            ceilings,
            boxes,
            markings,
            decals,
            plantings,
            pools,
            elevators,
            runs,
            devices,
            reinforcements,
            footing_offsets,
            footing_piles,
            rooms,
            // What every element may depend on.
            levels,
            materials,
            wall_types,
            slab_types,
            families,
            opening_types,
            member_types,
            curtain_types,
            meshes,
            object_types,
            module_sources,
            modules,
            appearances,
            paint_zones,
            face_zones,
            true_north,
            georeference,
            orthophoto,
            framings,
            floor_systems,
            slab_joists,
            modular_site,
            catalogue,
            board_layouts,
            board_faces,
            board_plans,
            user_params,
        } = self;
        let b = before;
        let global = *levels != b.levels
            || *materials != b.materials
            || *wall_types != b.wall_types
            || *slab_types != b.slab_types
            || *families != b.families
            || *opening_types != b.opening_types
            || *member_types != b.member_types
            || *curtain_types != b.curtain_types
            || *meshes != b.meshes
            || *object_types != b.object_types
            || *module_sources != b.module_sources
            || *modules != b.modules
            || *appearances != b.appearances
            || *paint_zones != b.paint_zones
            || *face_zones != b.face_zones
            || *true_north != b.true_north
            || *georeference != b.georeference
            || *orthophoto != b.orthophoto
            || *framings != b.framings
            || *floor_systems != b.floor_systems
            || *slab_joists != b.slab_joists
            || *modular_site != b.modular_site
            || *catalogue != b.catalogue
            || *board_layouts != b.board_layouts
            || *board_faces != b.board_faces
            || *board_plans != b.board_plans
            || *user_params != b.user_params;
        let mut ids = BTreeSet::new();
        if !global {
            let o = &mut ids;
            diff_map(walls, &b.walls, |k| k.0, o);
            diff_map(slabs, &b.slabs, |k| k.0, o);
            diff_map(openings, &b.openings, |k| k.0, o);
            diff_map(columns, &b.columns, |k| k.0, o);
            diff_map(beams, &b.beams, |k| k.0, o);
            diff_map(phases, &b.phases, |k| k, o);
            diff_map(disciplines, &b.disciplines, |k| k, o);
            diff_map(param_values, &b.param_values, |k| k, o);
            diff_map(grids, &b.grids, |k| k.0, o);
            diff_map(footings, &b.footings, |k| k.0, o);
            diff_map(roofs, &b.roofs, |k| k.0, o);
            diff_map(wall_roofs, &b.wall_roofs, |k| k.0, o);
            diff_map(stairs, &b.stairs, |k| k.0, o);
            diff_map(terrains, &b.terrains, |k| k.0, o);
            diff_map(curtain_walls, &b.curtain_walls, |k| k.0, o);
            diff_map(railings, &b.railings, |k| k.0, o);
            diff_map(objects, &b.objects, |k| k.0, o);
            diff_map(masses, &b.masses, |k| k.0, o);
            diff_map(edges, &b.edges, |k| k.0, o);
            // A face's paint is its element's.
            let paints = |m: &BTreeMap<(u64, crate::FaceRef), crate::MaterialId>| {
                let mut by: BTreeMap<u64, Vec<(crate::FaceRef, crate::MaterialId)>> =
                    BTreeMap::new();
                for ((id, face), mat) in m {
                    by.entry(*id).or_default().push((*face, *mat));
                }
                by
            };
            diff_map(&paints(face_paints), &paints(&b.face_paints), |k| k, o);
            diff_map(plots, &b.plots, |k| k.0, o);
            diff_map(roads, &b.roads, |k| k.0, o);
            diff_map(roundabouts, &b.roundabouts, |k| k.0, o);
            diff_map(ceilings, &b.ceilings, |k| k.0, o);
            diff_map(boxes, &b.boxes, |k| k.0, o);
            diff_map(markings, &b.markings, |k| k.0, o);
            diff_map(decals, &b.decals, |k| k.0, o);
            diff_map(plantings, &b.plantings, |k| k.0, o);
            diff_map(pools, &b.pools, |k| k.0, o);
            diff_map(elevators, &b.elevators, |k| k.0, o);
            diff_map(runs, &b.runs, |k| k.0, o);
            diff_map(devices, &b.devices, |k| k.0, o);
            diff_map(reinforcements, &b.reinforcements, |k| k, o);
            diff_map(footing_offsets, &b.footing_offsets, |k| k, o);
            diff_map(footing_piles, &b.footing_piles, |k| k, o);
            diff_map(rooms, &b.rooms, |k| k.0, o);
        }
        Changes { ids, global }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CutPattern, Layer, LayerFunction, NewWall, Point2, ReferenceLine, WallFunction};

    #[test]
    fn a_moved_wall_is_the_only_change() {
        let mut m = Model::new("p");
        let level = m.add_level("Baja", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [180, 90, 60], CutPattern::Diagonal);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Interior,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = |x: f64| NewWall {
            wall_type: wt,
            level,
            start: Point2::new(x, 0.0),
            end: Point2::new(x + 4.0, 0.0),
            reference: ReferenceLine::Center,
            height: 2.7,
            base_offset: 0.0,
            bulge: 0.0,
            top: crate::WallTop::Unconnected,
        };
        let a = m.add_wall(wall(0.0)).unwrap();
        let b = m.add_wall(wall(10.0)).unwrap();
        let before = m.clone();
        assert!(m.changes_since(&before).is_empty());
        m.walls.get_mut(&a).unwrap().end.y = 0.5;
        let c = m.changes_since(&before);
        assert!(!c.global);
        assert_eq!(c.ids, BTreeSet::from([a.0]));
        m.remove_wall(b).unwrap();
        assert_eq!(m.changes_since(&before).ids, BTreeSet::from([a.0, b.0]));
        // A new material: everything may change.
        let mut n = before.clone();
        n.add_material(
            "Hormigón",
            "concrete",
            [200, 200, 200],
            CutPattern::Diagonal,
        );
        assert!(n.changes_since(&before).global);
    }
}
