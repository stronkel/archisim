//! The tool icons of the disciplines (`docs/INTERFAZ.md` §5): drawings in the
//! language of architectural drawing, one line weight on a 20 × 20 grid,
//! the same ones as `docs/interfaz/iconos.svg`. Each is drawn once per size
//! in white and tinted with the colour it is shown in.

use std::collections::HashMap;

use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, TextureOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glyph {
    Edge,
    FaceRect,
    FaceCircle,
    PushPull,
    FollowMe,
    FaceOffset,
    Volume,
    Mass,
    Tape,
    Paint,
    Grid,
    Convert,
    Wall,
    Slab,
    Door,
    Window,
    Opening,
    Roof,
    Stair,
    Railing,
    CurtainWall,
    Object,
    Room,
    Column,
    Beam,
    StructuralSlab,
    BearingWall,
    Footing,
    Raft,
    Pipe,
    Duct,
    Fixture,
    Light,
    Outlet,
    Terrain,
    Plot,
    Road,
    Tree,
    Dimension,
    LevelMark,
    Tag,
    Text,
    Section,
    Elevation,
    Sheet,
    Schedule,
    Cte,
    Takeoff,
    Link,
    Module,
    Versions,
    Clash,
    Issue,
    Requirements,
    Team,
    Publish,
    /// The magnifier of the command search.
    Search,
    Eye,
    Lock,
    Spline,
    PickLine,
    Align,
    Break,
    Join,
    SnapEnd,
    SnapMid,
    SnapEdge,
    SnapTrack,
    Ortho,
    Hatch,
    Line,
}

