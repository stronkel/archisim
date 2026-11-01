//! The street types as cards (U3): each drawn as its cross-section, in its
//! group, with the types kept in the library («Tipos de calle.ascalle» in
//! the library folder) as a group of their own.

use std::path::PathBuf;

use archisim_core::{
    street_type_of, street_types, RoadBand, RoadSection, SavedStreet, StreetGroup,
};
use eframe::egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};

/// The library file of street types.
pub const STREET_FILE: &str = "Tipos de calle.ascalle";

/// The title of the group of the library's types.
const MINE: &str = "En mi biblioteca";

/// The street types of the library folder.
pub fn library_streets() -> Vec<SavedStreet> {
    std::fs::read_to_string(library_file())
        .map(|t| SavedStreet::decode_all(&t))
        .unwrap_or_default()
}

fn library_file() -> PathBuf {
    crate::editor::library::library_dir().join(STREET_FILE)
}

/// Keeps a street type in the library folder (replacing one of its name).
pub fn save_street(t: &SavedStreet) -> Result<PathBuf, String> {
    let path = library_file();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::write(&path, t.into_file(&old)).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The colour of a band in the section's picture.
pub fn band_colour(b: RoadBand) -> Color32 {
    match b {
        RoadBand::Carriageway => Color32::from_rgb(110, 114, 120),
        RoadBand::Parking => Color32::from_rgb(146, 150, 156),
        RoadBand::Kerb => Color32::from_rgb(236, 236, 232),
        RoadBand::Sidewalk | RoadBand::TreePits => Color32::from_rgb(222, 208, 180),
        RoadBand::BikeLane => Color32::from_rgb(200, 112, 92),
        RoadBand::BikePath => Color32::from_rgb(214, 140, 118),
        RoadBand::Planting => Color32::from_rgb(128, 168, 92),
        RoadBand::Median => Color32::from_rgb(212, 204, 178),
        RoadBand::BusLane => Color32::from_rgb(168, 86, 76),
        RoadBand::Tram => Color32::from_rgb(146, 182, 106),
        RoadBand::Shoulder => Color32::from_rgb(160, 162, 166),
        RoadBand::Ditch => Color32::from_rgb(112, 146, 84),
    }
}

/// A section drawn across, as seen along the street: each strip at its
/// height (sidewalks and kerbs up, ditches down), the trees of its green
/// strips and tree pits, the lines between lanes and the rails of a tram.
pub fn paint_section(ui: &Ui, r: Rect, s: &RoadSection) {
    let painter = ui.painter();
    let w = s.width();
    if w <= 0.0 {
        return;
    }
    let r = r.shrink(4.0);
    let k = r.width() as f64 / w;
    let ground = r.max.y - r.height() * 0.36;
    let road_h = 9.0;
    let up = if s.single_level { 0.0 } else { 4.0 };
    let ink = Color32::from_gray(70);
    let mut x = r.min.x as f64;
    let bands: Vec<(RoadBand, f32, f32)> = s
        .strips
        .iter()
        .map(|st| {
            let x0 = x as f32;
            x += st.width * k;
            (st.band, x0, x as f32)
        })
        .collect();
    for (i, &(band, x0, x1)) in bands.iter().enumerate() {
        let lift = if s.raised(band) { up } else { 0.0 };
        let top = match band {
            RoadBand::Ditch => ground + 3.0,
            _ => ground - lift,
        };
        let rect = Rect::from_min_max(Pos2::new(x0, top), Pos2::new(x1, ground + road_h));
        painter.rect_filled(rect, 0.0, band_colour(band));
        painter.line_segment(
            [Pos2::new(x0, top), Pos2::new(x1, top)],
            Stroke::new(1.0, ink),
        );
        // The lines between two lanes.
        if band == RoadBand::Carriageway
            && bands
                .get(i + 1)
                .is_some_and(|n| n.0 == RoadBand::Carriageway)
        {
            painter.line_segment(
                [Pos2::new(x1, top - 0.5), Pos2::new(x1, top + 1.5)],
                Stroke::new(1.5, Color32::WHITE),
            );
        }
        let mid = (x0 + x1) / 2.0;
        match band {
            // Trees: a trunk and a crown.
            RoadBand::Planting | RoadBand::TreePits if x1 - x0 > 2.0 => {
                let crown = (r.height() * 0.2).min(10.0);
                let foot = top;
                let head = Pos2::new(mid, r.min.y + crown + 1.0);
                painter.line_segment(
                    [Pos2::new(mid, foot), Pos2::new(mid, head.y + crown * 0.6)],
                    Stroke::new(1.4, Color32::from_rgb(110, 84, 60)),
                );
                painter.circle(
                    head,
                    crown,
                    Color32::from_rgba_unmultiplied(96, 146, 72, 220),
                    Stroke::new(0.8, Color32::from_rgb(60, 100, 50)),
                );
            }
            // A tram's rails.
            RoadBand::Tram => {
                for f in [0.25, 0.4, 0.6, 0.75] {
                    let px = x0 + (x1 - x0) * f;
                    painter.line_segment(
                        [Pos2::new(px, top - 1.0), Pos2::new(px, top + 1.0)],
                        Stroke::new(1.5, Color32::from_gray(60)),
                    );
                }
            }
            // A parked car.
            RoadBand::Parking if x1 - x0 > 3.0 => {
                let car = Rect::from_center_size(
                    Pos2::new(mid, top - 3.5),
                    Vec2::new((x1 - x0) * 0.7, 6.0),
                );
                painter.rect_filled(car, 2.0, Color32::from_rgb(84, 104, 140));
            }
            // A bus.
            RoadBand::BusLane if x1 - x0 > 3.0 => {
                let bus = Rect::from_min_max(
                    Pos2::new(x0 + (x1 - x0) * 0.12, r.min.y + r.height() * 0.3),
                    Pos2::new(x1 - (x1 - x0) * 0.12, top),
                );
                painter.rect_filled(bus, 2.0, Color32::from_rgb(196, 70, 56));
            }
            _ => {}
        }
    }
    // Its width.
    painter.line_segment(
        [
            Pos2::new(r.min.x, ground + road_h),
            Pos2::new(r.max.x, ground + road_h),
        ],
        Stroke::new(1.0, ink),
    );
    painter.text(
        Pos2::new(r.center().x, r.max.y - 1.0),
        eframe::egui::Align2::CENTER_BOTTOM,
        format!("{:.2} m", w).replace('.', ","),
        eframe::egui::FontId::proportional(9.0),
        Color32::from_gray(90),
    );
}

/// What was picked among the street types' cards.
pub enum PickedStreet {
    /// One of ArchiSim's types: its section.
    Type(RoadSection),
    /// One of the library: its section and materials.
    Saved(SavedStreet),
}

/// The street type of a section as a card, and every type when clicked;
/// the one picked.
pub fn street_type_cards(ui: &mut Ui, width: f32, s: &RoadSection) -> Option<PickedStreet> {
    let types = street_types();
    let saved = library_streets();
    let builtin = |n: &str| types.iter().any(|t| t.name == n);
    let saved_name = |t: &SavedStreet| {
        if builtin(&t.name) {
            format!("{} (biblioteca)", t.name)
        } else {
            t.name.clone()
        }
    };
    let mut groups: Vec<(String, Vec<String>)> = StreetGroup::ALL
        .iter()
        .map(|g| {
            (
                g.name().to_string(),
                types
                    .iter()
                    .filter(|t| t.group == *g)
                    .map(|t| t.name.to_string())
                    .collect(),
            )
        })
        .collect();
    groups.push((MINE.to_string(), saved.iter().map(saved_name).collect()));
    let alike = |a: &RoadSection, b: &RoadSection| {
        a.strips == b.strips && a.single_level == b.single_level && a.parking == b.parking
    };
    let current = street_type_of(s)
        .map(|t| t.name.to_string())
        .or_else(|| saved.iter().find(|t| alike(&t.section, s)).map(saved_name))
        .unwrap_or_else(|| "A medida".to_string());
    let section_of = |n: &str| -> Option<RoadSection> {
        types
            .iter()
            .find(|t| t.name == n)
            .map(|t| t.section.clone())
            .or_else(|| {
                saved
                    .iter()
                    .find(|t| saved_name(t) == n)
                    .map(|t| t.section.clone())
            })
    };
    let picked = super::cards::grouped_cards(ui, width, &groups, &current, &mut |ui, r, n| {
        let sec = if n == "A medida" {
            Some(s.clone())
        } else {
            section_of(n)
        };
        if let Some(sec) = sec {
            paint_section(ui, r, &sec);
        }
    })?;
    if let Some(t) = types.iter().find(|t| t.name == picked) {
        return Some(PickedStreet::Type(t.section.clone()));
    }
    saved
        .into_iter()
        .find(|t| saved_name(t) == picked)
        .map(PickedStreet::Saved)
}
