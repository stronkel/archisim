//! Floor plan views rendered to SVG.
//!
//! The SVG is drawn at a real drawing scale: its width and height are in
//! paper millimetres, and line weights and hatch spacing are specified in
//! paper millimetres too, so printing it at 100 % gives a correct 1:50 (or
//! whatever scale is chosen) drawing.
//!
//! Drawing conventions (MVP):
//! - every cut layer is filled with its material colour and pattern;
//! - wall faces and free ends are drawn with the heavy cut line;
//! - layer boundaries are thin; mitred joins draw no line, so the poché of
//!   connected walls reads as continuous;
//! - insulation is drawn with the usual zigzag, cut to its layer; all
//!   hatching is real geometry cut to each layer (no SVG clip paths);
//! - slabs are drawn under the walls according to the cut plane: below it
//!   (the usual case: the floor you stand on) with a light cover fill and a
//!   thin projection line; crossing it as a cut, with the heavy line and the
//!   fill of its structural layer; above it with a thin dashed line.
//! - doors and windows crossed by the cut plane interrupt the wall: every
//!   layer stops at the jambs, which are drawn with the heavy cut line. The
//!   frame is drawn in the gap with a medium line; doors show the leaf open
//!   at 90° and its swing; windows show the glazing line, the sill board
//!   outside and the edge of the sill wall inside. Windows entirely above
//!   the cut plane are drawn dashed and do not interrupt the wall.

pub mod annotate;
pub mod axo;
pub mod concept;
pub mod curtain;
pub(crate) mod dim;
pub mod drafting;
pub mod grids;
pub mod lettering;
pub mod members;
pub mod phases;
pub mod railings;
pub mod roofs;
pub mod rooms;
pub mod schedules;
pub mod section;
pub mod sheet;
pub mod stairs;
pub mod terrain;

use std::fmt::Write as _;

use archisim_core::{
    CutPattern, LayerFunction, LevelId, LineStyle, Mark, Model, Pen, Point2, Slab, WallFunction,
    WallId, WallSide, EPSILON,
};
use archisim_geom::{
    clip_span, openings_in_wall, wall_geometry_in, wall_pieces, EndCut, OpeningGeometry,
    WallGeometry, WallIndex, WallPiece,
};

/// Options for a plan view.
#[derive(Debug, Clone)]
pub struct PlanOptions {
    /// Drawing scale denominator (50 means 1:50).
    pub scale: f64,
    /// Paper margin around the drawing, in millimetres.
    pub margin_mm: f64,
    /// Draw the walls' reference lines (dashed).
    pub show_reference_lines: bool,
    /// Height of the horizontal cut plane above the level, in metres.
    pub cut_plane: f64,
    /// Draw exactly this region of the plan (lower-left and upper-right
    /// corners, in model metres) instead of fitting the drawing to the
    /// elements. Used by on-screen views: no margin, title or background.
    pub viewport: Option<(Point2, Point2)>,
    /// Another level drawn underneath in grey as a reference to draw on
    /// (ArchiCAD's "Calco", Revit's "Calco subyacente"), usually the one
    /// below. Its walls cut by this plan's cut plane are drawn cut anyway.
    pub reference_level: Option<LevelId>,
    /// Screen pixels per paper millimetre, for on-screen views: details
    /// smaller than the screen can show (hatches, the insulation zigzag,
    /// the symbols of tiny openings) are left out when zoomed out, so a
    /// large model stays quick. `None` (exports): everything.
    pub px_per_mm: Option<f64>,
    /// Every line drawn as a hairline, whatever its weight (ArchiCAD's
    /// "Líneas finas"): one pixel on screen, 0.1 mm on paper. Off, lines
    /// have their real weight at the drawing scale.
    pub thin_lines: bool,
    /// Which phases of a refurbishment the plan shows.
    pub phase_view: phases::PhaseView,
    /// What the view shows (categories, layers, level of detail).
    pub style: archisim_core::ViewStyle,
    /// Line weights; `None`: the project's for this scale.
    pub pens: Option<archisim_core::Pens>,
    /// The title under an exported plan (project, level and scale).
    pub title: bool,
}

/// Width of a hairline on paper, in millimetres, for [`PlanOptions::thin_lines`]
/// when the plan is not for the screen.
pub const HAIRLINE_MM: f64 = 0.1;

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            scale: 50.0,
            margin_mm: 15.0,
            show_reference_lines: false,
            cut_plane: 1.1,
            viewport: None,
            reference_level: None,
            px_per_mm: None,
            thin_lines: false,
            phase_view: phases::PhaseView::All,
            style: Default::default(),
            pens: None,
            title: true,
        }
    }
}