impl Glyph {
    /// The drawing, as SVG elements on the 20 × 20 grid.
    fn markup(self) -> &'static str {
        match self {
            Glyph::Edge => {
                r##"<path d="M3 15L15 4"/><circle cx="3" cy="15" r="1.2" fill="#fff"/><circle cx="15" cy="4" r="1.2" fill="#fff"/>"##
            }
            Glyph::FaceRect => r##"<path d="M3 13l6-6h9l-6 6z"/>"##,
            Glyph::FaceCircle => r##"<ellipse cx="10" cy="11" rx="7" ry="4"/>"##,
            Glyph::PushPull => {
                r##"<path d="M3 15l5-4h9l-5 4z"/><path d="M8 11V5h9v6M12 15V9" stroke-width="1"/><path d="M14 3v-1M12.5 2.5L14 1l1.5 1.5" stroke-width="1"/>"##
            }
            Glyph::FollowMe => {
                r##"<path d="M3 16c0-6 4-10 12-10"/><rect x="13" y="3" width="4" height="6" stroke-width="1"/>"##
            }
            Glyph::FaceOffset => {
                r##"<rect x="3" y="3" width="14" height="14"/><rect x="6" y="6" width="8" height="8" stroke-dasharray="1.6 1.4"/>"##
            }
            Glyph::Volume => {
                r##"<path d="M4 7l6-4 6 4v7l-6 4-6-4z"/><path d="M4 7l6 4 6-4M10 11v7" stroke-width="1"/>"##
            }
            Glyph::Mass => {
                r##"<path d="M4 6l6-3 6 3v10l-6 3-6-3z"/><path d="M4 9.5l6 3 6-3M4 13l6 3 6-3" stroke-width="0.8"/>"##
            }
            Glyph::Tape => {
                r##"<path d="M3 15L16 4"/><path d="M5 13l2 2M8 10.5l1.2 1.2M11 8l2 2M14 5.5l1.2 1.2" stroke-width="0.9"/><path d="M2 9l14 0" stroke-dasharray="1.5 1.5" stroke-width="0.8"/>"##
            }
            Glyph::Paint => {
                r##"<path d="M4 12l7-7 4 4-7 7z"/><path d="M15 9c1.5 2 1.5 4 0 5" stroke-width="1"/>"##
            }
            Glyph::Grid => {
                r##"<circle cx="4" cy="4" r="2" stroke-width="1"/><path d="M8 4h11M4 8v11" stroke-dasharray="3 1.5 0.5 1.5" stroke-width="1"/><path d="M12 8v11M8 13h11" stroke-dasharray="3 1.5 0.5 1.5" stroke-width="1"/>"##
            }
            Glyph::Convert => {
                r##"<path d="M3 8l4-3 4 3v6l-4 3-4-3z" stroke-width="1"/><path d="M12 10h3M14 8.5l1.5 1.5-1.5 1.5" stroke-width="1"/><path d="M16 6v9M19 6v9M16 6h3" stroke-width="1"/>"##
            }
            Glyph::Wall => {
                r##"<path d="M2 7h16M2 13h16"/><path d="M5 7l-3 3M9 7l-6 6M13 7l-6 6M17 7l-6 6M18 10l-3 3" stroke-width="0.8"/>"##
            }
            Glyph::Slab => r##"<path d="M2 13l5-6h11l-5 6z"/><path d="M2 13v2h11l5-6V7"/>"##,
            Glyph::Door => {
                r##"<path d="M2 16h3M15 16h3M5 16V5"/><path d="M5 5a11 11 0 0 1 10 11" stroke-dasharray="1.5 1.5"/>"##
            }
            Glyph::Window => {
                r##"<rect x="3" y="3" width="14" height="14"/><path d="M10 3v14"/><path d="M3 3l7 7-7 7M17 3l-7 7 7 7" stroke-width="0.8" stroke-dasharray="1.6 1.4"/>"##
            }
            Glyph::Opening => {
                r##"<rect x="4" y="4" width="12" height="12"/><path d="M4 4l12 12M16 4L4 16" stroke-width="0.8"/>"##
            }
            Glyph::Roof => {
                r##"<path d="M1 11L10 3l9 8"/><path d="M3 9.5L10 4l7 5.5" stroke-width="0.8"/><path d="M4 10v7h12v-7" stroke-width="1"/>"##
            }
            Glyph::Stair => {
                r##"<path d="M4 2v16M16 2v16M4 5h12M4 8h12M4 11h12M4 14h12"/><path d="M10 16V4l-2 2M10 4l2 2" stroke-width="1"/>"##
            }
            Glyph::Railing => {
                r##"<path d="M2 5h16"/><path d="M2 9h16" stroke-width="1"/><path d="M5 9v-4M15 9v-4" stroke-width="0.8"/><path d="M4 5v13M10 5v13M16 5v13" stroke-width="1"/><path d="M2 18h16" stroke-width="0.8"/>"##
            }
            Glyph::CurtainWall => {
                r##"<rect x="3" y="3" width="14" height="14"/><path d="M8 3v14M13 3v14M3 8h14M3 13h14" stroke-width="0.9"/>"##
            }
            Glyph::Object => {
                r##"<rect x="5" y="6" width="10" height="10" rx="2"/><path d="M5 6V4h10v2"/>"##
            }
            Glyph::Room => {
                r##"<rect x="3" y="3" width="14" height="14" stroke-dasharray="2 1.5"/><path d="M7 10h6M8 12.5h4" stroke-width="1"/>"##
            }
            Glyph::Column => {
                r##"<path d="M5 3h10M5 17h10"/><path d="M6 3v2h8V3M6 17v-2h8v2" stroke-width="1"/><path d="M8 5v10M12 5v10"/>"##
            }
            Glyph::Beam => {
                r##"<path d="M2 8h16M2 12h16"/><path d="M2 8v4M18 8v4"/><path d="M2 10h16" stroke-dasharray="2 1.5" stroke-width="0.8"/>"##
            }
            Glyph::StructuralSlab => {
                r##"<path d="M2 13l5-6h11l-5 6z"/><path d="M5 10h10M7 8l-2 4" stroke-width="0.8"/>"##
            }
            Glyph::BearingWall => {
                r##"<path d="M2 7h16M2 13h16"/><path d="M2 7l6 6M6 7l6 6M10 7l6 6M14 7l4 4M2 11l2 2" stroke-width="0.8"/>"##
            }
            Glyph::Footing => {
                r##"<rect x="3" y="11" width="14" height="5"/><rect x="8" y="3" width="4" height="8"/>"##
            }
            Glyph::Raft => {
                r##"<rect x="2" y="9" width="16" height="6"/><path d="M5 9v6M9 9v6M13 9v6" stroke-width="0.8" stroke-dasharray="1.5 1.5"/>"##
            }
            Glyph::Pipe => r##"<path d="M2 8h10a4 4 0 0 1 4 4v6M2 12h10"/>"##,
            Glyph::Duct => {
                r##"<rect x="2" y="6" width="16" height="8"/><path d="M2 6l16 8M18 6L2 14" stroke-width="0.8"/>"##
            }
            Glyph::Fixture => r##"<path d="M4 7h12v3a6 6 0 0 1-12 0z"/><path d="M10 4v3"/>"##,
            Glyph::Light => {
                r##"<circle cx="10" cy="10" r="6"/><path d="M6 6l8 8M14 6l-8 8" stroke-width="0.9"/>"##
            }
            Glyph::Outlet => r##"<path d="M5 12a5 5 0 0 1 10 0z"/><path d="M10 12v5M7 17h6"/>"##,
            Glyph::Terrain => {
                r##"<path d="M2 15c3-2 4-6 8-6s5 4 8 3M2 11c3-2 4-5 8-5s5 3 8 2" stroke-width="1"/>"##
            }
            Glyph::Plot => r##"<path d="M3 5l12-2 2 12-14 2z" stroke-dasharray="3 1.5 0.5 1.5"/>"##,
            Glyph::Road => r##"<path d="M2 5h16M2 15h16"/><path d="M2 10h3M8 10h4M15 10h3"/>"##,
            Glyph::Tree => {
                r##"<circle cx="10" cy="10" r="6"/><path d="M10 4v12M4 10h12M6 6l8 8M14 6l-8 8" stroke-width="0.7"/>"##
            }
            Glyph::Dimension => {
                r##"<path d="M2 12h16M4 8v8M16 8v8"/><path d="M3 13l2-2M15 13l2-2"/>"##
            }
            Glyph::LevelMark => r##"<path d="M5 12h10M10 12l-3-5h6z"/>"##,
            Glyph::Tag => r##"<path d="M3 7h9l4 3-4 3H3z"/><circle cx="6" cy="10" r="0.8"/>"##,
            Glyph::Text => r##"<path d="M5 5h10M10 5v11M8 16h4"/>"##,
            Glyph::Section => {
                r##"<path d="M10 2v16" stroke-dasharray="3 1.5 0.5 1.5"/><path d="M10 3h5l-2-2M10 17h5l-2 2"/>"##
            }
            Glyph::Elevation => {
                r##"<circle cx="10" cy="10" r="4"/><path d="M10 6L5 10l5 4" fill="#fff"/>"##
            }
            Glyph::Sheet => {
                r##"<rect x="2" y="4" width="16" height="12"/><rect x="12" y="12" width="5" height="3" stroke-width="0.8"/>"##
            }
            Glyph::Schedule => {
                r##"<rect x="3" y="4" width="14" height="12"/><path d="M3 8h14M3 12h14M8 4v12" stroke-width="0.9"/>"##
            }
            Glyph::Cte => {
                r##"<path d="M10 2l7 3v5c0 4-3 7-7 8-4-1-7-4-7-8V5z"/><path d="M7 10l2 2 4-4"/>"##
            }
            Glyph::Takeoff => {
                r##"<rect x="4" y="2" width="12" height="16" rx="1"/><path d="M7 6h6M7 10h2M11 10h2M7 14h2M11 14h2" stroke-width="1"/>"##
            }
            Glyph::Link => {
                r##"<path d="M8 12l4-4"/><path d="M9 6l2-2a3 3 0 0 1 4 4l-2 2M11 14l-2 2a3 3 0 0 1-4-4l2-2"/>"##
            }
            Glyph::Module => {
                r##"<rect x="2" y="9" width="6" height="7"/><path d="M2 9l3-3 3 3"/><rect x="12" y="9" width="6" height="7"/><path d="M12 9l3-3 3 3"/><path d="M8 12.5h4" stroke-dasharray="1.5 1.2" stroke-width="0.9"/>"##
            }
            Glyph::Versions => r##"<circle cx="10" cy="10" r="7"/><path d="M10 6v4l3 2"/>"##,
            Glyph::Clash => {
                r##"<rect x="3" y="3" width="9" height="9"/><rect x="8" y="8" width="9" height="9"/>"##
            }
            Glyph::Issue => r##"<path d="M3 4h14v9H9l-4 3v-3H3z"/>"##,
            Glyph::Requirements => {
                r##"<rect x="4" y="2" width="12" height="16" rx="1"/><path d="M7 7l1.5 1.5L11 6M7 12l1.5 1.5L11 11M12 8h2M12 13h2" stroke-width="1"/>"##
            }
            Glyph::Team => {
                r##"<circle cx="7" cy="7" r="2.5"/><circle cx="14" cy="8" r="2"/><path d="M2 16c0-3 2-5 5-5s5 2 5 5M12 16c0-2 1-4 3-4s3 2 3 4" stroke-width="1"/>"##
            }
            Glyph::Spline => {
                r##"<path d="M3 15C6 4 10 16 17 5"/><circle cx="3" cy="15" r="1.3" fill="#fff" stroke="none"/><circle cx="17" cy="5" r="1.3" fill="#fff" stroke="none"/>"##
            }
            Glyph::PickLine => {
                r##"<path d="M2 15L13 4" stroke-dasharray="2 1.6"/><path d="M10.5 10.5l6.5 2.4-2.8 1.3-1.3 2.8z" fill="#fff"/>"##
            }
            Glyph::Align => {
                r##"<path d="M3 2v16"/><rect x="3" y="5" width="9" height="3.5"/><rect x="3" y="11.5" width="13" height="3.5"/>"##
            }
            Glyph::Break => {
                r##"<path d="M2 11h6M12 11h6"/><path d="M8.6 7l-1.2 8M12.6 7l-1.2 8" stroke-width="0.9"/>"##
            }
            Glyph::Join => {
                r##"<path d="M2 14h6.5M11.5 14H18"/><path d="M6.5 9c1.6-3.2 5.4-3.2 7 0" stroke-width="0.9"/><path d="M11.8 8.6l1.7.4.6-1.7" stroke-width="0.9"/>"##
            }
            Glyph::SnapEnd => {
                r##"<path d="M3 17L13 7"/><rect x="12" y="2" width="6" height="6"/>"##
            }
            Glyph::SnapMid => r##"<path d="M2 15h16"/><path d="M10 7.5l4.2 7.5H5.8z"/>"##,
            Glyph::SnapEdge => {
                r##"<path d="M2 15h16"/><path d="M7 9h6l-6 6h6z" stroke-width="1"/>"##
            }
            Glyph::SnapTrack => {
                r##"<path d="M2 10h16M10 2v16" stroke-dasharray="2 1.6"/><circle cx="10" cy="10" r="2.2"/>"##
            }
            Glyph::Ortho => r##"<path d="M4 3v13h13"/><path d="M4 12h4v4" stroke-width="0.9"/>"##,
            Glyph::Hatch => {
                r##"<rect x="3" y="3" width="14" height="14"/><path d="M3 9l6-6M3 15L15 3M7 17l10-10M13 17l4-4" stroke-width="0.8"/>"##
            }
            Glyph::Line => {
                r##"<path d="M3 16L17 4"/><circle cx="3" cy="16" r="1.3" fill="#fff" stroke="none"/><circle cx="17" cy="4" r="1.3" fill="#fff" stroke="none"/>"##
            }
            Glyph::Eye => {
                r##"<path d="M2 10c2.5-4 5-6 8-6s5.5 2 8 6c-2.5 4-5 6-8 6s-5.5-2-8-6z"/><circle cx="10" cy="10" r="2.5"/>"##
            }
            Glyph::Lock => {
                r##"<rect x="4.5" y="9" width="11" height="8" rx="1"/><path d="M7 9V6.5a3 3 0 0 1 6 0V9"/>"##
            }
            Glyph::Search => r##"<circle cx="8.5" cy="8.5" r="5.5"/><path d="M12.6 12.6L17 17"/>"##,
            Glyph::Publish => {
                r##"<path d="M5 15a4 4 0 0 1 0-8 5 5 0 0 1 9.5 1.5A3.5 3.5 0 0 1 15 15z"/><path d="M10 9v5M8 11l2-2 2 2" stroke-width="1"/>"##
            }
        }
    }

    /// Every glyph (for tests and the reference sheet).
    #[cfg(test)]
    pub const ALL: &'static [Glyph] = &[
        Glyph::Edge,
        Glyph::FaceRect,
        Glyph::FaceCircle,
        Glyph::PushPull,
        Glyph::FollowMe,
        Glyph::FaceOffset,
        Glyph::Volume,
        Glyph::Mass,
        Glyph::Tape,
        Glyph::Paint,
        Glyph::Grid,
        Glyph::Convert,
        Glyph::Wall,
        Glyph::Slab,
        Glyph::Door,
        Glyph::Window,
        Glyph::Opening,
        Glyph::Roof,
        Glyph::Stair,
        Glyph::Railing,
        Glyph::CurtainWall,
        Glyph::Object,
        Glyph::Room,
        Glyph::Column,
        Glyph::Beam,
        Glyph::StructuralSlab,
        Glyph::BearingWall,
        Glyph::Footing,
        Glyph::Raft,
        Glyph::Pipe,
        Glyph::Duct,
        Glyph::Fixture,
        Glyph::Light,
        Glyph::Outlet,
        Glyph::Terrain,
        Glyph::Plot,
        Glyph::Road,
        Glyph::Tree,
        Glyph::Dimension,
        Glyph::LevelMark,
        Glyph::Tag,
        Glyph::Text,
        Glyph::Section,
        Glyph::Elevation,
        Glyph::Sheet,
        Glyph::Schedule,
        Glyph::Cte,
        Glyph::Takeoff,
        Glyph::Link,
        Glyph::Module,
        Glyph::Versions,
        Glyph::Clash,
        Glyph::Issue,
        Glyph::Requirements,
        Glyph::Team,
        Glyph::Publish,
        Glyph::Search,
        Glyph::Eye,
        Glyph::Lock,
        Glyph::Spline,
        Glyph::PickLine,
        Glyph::Align,
        Glyph::Break,
        Glyph::Join,
        Glyph::SnapEnd,
        Glyph::SnapMid,
        Glyph::SnapEdge,
        Glyph::SnapTrack,
        Glyph::Ortho,
        Glyph::Hatch,
        Glyph::Line,
    ];
}

