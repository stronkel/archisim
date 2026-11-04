//! The drawings of types (elevations and sections of doors, windows and
//! curtain walls, layers of walls and slabs), made in `archisim-plan` so
//! the sheets can draw them too.

pub use archisim_plan::type_drawing::*;

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::OpeningType;

    #[test]
    fn every_library_curtain_wall_has_its_drawings() {
        let m = archisim_library::new_project("p");
        for t in m.curtain_types() {
            let model = crate::type_preview::curtain_model(&m, &t.design, 4.8, 3.0)
                .unwrap_or_else(|| panic!("{}", t.name));
            let (e, xs, ys) = curtain_elevation(&model).unwrap();
            assert!(e.width() > 4.8 && e.height() > 3.0, "{}", t.name);
            assert_eq!(
                xs.len(),
                model
                    .curtain_walls()
                    .next()
                    .unwrap()
                    .grid(&t.design)
                    .mullions
                    .len()
            );
            assert!(ys.len() >= 2);
            let p = curtain_plan_detail(&model).unwrap();
            assert!(p.items.iter().any(|i| matches!(
                i,
                Item::Rect {
                    pen: Pen::Thick,
                    ..
                }
            )));
            let s = curtain_section(&model).unwrap();
            assert!(s.height() > 3.0, "{}", t.name);
            assert!(e.svg(40.0).starts_with("<svg"));
        }
    }

    #[test]
    fn swing_doors_and_casements_show_their_opening_lines() {
        let m = archisim_library::new_project("p");
        let lines = |t: &OpeningType| {
            opening_elevation(&m, t)
                .unwrap()
                .items
                .iter()
                .filter(|i| matches!(i, Item::Line { pen: Pen::Thin, .. }))
                .count()
        };
        for t in m.opening_types() {
            let family = m.family(t.family).unwrap().name().to_string();
            let n = lines(t);
            match family.as_str() {
                // Two lines per leaf that turns.
                "Puerta abatible" => assert!(n >= 2, "{}: {n}", t.name),
                "Ventana practicable" => {
                    let turning = t.panels.iter().filter(|p| *p != "FIXEDCASEMENT").count();
                    assert!(n >= 2 * turning, "{}: {n}", t.name);
                }
                "Hueco" => assert_eq!(n, 0, "{}", t.name),
                _ => {}
            }
        }
    }

    #[test]
    fn every_library_door_and_window_has_an_elevation_and_a_section() {
        let m = archisim_library::new_project("p");
        for t in m.opening_types() {
            let e = opening_elevation(&m, t).unwrap_or_else(|| panic!("{}", t.name));
            // The wall around the opening frames it.
            assert!(e.width() > t.width + 2.0 * SURROUND, "{}", t.name);
            assert!(e.height() > t.height, "{}", t.name);
            let s = opening_section(&m, t).unwrap_or_else(|| panic!("{}", t.name));
            assert!(s.height() > t.height, "{}", t.name);
            // Something of the door or window is cut, unless it is a void.
            let thick = s
                .items
                .iter()
                .filter(|i| {
                    matches!(
                        i,
                        Item::Rect {
                            pen: Pen::Thick,
                            ..
                        }
                    )
                })
                .count();
            let walls = if t.is_door() { 1 } else { 2 };
            if t.is_void() {
                assert_eq!(thick, walls, "{}", t.name);
            } else {
                assert!(thick > walls, "{}", t.name);
            }
        }
    }

    #[test]
    fn layers_are_drawn_in_order() {
        let m = archisim_library::new_project("p");
        let wt = m.wall_types().next().unwrap();
        let d = layers_section(&m, &wt.layers, false);
        let firsts: Vec<f64> = d
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Rect {
                    min,
                    pen: Pen::Thin,
                    ..
                } => Some(min[0]),
                _ => None,
            })
            .collect();
        assert_eq!(firsts.len(), wt.layers.len());
        assert!(firsts.windows(2).all(|w| w[0] < w[1]));
        assert!((d.width() - wt.thickness() - 0.04).abs() < 1e-9);
        let st = m.slab_types().next().unwrap();
        let d = layers_section(&m, &st.layers, true);
        assert!((d.height() - st.thickness() - 0.04).abs() < 1e-9);
    }
}