/// Sets every `stroke-width` of an SVG to `width` (in the SVG's units).
pub fn hairlines(svg: &str, width: f64) -> String {
    let key = "stroke-width=\"";
    let w = num(width);
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(key) {
        out.push_str(&rest[..i + key.len()]);
        rest = &rest[i + key.len()..];
        match rest.find('"') {
            Some(j) => {
                out.push_str(&w);
                rest = &rest[j..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

thread_local! {
    /// Whether the plan being drawn leaves out hatches and zigzags (set by
    /// [`render_plan`] from [`PlanOptions::px_per_mm`]).
    static COARSE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Screen pixels per paper millimetre of the plan being drawn, if it is
    /// for the screen.
    static PX_PER_MM: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}

thread_local! {
    /// Line weights of the plan being drawn.
    static PENS: std::cell::Cell<archisim_core::Pens> =
        const { std::cell::Cell::new(archisim_core::Pens::FINE) };
    /// Level of detail of the plan being drawn.
    static DETAIL: std::cell::Cell<archisim_core::Detail> =
        const { std::cell::Cell::new(archisim_core::Detail::Fine) };
}

/// The line weights of the plan being drawn, paper millimetres.
pub(crate) fn pens() -> archisim_core::Pens {
    PENS.with(|p| p.get())
}

pub(crate) fn set_pens(pens: archisim_core::Pens) {
    PENS.with(|p| p.set(pens));
}

fn detail() -> archisim_core::Detail {
    DETAIL.with(|d| d.get())
}

fn coarse() -> bool {
    detail() != archisim_core::Detail::Fine || COARSE.with(|c| c.get())
}

/// Openings narrower than this on screen, in pixels, are drawn as the gap
/// in the wall only, without their symbol.
const MIN_SYMBOL_PX: f64 = 8.0;

/// Walls thinner than this on screen, in pixels, are drawn as one band.
const MIN_LAYERS_PX: f64 = 4.0;

/// The colour of a wall drawn as one band.
const FAR_POCHE: &str = "rgb(150,146,138)";

/// Whether a wall this thick in paper millimetres is too thin on screen to
/// draw its layers.
fn too_small_for_layers(paper_mm: f64) -> bool {
    detail() == archisim_core::Detail::Coarse
        || PX_PER_MM
            .with(|c| c.get())
            .is_some_and(|px| paper_mm * px < MIN_LAYERS_PX)
}

/// Whether a length in paper millimetres is too small on screen for the
/// symbol of a door or window.
fn too_small_for_symbol(paper_mm: f64) -> bool {
    PX_PER_MM
        .with(|c| c.get())
        .is_some_and(|px| paper_mm * px < MIN_SYMBOL_PX)
}

/// Hatches closer than this on screen, in pixels, are not drawn.
const MIN_HATCH_PX: f64 = 2.5;

/// Line weights in paper millimetres.
// Line weights come from the project's graphic standard, by scale: see
// [`pens`].
const REF_LINE: f64 = 0.13;
const SLAB_COVER: &str = "#f1f1ee";
const HATCH_SPACING: f64 = 1.5;
const TEXT_SIZE: f64 = 3.0;
const INK: &str = "#1a1a1a";

/// Walls of other levels that a plan shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OtherWalls {
    /// Walls of other levels crossed by this plan's cut plane (a wall of
    /// the floor below that rises through this one): drawn cut, as Revit
    /// does with whatever its view range cuts. Each one with the height of
    /// the cut above its own level.
    pub cut: Vec<(WallId, f64)>,
    /// Walls of the reference level not already cut: drawn in grey.
    pub reference: Vec<WallId>,
}

impl OtherWalls {
    /// Every wall listed, cut or reference.
    pub fn ids(&self) -> Vec<WallId> {
        self.cut
            .iter()
            .map(|c| c.0)
            .chain(self.reference.iter().copied())
            .collect()
    }
}

/// The walls of other levels the plan of `level` shows, with its cut plane
/// `cut_plane` metres above the level and `reference` drawn underneath.
pub fn other_level_walls(
    model: &Model,
    level: LevelId,
    cut_plane: f64,
    reference: Option<LevelId>,
) -> OtherWalls {
    let elevation = |l: LevelId| model.level(l).map(|l| l.elevation);
    let Some(here) = elevation(level) else {
        return OtherWalls::default();
    };
    let cut_at = here + cut_plane;
    let mut out = OtherWalls::default();
    for w in model.walls().filter(|w| w.level != level) {
        let Some(base_level) = elevation(w.level) else {
            continue;
        };
        let base = base_level + w.base_offset;
        let top = base + w.height;
        if base < cut_at - EPSILON && top > cut_at + EPSILON {
            out.cut.push((w.id, cut_at - base_level));
        } else if Some(w.level) == reference {
            out.reference.push(w.id);
        }
    }
    out
}

/// The level right below `level`, if any.
pub fn level_below(model: &Model, level: LevelId) -> Option<LevelId> {
    let here = model.level(level)?.elevation;
    model
        .levels()
        .into_iter()
        .filter(|l| l.elevation < here - EPSILON)
        .map(|l| l.id)
        .next_back()
}

/// Renders the plan of one level as an SVG document.
pub fn render_plan(model: &Model, level: LevelId, options: &PlanOptions) -> String {
    let coarse = options
        .px_per_mm
        .is_some_and(|px| px * HATCH_SPACING < MIN_HATCH_PX);
    COARSE.with(|c| c.set(coarse));
    PX_PER_MM.with(|c| c.set(options.px_per_mm));
    let pens = options
        .pens
        .unwrap_or_else(|| model.graphics.pens(options.scale));
    PENS.with(|p| p.set(pens));
    DETAIL.with(|d| d.set(options.style.detail));
    let phased = phases::phase_model(model, options.phase_view);
    // Ids are the same in the phased copy: what is greyed comes from the
    // model as it is.
    let _greyed = dim::begin(model, &options.style);
    let svg = render_plan_inner(phased.as_ref().unwrap_or(model), level, options);
    COARSE.with(|c| c.set(false));
    PX_PER_MM.with(|c| c.set(None));
    PENS.with(|p| p.set(archisim_core::Pens::FINE));
    DETAIL.with(|d| d.set(archisim_core::Detail::Fine));
    if options.thin_lines {
        // One pixel on screen; a hairline on paper. In model metres.
        let k = options.scale / 1000.0;
        let mm = options
            .px_per_mm
            .map_or(HAIRLINE_MM, |px| 1.0 / px.max(1e-6));
        return hairlines(&svg, mm * k);
    }
    svg
}

fn render_plan_inner(model: &Model, level: LevelId, options: &PlanOptions) -> String {
    // Paper millimetres → model metres.
    let k = options.scale / 1000.0;
    let others = other_level_walls(model, level, options.cut_plane, options.reference_level);
    // On screen only what the view shows (with a margin, so nothing pops
    // in at the edges), found through an index: large models stay fast.
    let idx = WallIndex::new(model);
    let visible = |lo: Point2, hi: Point2| match options.viewport {
        Some((a, b)) => {
            let m = 1.0;
            lo.x <= b.x + m && hi.x >= a.x - m && lo.y <= b.y + m && hi.y >= a.y - m
        }
        None => true,
    };
    let wall_visible = |id: WallId| {
        model.wall(id).is_some_and(|w| {
            let (lo, hi) = archisim_geom::index::bounds(model, w);
            visible(lo, hi)
        })
    };
    // Walls of this level, then those of other levels it cuts, each with
    // the height of the cut above its own level.
    let show = |c: archisim_core::Category| options.style.shows(c);
    let mut geos: Vec<WallGeometry> = model
        .walls_on_level(level)
        .filter(|_| show(archisim_core::Category::Walls))
        .filter(|w| wall_visible(w.id))
        .filter_map(|w| wall_geometry_in(&idx, w.id))
        .collect();
    let mut cuts = vec![options.cut_plane; geos.len()];
    for (id, z) in &others.cut {
        if !wall_visible(*id) || !show(archisim_core::Category::Walls) {
            continue;
        }
        if let Some(g) = wall_geometry_in(&idx, *id) {
            geos.push(g);
            cuts.push(*z);
        }
    }
    let reference: Vec<WallGeometry> = others
        .reference
        .iter()
        .filter(|id| wall_visible(**id))
        .filter_map(|id| wall_geometry_in(&idx, *id))
        .collect();
    let slabs: Vec<&Slab> = model
        .slabs_on_level(level)
        .filter(|_| show(archisim_core::Category::Slabs))
        .filter(|s| {
            let lo = s
                .outline
                .iter()
                .fold(Point2::new(f64::INFINITY, f64::INFINITY), |a, p| {
                    Point2::new(a.x.min(p.x), a.y.min(p.y))
                });
            let hi = s
                .outline
                .iter()
                .fold(Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |a, p| {
                    Point2::new(a.x.max(p.x), a.y.max(p.y))
                });
            visible(lo, hi)
        })
        .collect();

    let mut min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let all_points = geos
        .iter()
        .flat_map(|g| g.outline.iter())
        .chain(slabs.iter().flat_map(|s| s.outline.iter()));
    let dimensions: Vec<_> = model
        .dimensions_on_level(level)
        .filter(|_| show(archisim_core::Category::Dimensions))
        .collect();
    let marks: Vec<_> = model
        .elevation_marks_on_level(level)
        .filter(|_| show(archisim_core::Category::LevelMarks))
        .collect();
    let annotation_points: Vec<Point2> = dimensions
        .iter()
        .flat_map(|d| {
            let g = annotate::dimension_lines(d, k);
            let mut v = vec![g.line.0, g.line.1];
            v.extend(g.extensions.iter().map(|e| e.1));
            v
        })
        .chain(marks.iter().map(|m| m.point))
        .chain(
            model
                .drafts_in(archisim_core::ViewRef::Level(level))
                .filter(|_| show(archisim_core::Category::Drafting))
                .filter(|d| model.draft_visible(d) && options.style.shows_layer(d.layer))
                .flat_map(|d| {
                    let (a, b) = archisim_draft::bounds(&d.shape, k);
                    [a, b]
                }),
        )
        .chain(
            model
                .rooms_on_level(level)
                .filter(|_| show(archisim_core::Category::Rooms))
                .map(|r| r.label),
        )
        .chain(
            model
                .tags_on_level(level)
                .filter(|_| show(archisim_core::Category::Tags))
                .map(|t| t.at),
        )
        .collect();
    let structure_points = members::member_points(model, level, options.cut_plane);
    let mut marker_points = section::marker_points(model, k);
    for roof in model.roofs_on_level(level) {
        marker_points.extend(roofs::roof_points(roof));
    }
    for stair in model.stairs_on_level(level) {
        marker_points.extend(stairs::stair_points(stair));
    }
    for (stair, _) in stairs::arriving(model, level) {
        marker_points.extend(stairs::stair_points(stair));
    }
    for t in model.terrains_on_level(level) {
        marker_points.extend(terrain::terrain_points(t));
    }
    for c in model.curtain_walls_on_level(level) {
        marker_points.extend(curtain::curtain_points(model, c));
    }
    for r in model.railings_on_level(level) {
        marker_points.extend(railings::railing_points(r));
    }
    for o in model.objects_on_level(level) {
        marker_points.extend(model.object_outline(o.id));
    }
    if show(archisim_core::Category::Grids) {
        marker_points.extend(grids::grid_points(model, k));
    }
    if show(archisim_core::Category::Foundations) {
        for f in model.footings_on_level(level) {
            marker_points.extend(f.outline());
        }
    }
    if show(archisim_core::Category::Masses) {
        for m in model.masses_on_level(level) {
            marker_points.extend(m.outline());
        }
    }
    let all_points = all_points
        .chain(annotation_points.iter())
        .chain(marker_points.iter())
        .chain(structure_points.iter());
    for p in all_points {
        min = Point2::new(min.x.min(p.x), min.y.min(p.y));
        max = Point2::new(max.x.max(p.x), max.y.max(p.y));
    }
    if geos.is_empty()
        && slabs.is_empty()
        && annotation_points.is_empty()
        && structure_points.is_empty()
        && marker_points.is_empty()
    {
        min = Point2::new(0.0, 0.0);
        max = Point2::new(1.0, 1.0);
    }
    let (vb_x, vb_y, vb_w, vb_h) = match options.viewport {
        Some((lo, hi)) => (
            lo.x,
            -hi.y,
            (hi.x - lo.x).max(EPSILON),
            (hi.y - lo.y).max(EPSILON),
        ),
        None => {
            let margin = options.margin_mm * k;
            let title_space = 4.0 * TEXT_SIZE * k;
            // SVG y grows downwards: plot (x, -y).
            (
                min.x - margin,
                -max.y - margin,
                (max.x - min.x) + 2.0 * margin,
                (max.y - min.y) + 2.0 * margin + title_space,
            )
        }
    };
    let margin = options.margin_mm * k;

    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.2}mm" height="{:.2}mm" viewBox="{} {} {} {}">"#,
        vb_w / k,
        vb_h / k,
        num(vb_x),
        num(vb_y),
        num(vb_w),
        num(vb_h)
    );
    if options.viewport.is_none() {
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
            num(vb_x),
            num(vb_y),
            num(vb_w),
            num(vb_h)
        );
    }

    s.push_str("<g id=\"slabs\">\n");
    for t in model
        .terrains_on_level(level)
        .filter(|_| show(archisim_core::Category::Terrain))
    {
        dim::element(&mut s, t.id.0, |s| terrain::draw_terrain(s, t, k));
    }
    for (si, slab) in slabs.iter().enumerate() {
        dim::element(&mut s, slab.id.0, |s| {
            draw_slab(s, model, slab, si, options.cut_plane, k)
        });
    }
    for roof in model
        .roofs_on_level(level)
        .filter(|_| show(archisim_core::Category::Roofs))
    {
        let (lo, hi) = bounds(&roof.eaves());
        if visible(lo, hi) {
            dim::element(&mut s, roof.id.0, |s| roofs::draw_roof(s, roof, k));
        }
    }
    for stair in model
        .stairs_on_level(level)
        .filter(|_| show(archisim_core::Category::Stairs))
    {
        let pts = stairs::stair_points(stair);
        let (lo, hi) = bounds(&pts);
        if visible(lo, hi) {
            dim::element(&mut s, stair.id.0, |s| {
                stairs::draw_stair(s, stair, options.cut_plane, k)
            });
        }
    }
    // Stairs arriving at this level, seen from above through their hole:
    // the whole stair below the cut plane.
    for (stair, rise) in stairs::arriving(model, level)
        .into_iter()
        .filter(|_| show(archisim_core::Category::Stairs))
    {
        let pts = stairs::stair_points(stair);
        let (lo, hi) = bounds(&pts);
        if visible(lo, hi) {
            dim::element(&mut s, stair.id.0, |s| {
                stairs::draw_stair(s, stair, rise + options.cut_plane, k)
            });
        }
    }
    s.push_str("</g>\n");
    if show(archisim_core::Category::Foundations) {
        let vis = |poly: &[Point2]| {
            let (lo, hi) = bounds(poly);
            visible(lo, hi)
        };
        grids::draw_footings(&mut s, model, level, k, &vis);
    }
    if show(archisim_core::Category::Masses) {
        let vis = |poly: &[Point2]| {
            let (lo, hi) = bounds(poly);
            visible(lo, hi)
        };
        concept::draw_masses(&mut s, model, level, k, &vis);
    }
    if show(archisim_core::Category::Grids) {
        grids::draw_grids(&mut s, model, k);
    }
    if !reference.is_empty() {
        draw_reference(&mut s, model, &reference, k);
    }

    // Openings of each wall, and the solid pieces left by those that are cut.
    let openings: Vec<Vec<OpeningGeometry>> =
        geos.iter().map(|g| idx.openings_in_wall(g.wall)).collect();
    let pieces: Vec<Vec<WallPiece>> = openings
        .iter()
        .zip(cuts.iter().copied())
        .map(|ops| {
            let spans: Vec<(f64, f64)> = ops
                .0
                .iter()
                .filter(|o| opening_view(o, ops.1) == OpeningView::Cut)
                .map(|o| o.cut_span())
                .collect();
            wall_pieces(&spans)
        })
        .collect();

    s.push_str("<g id=\"poche\">\n");
    for (gi, g) in geos.iter().enumerate() {
        let cut: Vec<OpeningGeometry> = openings[gi]
            .iter()
            .filter(|o| opening_view(o, cuts[gi]) == OpeningView::Cut)
            .cloned()
            .collect();
        dim::element(&mut s, g.wall.0, |s| {
            draw_layers(s, model, g, gi, &pieces[gi], &cut, cuts[gi], k)
        });
    }
    s.push_str("</g>\n<g id=\"openings\" stroke-linejoin=\"miter\">\n");
    for (ops, cut) in openings.iter().zip(&cuts) {
        for o in ops {
            if !show(archisim_core::Category::Openings) {
                continue;
            }
            dim::element(&mut s, o.opening.0, |s| match opening_view(o, *cut) {
                OpeningView::Cut => draw_opening(s, model, o, options.scale, k),
                OpeningView::Above => draw_opening_above(s, o, k),
                OpeningView::Below => {}
            });
        }
    }
    // Cut lines go last so frames and sill boards never cover the jambs.
    s.push_str("</g>\n<g id=\"cut-lines\" fill=\"none\" stroke-linecap=\"square\">\n");
    for ((g, p), cut) in geos.iter().zip(&pieces).zip(&cuts) {
        dim::element(&mut s, g.wall.0, |s| draw_cut_lines(s, g, p, *cut, k));
    }
    s.push_str("</g>\n");
    // Columns and beams on top: the structure goes through the walls.
    let poly_visible = |poly: &[Point2]| {
        let (lo, hi) = bounds(poly);
        visible(lo, hi)
    };
    if show(archisim_core::Category::Objects) {
        draw_objects(&mut s, model, level, options.scale, k, &poly_visible);
    }
    if show(archisim_core::Category::Structure) {
        members::draw_members(&mut s, model, level, options.cut_plane, k, &poly_visible);
    }
    s.push_str("<g id=\"curtain-walls\">\n");
    for c in model
        .curtain_walls_on_level(level)
        .filter(|_| show(archisim_core::Category::CurtainWalls))
    {
        if poly_visible(&curtain::curtain_points(model, c)) {
            dim::element(&mut s, c.id.0, |s| {
                curtain::draw_curtain_wall(s, model, c, options.cut_plane, k)
            });
        }
    }
    s.push_str("</g>\n<g id=\"railings\">\n");
    for r in model
        .railings_on_level(level)
        .filter(|_| show(archisim_core::Category::Railings))
    {
        if poly_visible(&r.path) {
            dim::element(&mut s, r.id.0, |s| {
                railings::draw_railing(s, r, options.cut_plane, k)
            });
        }
    }
    s.push_str("</g>\n");

    if show(archisim_core::Category::Drafting) {
        drafting::draw_drafts(
            &mut s,
            model,
            archisim_core::ViewRef::Level(level),
            k,
            options.viewport,
            options.viewport.is_none(),
            &options.style.hidden_layers,
            !show(archisim_core::Category::Dimensions),
        );
    }
    if show(archisim_core::Category::Rooms) {
        rooms::draw_rooms(&mut s, model, level, k);
    }
    if show(archisim_core::Category::Tags) {
        rooms::draw_tags(&mut s, model, level, k);
    }
    s.push_str("<g id=\"annotations\">\n");
    for d in &dimensions {
        annotate::draw_dimension(&mut s, d, k);
    }
    for m in &marks {
        annotate::draw_elevation_mark(&mut s, model, m, k);
    }
    if show(archisim_core::Category::SectionMarks) {
        section::draw_markers(&mut s, model, k);
    }
    s.push_str("</g>\n");

    if options.show_reference_lines {
        let _ = writeln!(
            s,
            r##"<g id="reference-lines" stroke="#c0392b" stroke-width="{}" stroke-dasharray="{} {}">"##,
            num(REF_LINE * k),
            num(4.0 * k),
            num(1.5 * k)
        );
        for g in &geos {
            let (a, b) = g.axis;
            match &g.arc {
                Some(arc) => {
                    let pts = arc.polyline(0.0, archisim_core::CURVE_TOLERANCE);
                    let _ = writeln!(s, "<path d=\"{}\"/>", open_path(&pts));
                }
                None => {
                    let _ = writeln!(s, "<path d=\"M{} L{}\"/>", pt(a), pt(b));
                }
            }
        }
        s.push_str("</g>\n");
    }

    if options.viewport.is_none() {
        // The north arrow, in the top right corner.
        s.push_str("<g id=\"north\">\n");
        terrain::draw_north(
            &mut s,
            Point2::new(max.x - 8.0 * k, max.y - 12.0 * k),
            model.true_north(),
            k,
        );
        s.push_str("</g>\n");
    }
    if options.viewport.is_none() && options.title {
        // Title, in the technical lettering (no fonts needed).
        let level_name = model.level(level).map(|l| l.name.as_str()).unwrap_or("");
        let h = TEXT_SIZE * k;
        let at = Point2::new(min.x, min.y - margin * 0.6 - h);
        let text = format!(
            "{} · {} · E 1:{}",
            model.project_name, level_name, options.scale
        );
        let _ = writeln!(
            s,
            r#"<path class="title" d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            lettering::text_path(&text, at, Point2::new(1.0, 0.0), h, lettering::Align::Left),
            num(lettering::STROKE * h)
        );
    }
    s.push_str("</svg>\n");
    s
}

/// Colour of the reference level drawn underneath.
const REFERENCE_INK: &str = "#6d8fb5";
/// Fill of the walls of the reference level.
const REFERENCE_FILL: &str = "#dce5ef";

/// The walls of the reference level: filled and outlined in a light
/// grey-blue, with their doors and windows marked, so they read as a
/// tracing underneath and never as walls of this level.
fn draw_reference(s: &mut String, model: &Model, geos: &[WallGeometry], k: f64) {
    let _ = writeln!(
        s,
        r#"<g id="reference" fill="none" stroke="{REFERENCE_INK}" stroke-linejoin="miter">"#
    );
    for g in geos {
        let outline = g.outline_at(None);
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{REFERENCE_FILL}" stroke-width="{}"/>"#,
            path(&outline),
            num(pens().frame * k)
        );
        // Openings of the wall below: where its doors and windows are.
        for o in openings_in_wall(model, g.wall) {
            let (a, b) = (
                o.point(o.t0, g.exterior_offset),
                o.point(o.t0, g.interior_offset),
            );
            let (c, d) = (
                o.point(o.t1, g.exterior_offset),
                o.point(o.t1, g.interior_offset),
            );
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} M{} L{}" stroke-width="{}"/>"#,
                pt(a),
                pt(b),
                pt(c),
                pt(d),
                num(pens().view * k)
            );
        }
    }
    s.push_str("</g>\n");
}