#[derive(Clone, Default)]
struct Cache(HashMap<(Glyph, u32), TextureHandle>);

/// Draws `glyph` in `rect` (square), in `color`.
pub fn paint(painter: &egui::Painter, rect: Rect, glyph: Glyph, color: Color32) {
    let ctx = painter.ctx();
    let px = (rect.width().min(rect.height()) * ctx.pixels_per_point())
        .round()
        .max(4.0) as u32;
    let id = egui::Id::unique("tool-glyphs");
    let cached = ctx.data(|d| {
        d.get_temp::<Cache>(id)
            .and_then(|c| c.0.get(&(glyph, px)).cloned())
    });
    // Made outside the lock of the context's data (loading a texture takes
    // the context too).
    let tex = cached.unwrap_or_else(|| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" width="20" height="20" fill="none" stroke="#fff" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
            glyph.markup()
        );
        let img = crate::plan_image::rasterize_svg(&svg, [px as usize, px as usize]);
        let tex = ctx.load_texture(format!("glyph-{glyph:?}-{px}"), img, TextureOptions::LINEAR);
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<Cache>(id)
                .0
                .insert((glyph, px), tex.clone())
        });
        tex
    });
    let side = rect.width().min(rect.height());
    let r = Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
    painter.image(
        tex.id(),
        r,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every glyph draws something, inside its square.
    #[test]
    fn every_glyph_draws() {
        for g in Glyph::ALL {
            let svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" width="20" height="20" fill="none" stroke="#fff" stroke-width="1.3">{}</svg>"##,
                g.markup()
            );
            let img = crate::plan_image::rasterize_svg(&svg, [40, 40]);
            let inked = img.pixels.iter().filter(|p| p.a() > 0).count();
            assert!(inked > 30, "{g:?} draws {inked} pixels");
        }
    }
}