/// How the layers of a wall type are finished at the jambs of a door or
/// window, as usually built in a Spanish double-leaf façade.
#[derive(Debug, Clone, PartialEq)]
struct JambDetail {
    /// The outer leaf that turns at the jamb to close the cavity, and the
    /// cavity layers it closes (they stop short of the jamb).
    turn: Option<(usize, std::ops::Range<usize>)>,
    /// The exterior finish (render) that wraps the reveal up to the frame.
    wrap: Option<usize>,
}

impl JambDetail {
    fn for_type(wtype: &archisim_core::WallType) -> Self {
        use LayerFunction::*;
        let layers = &wtype.layers;
        let is_cavity = |f: LayerFunction| matches!(f, Insulation | AirGap | Membrane);
        // The first leaf (from outside) followed by a cavity with something
        // behind it: that leaf returns across the cavity.
        let turn = (0..layers.len()).find_map(|i| {
            if !matches!(layers[i].function, Structure | Substrate) {
                return None;
            }
            let start = i + 1;
            let mut end = start;
            while end < layers.len() && is_cavity(layers[end].function) {
                end += 1;
            }
            (end > start && end < layers.len()).then_some((i, start..end))
        });
        let wrap = (wtype.function == WallFunction::Exterior
            && layers.len() > 1
            && layers[0].function == Finish)
            .then_some(0);
        Self { turn, wrap }
    }

    fn cut_back(&self, layer: usize) -> bool {
        self.turn
            .as_ref()
            .is_some_and(|(_, cavity)| cavity.contains(&layer))
    }
}

/// Fills a cut polygon with its material colour and hatch. `id` names the
/// fill so each cut piece can be found in the drawing.
fn fill_cut(s: &mut String, mat: &archisim_core::Material, polygon: &[Point2], id: &str, k: f64) {
    let d = path(polygon);
    let [r, gg, bb] = mat.cut_color;
    // A hairline in the fill colour hides anti-aliasing seams at mitres.
    let _ = writeln!(
        s,
        r#"<path id="{id}" d="{d}" fill="rgb({r},{gg},{bb})" stroke="rgb({r},{gg},{bb})" stroke-width="{}"/>"#,
        num(pens().hatch * k)
    );
    // Hatching is real geometry cut to the layer (not SVG patterns or clip
    // paths), so it prints identically everywhere, carries over to PDF/DXF
    // and draws fast on screen.
    let sp = HATCH_SPACING * k;
    let hatch = match mat.cut_pattern {
        CutPattern::Diagonal => hatch_lines(polygon, &[45.0], sp),
        CutPattern::Cross => hatch_lines(polygon, &[45.0, 135.0], sp),
        _ => String::new(),
    };
    if !hatch.is_empty() {
        let _ = writeln!(
            s,
            r#"<path d="{hatch}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            num(pens().hatch * k)
        );
    }
    if mat.cut_pattern == CutPattern::Dots {
        let dots = hatch_dots(polygon, sp, pens().hatch * k * 1.2);
        if !dots.is_empty() {
            let _ = writeln!(s, r#"<path d="{dots}" fill="{INK}"/>"#);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_layers(
    s: &mut String,
    model: &Model,
    g: &WallGeometry,
    gi: usize,
    pieces: &[WallPiece],
    cut: &[OpeningGeometry],
    cut_plane: f64,
    k: f64,
) {
    if g.arc.is_some() {
        draw_curved_layers(s, model, g, gi, k);
        return;
    }
    let (a, b) = g.axis;
    let dir = b - a;
    let len = dir.length();
    let (u, n) = match dir.normalized() {
        Some(u) => (u, u.perp_left()),
        None => return,
    };
    let wall_t = g.exterior_offset - g.interior_offset;
    let Some(wtype) = model
        .wall(g.wall)
        .and_then(|w| model.wall_type(w.wall_type))
    else {
        return;
    };
    if too_small_for_layers(wall_t / k) {
        // Zoomed far out the layers cannot be told apart: the wall is one
        // grey band (its cut lines are still drawn).
        for piece in pieces {
            let poly = clip_span(&g.outline, a, u, piece.from, piece.to);
            if poly.len() >= 3 {
                let _ = writeln!(s, r#"<path d="{}" fill="{FAR_POCHE}"/>"#, path(&poly));
            }
        }
        return;
    }
    let jamb = JambDetail::for_type(wtype);
    let turn_width = jamb
        .turn
        .as_ref()
        .map_or(0.0, |(i, _)| wtype.layers[*i].thickness);

    // Each layer clipped to a piece, stopping short of the jambs if it is
    // inside the cavity that the outer leaf closes. Walls butting into this
    // one may take parts of a layer, leaving it in several bits.
    let bits: Vec<Vec<Vec<Point2>>> = (0..g.layers.len())
        .map(|li| g.layer_pieces(li, Some(cut_plane)))
        .collect();
    let layer_piece = |li: usize, piece: &WallPiece| -> Vec<Vec<Point2>> {
        let back = if jamb.cut_back(li) { turn_width } else { 0.0 };
        bits[li]
            .iter()
            .map(|poly| {
                clip_span(
                    poly,
                    a,
                    u,
                    piece.from.map(|f| f + back),
                    piece.to.map(|t| t - back),
                )
            })
            .filter(|p| !p.is_empty())
            .collect()
    };

    // Offsets of each layer's exterior face.
    let mut tops = Vec::with_capacity(wtype.layers.len() + 1);
    let mut high = g.exterior_offset;
    for layer in &wtype.layers {
        tops.push(high);
        high -= layer.thickness;
    }
    tops.push(high);

    for ((li, band), layer) in g.layers.iter().enumerate().zip(&wtype.layers) {
        let Some(mat) = model.material(band.material) else {
            continue;
        };
        let thickness = layer.thickness;
        let (high, low) = (tops[li], tops[li + 1]);
        let polygons = pieces.iter().flat_map(|piece| layer_piece(li, piece));
        for (pi, polygon) in polygons.enumerate() {
            fill_cut(s, mat, &polygon, &format!("cut-{gi}-{li}-{pi}"), k);

            if mat.cut_pattern == CutPattern::Insulation
                || band.function == LayerFunction::Insulation
            {
                // Zigzag along the layer, overshooting the ends and cut to it.
                let mid = (high + low) / 2.0;
                let amp = thickness / 2.0;
                let step = thickness.max(0.01);
                let mut zigzag = Vec::new();
                let mut t = -wall_t * 2.0;
                let mut up = true;
                while t <= len + wall_t * 2.0 {
                    let off = mid + if up { amp } else { -amp };
                    zigzag.push(a + u * t + n * off);
                    t += step / 2.0;
                    up = !up;
                }
                let zz = clip_polyline(&zigzag, &polygon);
                if !zz.is_empty() {
                    let _ = writeln!(
                        s,
                        r#"<path d="{zz}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                        num(pens().hatch * k)
                    );
                }
            }
        }
    }

    // Thin boundaries between layers, along the faces only. Where a cavity
    // layer stops short of a jamb, the boundary stops with it. At a
    // T-junction the layer that goes further into the other wall draws it.
    let tee_end = matches!(g.start_cut, EndCut::Tee(_)) || matches!(g.end_cut, EndCut::Tee(_));
    for li in 0..wtype.layers.len().saturating_sub(1) {
        let owners: &[usize] = if jamb.cut_back(li + 1) {
            &[li + 1]
        } else if tee_end {
            &[li, li + 1]
        } else {
            &[li]
        };
        for &owner in owners {
            for piece in pieces {
                for polygon in layer_piece(owner, piece) {
                    if let Some((p, q)) = face_edge_at(&polygon, a, n, tops[li + 1]) {
                        line(s, p, q, pens().thin, k);
                    }
                }
            }
        }
    }

    // Jamb details: (jamb position, +1 if the wall continues towards
    // growing t, the opening whose jamb it is).
    let mut jambs = Vec::new();
    for piece in pieces {
        if let Some(t) = piece.from {
            if let Some(o) = cut.iter().find(|o| (o.t1 - t).abs() < 1e-9) {
                jambs.push((t, 1.0, o));
            }
        }
        if let Some(t) = piece.to {
            if let Some(o) = cut.iter().find(|o| (o.t0 - t).abs() < 1e-9) {
                jambs.push((t, -1.0, o));
            }
        }
    }
    let local = |t: f64, off: f64| a + u * t + n * off;
    let rect = |t0: f64, t1: f64, y0: f64, y1: f64| {
        vec![local(t0, y0), local(t1, y0), local(t1, y1), local(t0, y1)]
    };
    for (ji, &(t, side, o)) in jambs.iter().enumerate() {
        // The outer leaf turns across the cavity, as wide as it is thick.
        if let Some((leaf, cavity)) = &jamb.turn {
            if let Some(mat) = model.material(wtype.layers[*leaf].material) {
                let (y_out, y_in) = (tops[*leaf + 1], tops[cavity.end]);
                let back = t + side * turn_width;
                fill_cut(
                    s,
                    mat,
                    &rect(t, back, y_in, y_out),
                    &format!("cut-{gi}-turn-{ji}"),
                    k,
                );
                // Against the cavity and against the inner leaf.
                line(s, local(back, y_out), local(back, y_in), pens().thin, k);
                line(s, local(t, y_in), local(back, y_in), pens().thin, k);
            }
        }
        // The render wraps the reveal from the façade to the frame.
        if let Some(fin) = jamb.wrap {
            let f = wtype.layers[fin].thickness;
            let (y_out, y_in) = (tops[fin + 1], o.frame_exterior);
            if let (Some(mat), true) = (
                model.material(wtype.layers[fin].material),
                y_in < y_out - EPSILON,
            ) {
                let back = t + side * f;
                fill_cut(
                    s,
                    mat,
                    &rect(t, back, y_in, y_out),
                    &format!("cut-{gi}-wrap-{ji}"),
                    k,
                );
                line(s, local(back, y_out), local(back, y_in), pens().thin, k);
                line(s, local(t, y_in), local(back, y_in), pens().thin, k);
            }
        }
    }
}

/// How a slab relates to the cut plane of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlabView {
    Below,
    Cut,
    Above,
}

fn slab_view(model: &Model, slab: &Slab, cut_plane: f64) -> SlabView {
    let thickness = model
        .slab_type(slab.slab_type)
        .map_or(0.0, |t| t.thickness());
    // An inclined slab is cut if the plane crosses it anywhere.
    let (low, high) = slab.top_range();
    let (bottom, top) = (low - slab.vertical(thickness), high);
    if top <= cut_plane {
        SlabView::Below
    } else if bottom < cut_plane {
        SlabView::Cut
    } else {
        SlabView::Above
    }
}

fn draw_slab(s: &mut String, model: &Model, slab: &Slab, si: usize, cut_plane: f64, k: f64) {
    // The outline and, as more rings of the same path, its holes: filled
    // even-odd, the holes stay empty.
    let mut d = path(&slab.outline);
    for h in &slab.holes {
        d.push(' ');
        d.push_str(&path(h));
    }
    // A cross in each hole (the usual mark of a hole through a floor).
    let crosses = |s: &mut String, dashed: bool| {
        for h in &slab.holes {
            let (lo, hi) = bounds(h);
            let mut x = String::new();
            for (a, b) in [(lo, hi), (Point2::new(lo.x, hi.y), Point2::new(hi.x, lo.y))] {
                for (p, q) in clip_segment(a, b, h) {
                    let _ = write!(x, "M{} L{} ", pt(p), pt(q));
                }
            }
            let dash = if dashed {
                format!(r#" stroke-dasharray="{} {}""#, num(3.0 * k), num(1.5 * k))
            } else {
                String::new()
            };
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"{dash}/>"#,
                x.trim_end(),
                num(pens().hatch * k)
            );
        }
    };
    match slab_view(model, slab, cut_plane) {
        SlabView::Below => {
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="{SLAB_COVER}" fill-rule="evenodd" stroke="{INK}" stroke-width="{}" stroke-linejoin="round"/>"#,
                num(pens().view * k)
            );
            crosses(s, false);
        }
        SlabView::Above => {
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
                num(pens().view * k),
                num(3.0 * k),
                num(1.5 * k)
            );
            crosses(s, true);
        }
        SlabView::Cut => {
            // Filled with the structural layer (or the thickest one).
            let mat = model.slab_type(slab.slab_type).and_then(|t| {
                t.layers
                    .iter()
                    .find(|l| l.function == LayerFunction::Structure)
                    .or_else(|| {
                        t.layers
                            .iter()
                            .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
                    })
                    .and_then(|l| model.material(l.material))
            });
            let (fill, pattern) = mat.map_or(([255, 255, 255], CutPattern::Solid), |m| {
                (m.cut_color, m.cut_pattern)
            });
            let [r, g, b] = fill;
            let _ = writeln!(
                s,
                r#"<path id="cut-slab-{si}" d="{d}" fill="rgb({r},{g},{b})" fill-rule="evenodd"/>"#
            );
            let angles: &[f64] = match pattern {
                CutPattern::Diagonal => &[45.0],
                CutPattern::Cross => &[45.0, 135.0],
                _ => &[],
            };
            let area = archisim_geom::solid::with_holes(&slab.outline, &slab.holes);
            let hatch = hatch_lines(&area, angles, HATCH_SPACING * k);
            if !hatch.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{hatch}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                    num(pens().hatch * k)
                );
            }
            let _ = writeln!(
                s,
                r#"<path d="{d}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                num(pens().cut * k)
            );
            crosses(s, false);
        }
    }
    if let Some(slope) = &slab.slope {
        draw_slope_arrow(s, slab, slope, k);
    }
}

/// The slope arrow of an inclined slab: a thin line from its tail to its
/// head with a filled arrowhead (uphill), and the slope in per cent.
fn draw_slope_arrow(s: &mut String, slab: &Slab, slope: &archisim_core::SlabSlope, k: f64) {
    let Some(u) = slope.direction() else {
        return;
    };
    // Along the arrow's line, as far as the slab goes.
    let (a, b) = (slope.from, slope.to);
    let pieces = clip_segment(a - u * 1e3, b + u * 1e3, &slab.outline);
    let (a, b) = pieces
        .iter()
        .copied()
        .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
        .unwrap_or((a, b));
    let (a, b) = if (b - a).dot(u) * slope.slope >= 0.0 {
        (a, b)
    } else {
        (b, a)
    };
    let inset = 1.5 * k;
    let dir = (b - a).normalized().unwrap_or(u);
    let (a, b) = (a + dir * inset, b - dir * inset);
    let head = 2.5 * k;
    let n = dir.perp_left();
    let base = b - dir * head;
    let _ = writeln!(
        s,
        r#"<g class="slope"><path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/><path d="M{} L{} L{} Z" fill="{INK}" stroke="none"/>"#,
        pt(a),
        pt(base),
        num(pens().view * k),
        pt(b),
        pt(base + n * (head * 0.35)),
        pt(base - n * (head * 0.35))
    );
    let text = format!("{} %", fmt_percent(slope.slope.abs() * 100.0));
    let th = annotate::TEXT_HEIGHT * k;
    // Read from below or from the right, as dimension texts.
    let read = if dir.x < -1e-9 || (dir.x.abs() <= 1e-9 && dir.y < 0.0) {
        dir * -1.0
    } else {
        dir
    };
    let mid = a + (b - a) * 0.5 + read.perp_left() * (1.0 * k);
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/></g>"#,
        lettering::text_path(&text, mid, read, th, lettering::Align::Centre),
        num(lettering::STROKE * th)
    );
}

/// A percentage with up to one decimal and a decimal comma: 8, 12,5.
fn fmt_percent(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}").replace('.', ",")
    }
}

/// Lowest and highest corner of the box around some points.
fn bounds(pts: &[Point2]) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

/// Parallel hatch lines cut to a polygon, anchored to the project origin so
/// that hatching continues seamlessly across elements. Angles are in
/// degrees, measured counter-clockwise from the X axis.
fn hatch_lines(poly: &[Point2], angles: &[f64], spacing: f64) -> String {
    if coarse() {
        return String::new();
    }
    let mut d = String::new();
    for &deg in angles {
        let rad = deg.to_radians();
        let dir = Point2::new(rad.cos(), rad.sin());
        let m = dir.perp_left();
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut t0, mut t1) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in poly {
            lo = lo.min(p.dot(m));
            hi = hi.max(p.dot(m));
            t0 = t0.min(p.dot(dir));
            t1 = t1.max(p.dot(dir));
        }
        let first = (lo / spacing).ceil() as i64;
        let last = (hi / spacing).floor() as i64;
        for i in first..=last {
            let c = i as f64 * spacing;
            let p = m * c + dir * (t0 - spacing);
            let q = m * c + dir * (t1 + spacing);
            for (a, b) in clip_segment(p, q, poly) {
                let _ = write!(d, "M{} L{} ", pt(a), pt(b));
            }
        }
    }
    d.trim_end().to_string()
}

/// A grid of small dots inside a polygon, anchored to the project origin.
fn hatch_dots(poly: &[Point2], spacing: f64, radius: f64) -> String {
    if coarse() {
        return String::new();
    }
    let (mut min, mut max) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in poly {
        min = Point2::new(min.x.min(p.x), min.y.min(p.y));
        max = Point2::new(max.x.max(p.x), max.y.max(p.y));
    }
    let mut d = String::new();
    let (i0, i1) = (
        (min.x / spacing).ceil() as i64,
        (max.x / spacing).floor() as i64,
    );
    let (j0, j1) = (
        (min.y / spacing).ceil() as i64,
        (max.y / spacing).floor() as i64,
    );
    for i in i0..=i1 {
        for j in j0..=j1 {
            // Stagger alternate rows for a more even texture.
            let x = (i as f64 + if j % 2 == 0 { 0.0 } else { 0.5 }) * spacing;
            let c = Point2::new(x, j as f64 * spacing);
            // Only whole dots: none sticks out of the layer.
            if contains(poly, c) && distance_to_outline(poly, c) >= radius {
                let _ = write!(
                    d,
                    "M{} a{r},{r} 0 1,0 {dd},0 a{r},{r} 0 1,0 -{dd},0 ",
                    pt(c - Point2::new(radius, 0.0)),
                    r = num(radius),
                    dd = num(2.0 * radius)
                );
            }
        }
    }
    d.trim_end().to_string()
}

/// The parts of segment `p`–`q` inside a simple polygon (any shape).
fn clip_segment(p: Point2, q: Point2, poly: &[Point2]) -> Vec<(Point2, Point2)> {
    let d = q - p;
    let n = poly.len();
    let mut ts = vec![0.0, 1.0];
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let e = b - a;
        let den = d.cross(e);
        if den.abs() < 1e-15 {
            continue;
        }
        let t = (a - p).cross(e) / den;
        let s = (a - p).cross(d) / den;
        if (-1e-12..=1.0 + 1e-12).contains(&s) && t > 0.0 && t < 1.0 {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    let mut out: Vec<(f64, f64)> = Vec::new();
    for w in ts.windows(2) {
        let (t0, t1) = (w[0], w[1]);
        if (t1 - t0) * d.length() < 1e-9 || !contains(poly, p + d * ((t0 + t1) / 2.0)) {
            continue;
        }
        match out.last_mut() {
            Some(last) if (last.1 - t0).abs() < 1e-12 => last.1 = t1,
            _ => out.push((t0, t1)),
        }
    }
    out.into_iter()
        .map(|(a, b)| (p + d * a, p + d * b))
        .collect()
}

/// A polyline cut to a polygon, as SVG path data: each run inside the
/// polygon stays one continuous line so its corners keep their joins.
fn clip_polyline(points: &[Point2], poly: &[Point2]) -> String {
    if coarse() {
        return String::new();
    }
    let mut d = String::new();
    let mut pen: Option<Point2> = None;
    for w in points.windows(2) {
        for (a, b) in clip_segment(w[0], w[1], poly) {
            match pen {
                Some(at) if at.approx_eq(a) => {}
                _ => {
                    let _ = write!(d, "M{} ", pt(a));
                }
            }
            let _ = write!(d, "L{} ", pt(b));
            pen = Some(b);
        }
    }
    d.trim_end().to_string()
}

/// Distance from a point to the outline of a polygon.
fn distance_to_outline(poly: &[Point2], p: Point2) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let e = b - a;
            let len2 = e.dot(e);
            let t = if len2 < 1e-18 {
                0.0
            } else {
                ((p - a).dot(e) / len2).clamp(0.0, 1.0)
            };
            (a + e * t).distance(p)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Point-in-polygon test (even-odd rule).
fn contains(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// The edge of a layer polygon lying on the given offset from the axis.
fn face_edge_at(
    poly: &[Point2],
    origin: Point2,
    n: Point2,
    offset: f64,
) -> Option<(Point2, Point2)> {
    let on = |p: Point2| ((p - origin).dot(n) - offset).abs() < 1e-6;
    (0..poly.len()).find_map(|i| {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        (on(p) && on(q)).then_some((p, q))
    })
}

fn draw_cut_lines(s: &mut String, g: &WallGeometry, pieces: &[WallPiece], cut_plane: f64, k: f64) {
    if let Some(arc) = &g.arc {
        draw_curved_cut_lines(s, g, arc, k);
        return;
    }
    let (a, b) = g.axis;
    let Some(u) = (b - a).normalized() else {
        return;
    };
    let n = u.perp_left();
    let len = a.distance(b);
    let t = |p: Point2| (p - a).dot(u);
    let off = |p: Point2| (p - a).dot(n);
    let near = |x: f64, y: f64| (x - y).abs() < 1e-6;

    let outline = g.outline_at(Some(cut_plane));
    // Stretches of each face where another wall butts in: the face is not
    // cut there, the walls join. A thin line marks where the layers of the
    // other wall stop at the face; where they go in, nothing.
    type Butt = (f64, f64, f64, Vec<(f64, f64)>);
    let butts: Vec<Butt> = g
        .notches
        .iter()
        .filter(|nt| cut_plane > nt.z.0 && cut_plane < nt.z.1)
        .filter_map(|nt| {
            let face = archisim_geom::Line::new(a + n * nt.face_offset, u);
            let at = |l: &archisim_geom::Line| face.intersect(l).map(t);
            let ts: Vec<f64> = nt.boundaries.iter().filter_map(at).collect();
            if ts.len() != nt.boundaries.len() || ts.len() < 2 {
                return None;
            }
            let stops: Vec<(f64, f64)> = (0..ts.len() - 1)
                .filter(|&j| {
                    !nt.regions
                        .iter()
                        .any(|r| r.sides == (nt.boundaries[j], nt.boundaries[j + 1]))
                })
                .map(|j| (ts[j].min(ts[j + 1]), ts[j].max(ts[j + 1])))
                .collect();
            let (lo, hi) = ts
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), x| {
                    (l.min(*x), h.max(*x))
                });
            Some((nt.face_offset, lo, hi, stops))
        })
        .collect();
    let mut thin = String::new();
    let mut d = String::new();
    for piece in pieces {
        let poly = clip_span(&outline, a, u, piece.from, piece.to);
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let on_face = [g.exterior_offset, g.interior_offset]
                .iter()
                .any(|&f| near(off(p), f) && near(off(q), f));
            let on_jamb = [piece.from, piece.to]
                .iter()
                .flatten()
                .any(|&j| near(t(p), j) && near(t(q), j));
            // A free end is cut; a mitre or a T-junction joins another wall,
            // and the bits other walls take out of this one are joins too.
            let on_start = near(t(p), 0.0) && near(t(q), 0.0);
            let on_end = near(t(p), len) && near(t(q), len);
            let heavy = on_face
                || on_jamb
                || (on_start && g.start_cut == EndCut::Square)
                || (on_end && g.end_cut == EndCut::Square);
            if !heavy {
                continue;
            }
            // Leave out where another wall butts against this face.
            let mut spans = vec![(t(p).min(t(q)), t(p).max(t(q)))];
            if on_face {
                for (face, b0, b1, stops) in &butts {
                    let (face, b0, b1) = (*face, *b0, *b1);
                    if !near(off(p), face) {
                        continue;
                    }
                    spans = spans
                        .into_iter()
                        .flat_map(|(s0, s1)| {
                            if b1 <= s0 || b0 >= s1 {
                                vec![(s0, s1)]
                            } else {
                                for (x0, x1) in stops {
                                    let (x0, x1) = (x0.max(s0), x1.min(s1));
                                    if x1 > x0 {
                                        let _ = write!(
                                            thin,
                                            "M{} L{} ",
                                            pt(a + u * x0 + n * face),
                                            pt(a + u * x1 + n * face)
                                        );
                                    }
                                }
                                [(s0, b0), (b1, s1)]
                                    .into_iter()
                                    .filter(|(x, y)| y - x > 1e-6)
                                    .collect()
                            }
                        })
                        .collect();
                }
            }
            let o = off(p);
            for (s0, s1) in spans {
                let (x, y) = if on_face {
                    (a + u * s0 + n * o, a + u * s1 + n * o)
                } else {
                    (p, q)
                };
                let _ = write!(d, "M{} L{} ", pt(x), pt(y));
            }
        }
    }
    if !thin.is_empty() {
        let _ = writeln!(
            s,
            r#"<path d="{}" stroke="{INK}" stroke-width="{}"/>"#,
            thin.trim_end(),
            num(pens().thin * k)
        );
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{INK}" stroke-width="{}"/>"#,
        d.trim_end(),
        num(pens().cut * k)
    );
}

/// How a door or window relates to the cut plane of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpeningView {
    Below,
    Cut,
    Above,
}

fn opening_view(o: &OpeningGeometry, cut_plane: f64) -> OpeningView {
    if o.head <= cut_plane + EPSILON {
        OpeningView::Below
    } else if o.sill < cut_plane - EPSILON {
        OpeningView::Cut
    } else {
        OpeningView::Above
    }
}

fn line(s: &mut String, p: Point2, q: Point2, weight: f64, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        pt(p),
        pt(q),
        num(weight * k)
    );
}

/// Objects of a level (furniture, sanitary ware…): the plan symbol of
/// their family for the drawing scale, turned and placed. Too small to
/// see: just their box.
fn draw_objects(
    s: &mut String,
    model: &Model,
    level: archisim_core::LevelId,
    scale: f64,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    s.push_str("<g id=\"objects\">\n");
    for o in model.objects_on_level(level) {
        let outline = model.object_outline(o.id);
        if outline.is_empty() || !visible(&outline) {
            continue;
        }
        let size = outline[0].distance(outline[2]);
        let marks = if too_small_for_symbol(size * 1000.0 / scale) {
            Err(())
        } else {
            model.object_plan_symbol(o.id, scale).map_err(|_| ())
        };
        dim::element(s, o.id.0, |s| match marks {
            Ok(marks) => draw_marks(s, marks, &|p| o.to_plan(p[0], p[1]), k),
            Err(()) => {
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&outline),
                    num(pens().view * k)
                );
            }
        });
    }
    s.push_str("</g>\n");
}

/// A door or window crossed by the cut plane: the plan symbol of its
/// family for the drawing scale.
///
/// The family draws in its own axes: `x` along the wall from the centre of
/// the opening, to the right seen from the side the leaves open to; `y`
/// across the wall from the middle of the frame, towards that side.
fn draw_opening(s: &mut String, model: &Model, o: &OpeningGeometry, scale: f64, k: f64) {
    if too_small_for_symbol((o.t1 - o.t0) * 1000.0 / scale) {
        // Zoomed far out: the gap in the wall says enough.
        return;
    }
    let mid = (o.t0 + o.t1) / 2.0;
    let y_mid = (o.frame_exterior + o.frame_interior) / 2.0;
    // The normal `n` points to the exterior. Seen from the exterior, the
    // wall's direction runs to the left; seen from the interior, to the
    // right.
    let (sx, sy) = match o.opens_to {
        WallSide::Exterior => (-1.0, 1.0),
        WallSide::Interior => (1.0, -1.0),
    };
    let family_y = |offset: f64| sy * (offset - y_mid);
    let Ok(marks) = model.opening_plan_symbol(
        o.opening,
        scale,
        family_y(o.exterior_offset),
        family_y(o.interior_offset),
    ) else {
        // A family that fails to draw (it should not: its values were
        // checked) leaves the opening empty rather than the plan.
        return;
    };
    let world = |p: [f64; 2]| o.point(mid + sx * p[0], y_mid + sy * p[1]);
    draw_marks(s, marks, &world, k);
}

/// The marks of a family's plan symbol, through `world` (family axes to
/// plan).
fn draw_marks(s: &mut String, marks: Vec<Mark>, world: &dyn Fn([f64; 2]) -> Point2, k: f64) {
    for mark in marks {
        match mark {
            Mark::Line {
                from,
                to,
                pen,
                style,
            } => {
                let dash = dash_attr(style, k);
                let _ = writeln!(
                    s,
                    r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"{dash}/>"#,
                    pt(world(from)),
                    pt(world(to)),
                    num(pen_weight(pen) * k)
                );
            }
            Mark::Polygon {
                points,
                pen,
                style,
                filled,
            } => {
                let pts: Vec<Point2> = points.into_iter().map(world).collect();
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="{}" stroke="{INK}" stroke-width="{}"{}/>"#,
                    path(&pts),
                    if filled { "white" } else { "none" },
                    num(pen_weight(pen) * k),
                    dash_attr(style, k)
                );
            }
            Mark::Arc {
                center,
                from,
                to,
                pen,
                style,
            } => {
                let (c, a, b) = (world(center), world(from), world(to));
                let r = (a - c).length();
                // Model counter-clockwise is screen counter-clockwise
                // (sweep 0); always the short way round.
                let sweep = if (a - c).cross(b - c) > 0.0 { 0 } else { 1 };
                let _ = writeln!(
                    s,
                    r#"<path d="M{} A{r},{r} 0 0,{sweep} {}" fill="none" stroke="{INK}" stroke-width="{}"{}/>"#,
                    pt(a),
                    pt(b),
                    num(pen_weight(pen) * k),
                    dash_attr(style, k),
                    r = num(r)
                );
            }
        }
    }
}

/// The SVG dash attribute of a line style, at the drawing scale (`k`
/// metres per paper millimetre); empty for a continuous line. Dots are
/// drawn with round caps so they read as dots.
pub(crate) fn dash_attr(style: LineStyle, k: f64) -> String {
    let pattern = style.pattern_mm();
    if pattern.is_empty() {
        return String::new();
    }
    let values: Vec<String> = pattern.iter().map(|v| num(v * k)).collect();
    let cap = if matches!(
        style,
        LineStyle::Dotted | LineStyle::DashDot | LineStyle::DashDotDot
    ) {
        r#" stroke-linecap="round""#
    } else {
        ""
    };
    format!(r#" stroke-dasharray="{}"{cap}"#, values.join(" "))
}

/// Paper line weight of a family pen, in millimetres.
fn pen_weight(pen: Pen) -> f64 {
    match pen {
        Pen::Cut => pens().cut,
        Pen::Frame => pens().frame,
        Pen::View => pens().view,
        Pen::Thin => pens().thin,
        Pen::Hatch => pens().hatch,
    }
}

/// A window above the cut plane: its outline, dashed, over the wall.
fn draw_opening_above(s: &mut String, o: &OpeningGeometry, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
        path(&o.footprint()),
        num(pens().view * k),
        num(2.0 * k),
        num(1.0 * k)
    );
    let mid = (o.frame_exterior + o.frame_interior) / 2.0;
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
        pt(o.point(o.t0, mid)),
        pt(o.point(o.t1, mid)),
        num(pens().thin * k),
        num(2.0 * k),
        num(1.0 * k)
    );
}

/// A number for the SVG, to the micrometre and without trailing zeros
/// (`1.5`, `-0.25`, `3`): the same text as Rust's shortest form of the
/// rounded value, written without going through floating point formatting
/// (a plan has millions of them).
pub(crate) fn num(v: f64) -> String {
    let n = (v * 1e6).round();
    if n == 0.0 || !n.is_finite() {
        return "0".into();
    }
    if n.abs() >= 1e15 {
        return format!("{}", n / 1e6);
    }
    let neg = n < 0.0;
    let n = n.abs() as u64;
    let (mut int, mut frac) = (n / 1_000_000, n % 1_000_000);
    // Digits from the right into a small buffer.
    let mut buf = [0u8; 32];
    let mut i = buf.len();
    if frac > 0 {
        let mut digits = 6;
        while frac % 10 == 0 {
            frac /= 10;
            digits -= 1;
        }
        for _ in 0..digits {
            i -= 1;
            buf[i] = b'0' + (frac % 10) as u8;
            frac /= 10;
        }
        i -= 1;
        buf[i] = b'.';
    }
    loop {
        i -= 1;
        buf[i] = b'0' + (int % 10) as u8;
        int /= 10;
        if int == 0 {
            break;
        }
    }
    if neg {
        i -= 1;
        buf[i] = b'-';
    }
    String::from_utf8_lossy(&buf[i..]).into_owned()
}

/// Runs of consecutive edges of a closed polygon whose two ends satisfy
/// `on`, as open point chains.
fn runs_on(poly: &[Point2], on: impl Fn(Point2) -> bool) -> Vec<Vec<Point2>> {
    let n = poly.len();
    let edge = |i: usize| on(poly[i]) && on(poly[(i + 1) % n]);
    // Start after an edge that is not on it, so no run is split in two.
    let Some(first) = (0..n).find(|&i| !edge(i)) else {
        let mut all = poly.to_vec();
        all.extend(poly.first().copied());
        return vec![all];
    };
    let mut runs: Vec<Vec<Point2>> = Vec::new();
    let mut current: Vec<Point2> = Vec::new();
    for k in 1..=n {
        let i = (first + k) % n;
        if edge(i) {
            if current.is_empty() {
                current.push(poly[i]);
            }
            current.push(poly[(i + 1) % n]);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

/// The layers of a curved wall: each band filled with its material, the
/// insulation zigzag along the curve and thin lines between layers.
fn draw_curved_layers(s: &mut String, model: &Model, g: &WallGeometry, gi: usize, k: f64) {
    let (Some(arc), Some(wtype)) = (
        g.arc.as_ref(),
        model
            .wall(g.wall)
            .and_then(|w| model.wall_type(w.wall_type)),
    ) else {
        return;
    };
    let mut tops = vec![g.exterior_offset];
    for layer in &wtype.layers {
        let last = tops[tops.len() - 1];
        tops.push(last - layer.thickness);
    }
    for ((li, band), layer) in g.layers.iter().enumerate().zip(&wtype.layers) {
        let Some(mat) = model.material(band.material) else {
            continue;
        };
        fill_cut(s, mat, &band.polygon, &format!("cut-{gi}-{li}-0"), k);
        if mat.cut_pattern == CutPattern::Insulation || band.function == LayerFunction::Insulation {
            let t = layer.thickness;
            let mid = (tops[li] + tops[li + 1]) / 2.0;
            let len = arc.radius_at(mid).abs() * arc.sweep.abs();
            let df = (t.max(0.01) / 2.0) / len.max(EPSILON);
            let over = 2.0 * (g.exterior_offset - g.interior_offset).abs() / len.max(EPSILON);
            let mut zigzag = Vec::new();
            let mut f = -over;
            let mut up = true;
            while f <= 1.0 + over {
                zigzag.push(arc.point(f, mid + if up { t / 2.0 } else { -t / 2.0 }));
                f += df;
                up = !up;
            }
            let zz = clip_polyline(&zigzag, &band.polygon);
            if !zz.is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path d="{zz}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"/>"#,
                    num(pens().hatch * k)
                );
            }
        }
    }
    for li in 0..g.layers.len().saturating_sub(1) {
        let off = tops[li + 1];
        for run in runs_on(&g.layers[li].polygon, |p| {
            (arc.project(p).1 - off).abs() < 1e-6
        }) {
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
                open_path(&run),
                num(pens().thin * k)
            );
        }
    }
}

/// The cut lines of a curved wall: both faces, and the ends that are not
/// joined to another wall.
fn draw_curved_cut_lines(s: &mut String, g: &WallGeometry, arc: &archisim_core::Arc, k: f64) {
    let mut d = String::new();
    let off = |p: Point2| arc.project(p).1;
    let near = |x: f64, y: f64| (x - y).abs() < 1e-6;
    for face in [g.exterior_offset, g.interior_offset] {
        for run in runs_on(&g.outline, |p| near(off(p), face)) {
            d.push_str(&open_path(&run));
            d.push(' ');
        }
    }
    let ring = &g.outline;
    for i in 0..ring.len() {
        let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
        if near(off(p), off(q)) {
            continue;
        }
        let at_start = arc.project(p).0 + arc.project(q).0 < 1.0;
        let cut = if at_start { &g.start_cut } else { &g.end_cut };
        if *cut == EndCut::Square {
            let _ = write!(d, "M{} L{} ", pt(p), pt(q));
        }
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{INK}" stroke-width="{}" stroke-linejoin="round"/>"#,
        d.trim_end(),
        num(pens().cut * k)
    );
}

/// An open polyline as SVG path data.
fn open_path(pts: &[Point2]) -> String {
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
    }
    d.trim_end().to_string()
}

pub(crate) fn pt(p: Point2) -> String {
    format!("{},{}", num(p.x), num(-p.y))
}

pub(crate) fn path(poly: &[Point2]) -> String {
    let mut d = String::new();
    for (i, p) in poly.iter().enumerate() {
        let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
    }
    d.push('Z');
    d
}

#[cfg(test)]
mod legacy_symbols;

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern as Pattern, DoorOperation, Hand, NewOpening, NewOpeningType, OpeningKind,
        WindowOperation,
    };

    #[test]
    fn hatching_is_cut_to_any_polygon() {
        let p = |x, y| Point2::new(x, y);
        // An L: the line y = 1 crosses the lower leg, the gap and nothing else.
        let l = [
            p(0.0, 0.0),
            p(4.0, 0.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        assert_eq!(
            clip_segment(p(-1.0, 1.0), p(5.0, 1.0), &l),
            vec![(p(0.0, 1.0), p(4.0, 1.0))]
        );
        // The line y = 3 crosses the upper leg only.
        assert_eq!(
            clip_segment(p(-1.0, 3.0), p(5.0, 3.0), &l),
            vec![(p(0.0, 3.0), p(2.0, 3.0))]
        );
        // Through the notch of a U: two pieces.
        let u = [
            p(0.0, 0.0),
            p(6.0, 0.0),
            p(6.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 4.0),
            p(0.0, 4.0),
        ];
        assert_eq!(clip_segment(p(-1.0, 3.0), p(7.0, 3.0), &u).len(), 2);
        // A zigzag cut to a band stays one continuous line inside it.
        let band = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 1.0), p(0.0, 1.0)];
        let zz = [
            p(-1.0, 0.2),
            p(0.5, 0.8),
            p(1.0, 0.2),
            p(1.5, 0.8),
            p(3.0, 0.2),
        ];
        let d = clip_polyline(&zz, &band);
        assert_eq!(d.matches('M').count(), 1, "{d}");
        assert!(hatch_lines(&band, &[45.0], 0.3).split('M').count() > 3);
    }
    use archisim_core::{Layer, NewSlab, NewWall, ReferenceLine, SlabFunction, WallFunction};

    /// Curved walls: their layers follow the arc, the insulation zigzags
    /// along it, the faces are cut lines and a partition stops at the
    /// inner face.
    #[test]
    fn curved_walls_in_plan() {
        let mut m = Model::new("Curvos");
        let pb = m.add_level("Planta baja", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [230, 200, 190], CutPattern::Diagonal);
        let wool = m.add_material(
            "Lana",
            "insulation",
            [250, 250, 210],
            CutPattern::Insulation,
        );
        let plaster = m.add_material("Yeso", "plaster", [240, 240, 240], CutPattern::Solid);
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let facade = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    layer(brick, 0.115, LayerFunction::Structure),
                    layer(wool, 0.05, LayerFunction::Insulation),
                    layer(brick, 0.07, LayerFunction::Substrate),
                    layer(plaster, 0.015, LayerFunction::Finish),
                ],
            )
            .unwrap();
        let partition = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![layer(brick, 0.07, LayerFunction::Structure)],
            )
            .unwrap();
        let mut wall = |t, a: (f64, f64), b: (f64, f64), bulge| {
            m.add_wall(NewWall {
                wall_type: t,
                level: pb,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap()
        };
        // A round room (two half circles, clockwise) and a partition across.
        wall(facade, (3.0, 0.0), (-3.0, 0.0), -1.0);
        wall(facade, (-3.0, 0.0), (3.0, 0.0), -1.0);
        wall(partition, (-2.8, 0.5), (2.8, 0.5), 0.0);
        let svg = render_plan(&m, pb, &PlanOptions::default());
        if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(format!("{dir}/curvos.svg"), &svg);
        }
        // Four layers per half, filled, with the zigzag of the insulation.
        assert_eq!(svg.matches("id=\"cut-").count() - 1, 9); // and the cut lines
        assert!(svg.contains("stroke-linejoin=\"miter\""));
        // The partition ends on the inner face (radius 2.765) of both
        // halves: its cut corners are there.
        let g = archisim_geom::wall_geometry(&m, m.walls().nth(2).unwrap().id).unwrap();
        let r_in = 3.0 - 0.25;
        for p in &g.outline {
            let r = p.distance(Point2::new(0.0, 0.0));
            assert!((r - r_in).abs() < 1e-9, "{r}");
        }
    }

    /// The plan of a level shows the walls of other levels its cut plane
    /// crosses, drawn cut, and the level below in grey when asked.
    #[test]
    fn other_levels_in_plan() {
        let mut m = Model::new("Dos plantas");
        let pb = m.add_level("Planta baja", 0.0);
        let p1 = m.add_level("Planta primera", 3.0);
        let mat = m.add_material("Ladrillo", "brick", [230, 200, 190], CutPattern::Diagonal);
        let t = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.24,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = |m: &mut Model, x: f64, height: f64| {
            m.add_wall(NewWall {
                wall_type: t,
                level: pb,
                start: Point2::new(x, 0.0),
                end: Point2::new(x, 4.0),
                reference: ReferenceLine::Center,
                height,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap()
        };
        let low = wall(&mut m, 0.0, 3.0);
        let tall = wall(&mut m, 5.0, 6.0);
        assert_eq!(level_below(&m, p1), Some(pb));
        assert_eq!(level_below(&m, pb), None);

        // The tall wall crosses the first floor's plane (4.1 m), 4.1 m
        // above its own level; the low one ends under it.
        let o = other_level_walls(&m, p1, 1.1, None);
        assert_eq!(o.cut, vec![(tall, 4.1)]);
        assert!(o.reference.is_empty());
        let o = other_level_walls(&m, p1, 1.1, Some(pb));
        assert_eq!(o.reference, vec![low]);
        assert_eq!(o.ids(), vec![tall, low]);
        // Seen from the ground floor, the first floor has nothing.
        assert_eq!(
            other_level_walls(&m, pb, 1.1, Some(p1)),
            OtherWalls::default()
        );

        // Drawn: the cut wall with its poché and cut lines, the low one only
        // with the reference.
        let plain = render_plan(&m, p1, &PlanOptions::default());
        assert!(plain.contains("<g id=\"poche\">\n<path"), "{plain}");
        assert!(!plain.contains("id=\"reference\""));
        let traced = render_plan(
            &m,
            p1,
            &PlanOptions {
                reference_level: Some(pb),
                ..PlanOptions::default()
            },
        );
        assert!(traced.contains("id=\"reference\""));
        assert!(traced.contains(REFERENCE_INK));
        // The ground floor plan is as before: its two walls, no reference.
        let ground = render_plan(&m, pb, &PlanOptions::default());
        assert!(!ground.contains("id=\"reference\""));
    }

    #[test]
    fn renders_scaled_svg() {
        let mut m = Model::new("Casa <A&B>");
        let lvl = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [230, 200, 190], CutPattern::Diagonal);
        let ins = m.add_material(
            "Lana mineral",
            "insulation",
            [250, 245, 200],
            CutPattern::Insulation,
        );
        let t = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    Layer {
                        material: mat,
                        thickness: 0.115,
                        function: LayerFunction::Structure,
                    },
                    Layer {
                        material: ins,
                        thickness: 0.05,
                        function: LayerFunction::Insulation,
                    },
                ],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: lvl,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("id=\"cut-0-0-0\""));
        assert!(!svg.contains("clip-path="), "hatching is real geometry");
        // The title is drawn with the lettering: no text, no fonts.
        assert!(svg.contains("class=\"title\""));
        assert!(!svg.contains("<text") && !svg.contains("<A&B>"));
        assert!(!svg.contains(" A0.8"), "no door yet");
        // 5 m at 1:50 = 100 mm plus two 15 mm margins.
        assert!(svg.contains("width=\"130.00mm\""), "{}", &svg[..200]);

        // An on-screen view draws exactly the requested region, with no
        // title or paper background, and the same poché.
        let screen = render_plan(
            &m,
            lvl,
            &PlanOptions {
                viewport: Some((Point2::new(-1.0, -2.0), Point2::new(3.0, 1.0))),
                ..PlanOptions::default()
            },
        );
        assert!(
            screen.contains("viewBox=\"-1 -1 4 3\""),
            "{}",
            &screen[..200]
        );
        assert!(screen.contains("width=\"80.00mm\" height=\"60.00mm\""));
        assert!(!screen.contains("<text"));
        assert!(!screen.contains("fill=\"white\""));
        assert!(screen.contains("id=\"cut-0-0-0\""));

        // A door cuts the wall in two pieces and draws its swing; a high
        // window above the cut plane is dashed and does not cut the wall.
        let wood = m.add_material("Madera", "wood", [180, 140, 90], CutPattern::Solid);
        let mut kind = |kind, width, height| {
            m.add_opening_type(archisim_core::NewOpeningType {
                name: "T".into(),
                kind,
                width,
                height,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap()
        };
        let door = kind(OpeningKind::Door(DoorOperation::SingleSwing), 0.9, 2.1);
        let high = kind(OpeningKind::Window(WindowOperation::Fixed), 0.6, 0.6);
        let wall = m.walls().next().unwrap().id;
        for (t, along, sill) in [(door, 1.5, 0.0), (high, 3.5, 1.8)] {
            m.add_opening(archisim_core::NewOpening {
                opening_type: t,
                wall,
                along,
                sill,
                opens_to: archisim_core::WallSide::Interior,
                hand: archisim_core::Hand::Left,
                setback: None,
            })
            .unwrap();
        }
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // Leaf width 0.9 − 2 × 0.05 = 0.8: one swing arc of that radius.
        assert_eq!(svg.matches(" A0.8,0.8 ").count(), 1);
        // Two pieces per layer instead of one.
        assert!(svg.contains("cut-0-1-1"));
        assert!(!svg.contains("cut-0-1-2"));
        assert_eq!(svg.matches("stroke-dasharray").count(), 2);

        // On screen, zoomed far out: the wall is one grey band per piece,
        // without layers, hatches or the door's symbol; close up, as on
        // paper.
        let view = Some((Point2::new(-1.0, -2.0), Point2::new(6.0, 1.0)));
        let far = render_plan(
            &m,
            lvl,
            &PlanOptions {
                viewport: view,
                px_per_mm: Some(0.2),
                ..PlanOptions::default()
            },
        );
        assert_eq!(far.matches(FAR_POCHE).count(), 2);
        assert!(!far.contains("cut-0-0-0"));
        assert_eq!(far.matches(" A0.8,0.8 ").count(), 0);
        let near = |px| {
            render_plan(
                &m,
                lvl,
                &PlanOptions {
                    viewport: view,
                    px_per_mm: px,
                    ..PlanOptions::default()
                },
            )
        };
        assert_eq!(near(Some(20.0)), near(None));
    }

    #[test]
    fn facade_jambs_return_and_casements_swing() {
        let mut m = Model::new("Jambas");
        let lvl = m.add_level("PB", 0.0);
        let mat = |m: &mut Model, name: &str, pattern| {
            m.add_material(name, "x", [200, 200, 200], pattern)
        };
        let render = mat(&mut m, "Monocapa", CutPattern::Dots);
        let brick = mat(&mut m, "Ladrillo", CutPattern::Diagonal);
        let wool = mat(&mut m, "Lana", CutPattern::Insulation);
        let inner = mat(&mut m, "LHD", CutPattern::Diagonal);
        let glass = mat(&mut m, "Vidrio", CutPattern::Solid);
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let facade = m
            .add_wall_type(
                "Fachada",
                WallFunction::Exterior,
                vec![
                    layer(render, 0.015, LayerFunction::Finish),
                    layer(brick, 0.115, LayerFunction::Structure),
                    layer(wool, 0.05, LayerFunction::Insulation),
                    layer(inner, 0.07, LayerFunction::Substrate),
                ],
            )
            .unwrap();
        let partition = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![layer(inner, 0.07, LayerFunction::Substrate)],
            )
            .unwrap();
        let detail = JambDetail::for_type(m.wall_type(facade).unwrap());
        assert_eq!(detail.turn, Some((1, 2..3)));
        assert_eq!(detail.wrap, Some(0));
        let none = JambDetail::for_type(m.wall_type(partition).unwrap());
        assert_eq!(
            none,
            JambDetail {
                turn: None,
                wrap: None
            }
        );

        let wall = m
            .add_wall(NewWall {
                wall_type: facade,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let window = m
            .add_opening_type(archisim_core::NewOpeningType {
                name: "V".into(),
                kind: OpeningKind::Window(WindowOperation::DoubleCasement),
                width: 1.2,
                height: 1.2,
                frame_width: 0.06,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: glass,
            })
            .unwrap();
        m.add_opening(archisim_core::NewOpening {
            opening_type: window,
            wall,
            along: 2.5,
            sill: 0.9,
            opens_to: archisim_core::WallSide::Interior,
            hand: archisim_core::Hand::Left,
            setback: Some(0.13),
        })
        .unwrap();
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // The brick returns and the render wraps at both jambs.
        assert_eq!(svg.matches("id=\"cut-0-turn-").count(), 2);
        assert_eq!(svg.matches("id=\"cut-0-wrap-").count(), 2);
        // Two leaves of (1.2 − 2 × 0.06) / 2 = 0.54 m swing into the room.
        assert_eq!(svg.matches(" A0.54,0.54 ").count(), 2);
    }

    #[test]
    fn slabs_follow_the_cut_plane() {
        let mut m = Model::new("Forjados");
        let lvl = m.add_level("Planta baja", 0.0);
        let hormigon = m.add_material("Hormigón", "concrete", [200, 200, 200], CutPattern::Cross);
        let st = m
            .add_slab_type(
                "Losa 20",
                SlabFunction::Floor,
                vec![Layer {
                    material: hormigon,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let square = |x: f64| {
            vec![
                Point2::new(x, 0.0),
                Point2::new(x + 4.0, 0.0),
                Point2::new(x + 4.0, 4.0),
                Point2::new(x, 4.0),
            ]
        };
        for (x, top) in [(0.0, 0.0), (5.0, 1.2), (10.0, 2.5)] {
            m.add_slab(NewSlab {
                slab_type: st,
                level: lvl,
                outline: square(x),
                top_offset: top,
            })
            .unwrap();
        }
        let svg = render_plan(&m, lvl, &PlanOptions::default());
        // Below: cover fill. Cut: structural fill with cross hatch. Above: dashed.
        assert!(svg.contains(SLAB_COVER));
        assert!(svg.contains("fill=\"rgb(200,200,200)\""));
        assert!(svg.contains("cut-slab-1"));
        assert!(svg.contains("stroke-dasharray"));
        // The drawing spans the three slabs: 14 m at 1:50 + margins.
        assert!(svg.contains("width=\"310.00mm\""), "{}", &svg[..200]);
    }

    /// The system families draw in plan what ArchiSim drew before families:
    /// every operation, two sizes, both sides, both hands, with and without
    /// sill, in a wall at an odd angle. Both drawings are rasterised and
    /// compared pixel by pixel.
    #[test]
    fn system_families_draw_the_previous_plan_symbols() {
        let mut m = Model::new("Regresión planta");
        let lvl = m.add_level("P1", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], Pattern::Diagonal);
        let wood = m.add_material("Madera", "wood", [190, 150, 100], Pattern::Solid);
        let glass = m.add_material("Vidrio", "glass", [190, 220, 240], Pattern::Solid);
        let wt = m
            .add_wall_type(
                "M",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.29,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: Point2::new(0.7, -1.3),
                end: Point2::new(0.7 + 500.0 * 0.6, -1.3 + 500.0 * 0.8),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let kinds = [
            OpeningKind::Door(DoorOperation::SingleSwing),
            OpeningKind::Door(DoorOperation::DoubleSwing),
            OpeningKind::Door(DoorOperation::Sliding),
            OpeningKind::Window(WindowOperation::Fixed),
            OpeningKind::Window(WindowOperation::SingleCasement),
            OpeningKind::Window(WindowOperation::DoubleCasement),
            OpeningKind::Window(WindowOperation::Sliding),
        ];
        let mut along = 1.0;
        let mut compared = 0;
        for kind in kinds {
            for (width, fw, fd) in [(0.83, 0.053, 0.071), (1.457, 0.067, 0.12)] {
                let t = m
                    .add_opening_type(NewOpeningType {
                        name: format!("{kind:?} {width}"),
                        kind,
                        width,
                        height: 1.4,
                        frame_width: fw,
                        frame_depth: fd,
                        frame_material: wood,
                        panel_material: glass,
                    })
                    .unwrap();
                let is_door = matches!(kind, OpeningKind::Door(_));
                for opens_to in [WallSide::Interior, WallSide::Exterior] {
                    for hand in [Hand::Left, Hand::Right] {
                        for sill in if is_door { vec![0.0] } else { vec![0.0, 0.9] } {
                            for setback in [None, Some(0.04)] {
                                let id = m
                                    .add_opening(NewOpening {
                                        opening_type: t,
                                        wall: w,
                                        along,
                                        sill,
                                        opens_to,
                                        hand,
                                        setback,
                                    })
                                    .unwrap();
                                along += 2.5;
                                let o = archisim_geom::opening_geometry(&m, id).unwrap();
                                let mut old = String::new();
                                legacy_symbols::draw_opening(&mut old, &o, 0.05);
                                let mut new = String::new();
                                draw_opening(&mut new, &m, &o, 50.0, 0.05);
                                assert_same_raster(
                                    &old,
                                    &new,
                                    &o,
                                    &format!(
                                        "{kind:?} {opens_to:?} {hand:?} sill {sill} {setback:?}"
                                    ),
                                );
                                compared += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(compared, 2 * 2 * 2 * 2 * (3 + 4 * 2));
    }

    /// Rasterises two symbol drawings around the opening and compares them.
    fn assert_same_raster(old: &str, new: &str, o: &OpeningGeometry, what: &str) {
        use resvg::{tiny_skia, usvg};
        let c = o.point((o.t0 + o.t1) / 2.0, 0.0);
        let half = 2.2;
        let doc = |body: &str| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="{} {} {} {}"><rect x="-1000" y="-1000" width="3000" height="3000" fill="white"/>{body}</svg>"#,
                c.x - half,
                -c.y - half,
                2.0 * half,
                2.0 * half
            )
        };
        let raster = |svg: &str| {
            let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
            let mut pm = tiny_skia::Pixmap::new(400, 400).unwrap();
            resvg::render(&tree, tiny_skia::Transform::identity(), &mut pm.as_mut());
            pm
        };
        let (a, b) = (raster(&doc(old)), raster(&doc(new)));
        let diffs: Vec<u8> = a
            .data()
            .chunks(4)
            .zip(b.data().chunks(4))
            .map(|(p, q)| {
                p.iter()
                    .zip(q.iter())
                    .map(|(x, y)| x.abs_diff(*y))
                    .max()
                    .unwrap()
            })
            .filter(|d| *d > 2)
            .collect();
        let worst = diffs.iter().copied().max().unwrap_or(0);
        assert!(!old.is_empty() && !new.is_empty(), "{what}");
        // A mirrored family may draw the same outlines starting at another
        // corner or in another order: that only leaves faint anti-aliasing
        // noise (at most 15/255 here). A moved or missing line would differ
        // by almost 255.
        assert!(
            worst <= 40,
            "{what}: {} pixels, worst {worst}:\nold {old}\nnew {new}",
            diffs.len()
        );
    }
}

#[cfg(test)]
mod num_tests {
    /// The fast number writer gives the same text as the shortest form.
    #[test]
    fn numbers_as_before() {
        for v in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.1,
            0.3,
            1.5,
            -0.25,
            12.000001,
            1.234_567_89,
            1e-7,
            -1e-7,
            123456.789012,
            0.05,
            1234.5,
            -987.654321,
            0.000_001,
            99_999.999_999_9,
        ] {
            let r = (v * 1e6_f64).round() / 1e6;
            let old = if r == 0.0 {
                "0".to_string()
            } else {
                format!("{r}")
            };
            assert_eq!(super::num(v), old, "{v}");
        }
    }

    /// The swing line style of a door type reaches the plan: dashed at 1:50
    /// is 2 mm dash and 1 mm gap of paper, so 0.1 and 0.05 m.
    #[test]
    fn swing_line_style_is_drawn() {
        use super::*;
        use archisim_core::{
            CutPattern as Pattern, DoorOperation, Hand, Layer, NewOpening, NewOpeningType, NewWall,
            OpeningKind, ReferenceLine,
        };
        let mut m = Model::new("Giro");
        let lvl = m.add_level("P1", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 100, 80], Pattern::Diagonal);
        let wood = m.add_material("Madera", "wood", [190, 150, 100], Pattern::Solid);
        let wt = m
            .add_wall_type(
                "M",
                WallFunction::Interior,
                vec![Layer {
                    material: brick,
                    thickness: 0.1,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(4.0, 0.0),
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let t = m
            .add_opening_type(NewOpeningType {
                name: "P".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 2.05,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: wood,
                panel_material: wood,
            })
            .unwrap();
        m.add_opening(NewOpening {
            opening_type: t,
            wall: w,
            along: 2.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        let arcs = |m: &Model| {
            render_plan(m, lvl, &PlanOptions::default())
                .lines()
                .filter(|l| l.contains(" A0.7,0.7 "))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let before = arcs(&m);
        assert_eq!(before.len(), 1);
        assert!(!before[0].contains("dasharray"));
        let ty = m.opening_type(t).unwrap().clone();
        let mut values = ty.values.clone();
        values.insert(
            "swing_line".into(),
            archisim_core::Value::Text("dashed".into()),
        );
        m.edit_family_type(t, ty.family, values).unwrap();
        let after = arcs(&m);
        assert!(
            after[0].contains(r#"stroke-dasharray="0.1 0.05""#),
            "{}",
            after[0]
        );
    }
}

#[cfg(test)]
mod thin_tests {
    #[test]
    fn hairlines_replace_every_weight() {
        let svg =
            r#"<path stroke-width="0.025"/><path d="M0 0" stroke-width="0.0045" fill="none"/>"#;
        assert_eq!(
            super::hairlines(svg, 0.005),
            r#"<path stroke-width="0.005"/><path d="M0 0" stroke-width="0.005" fill="none"/>"#
        );
    }
}
