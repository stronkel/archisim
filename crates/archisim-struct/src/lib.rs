//! Structural analysis of an ArchiSim model, by the Spanish code (CTE
//! DB-SE, DB-SE-AE, DB-SE-A, DB-SE-M, DB-SE-C, NCSE-02) and the
//! Eurocodes:
//!
//! - `actions`: the live loads by use, the snow and the wind of the site;
//! - `weights`: the weights of the materials and the element types;
//! - `cases`: the load cases and their combinations;
//! - `gravity`: the takedown of the vertical loads, from the floors and
//!   roofs down to the ground;
//! - `wind`: the wind on the building, storey by storey, and on each
//!   exterior wall;
//! - `seismic`: the earthquake by NCSE-02's simplified method;
//! - `frame`, `analytical`, `analysis`: the 3D frame of columns and beams;
//! - `cold`, `timber`, `lightframe`: the light frames;
//! - `concrete`, `members`, `slab`: reinforced concrete beams, columns and
//!   slabs (the slabs as a grillage);
//! - `frame_members`, `stability`, `connections`: steel and timber
//!   members, drift and second order (P-Δ amplified), and the joints;
//! - `detailing`: the construction rules of NCSE-02 against the
//!   reinforcement drawn;
//! - `masonry`, `foundations`, `retaining`: load-bearing brick, the
//!   footings and the basement and retaining walls;
//! - `phases`: in a refurbishment, the structure as it will stand;
//! - `report`: the calculation report.
//!
//! It works on the model as drawn: it adds no elements and stores
//! nothing but the site data and the loads placed by hand (in the core).
//! It is a design aid for the architect; the project's structural
//! calculation is signed by its author, who must check its hypotheses.

// Matrix code reads better with indices.
#![allow(clippy::needless_range_loop)]

pub mod actions;
pub mod analysis;
pub mod analytical;
pub mod boxes;
pub mod cases;
pub mod cold;
pub mod concrete;
pub mod connections;
pub mod detailing;
pub mod fire;
pub mod fix;
pub mod floors;
pub mod foundations;
pub mod frame;
pub mod frame_members;
pub mod geo;
pub mod gravity;
pub mod lightframe;
pub mod masonry;
pub mod members;
pub mod phases;
pub mod proposals;
pub mod report;
pub mod retaining;
pub mod sections;
pub mod seismic;
pub mod slab;
pub mod stability;
pub mod stairs;
pub mod text;
pub mod timber;
pub mod weights;
pub mod wind;

pub use actions::UseCategory;
pub use cases::{combinations, Case, Combination, Kind, WindDir};
pub use gravity::{takedown, Carried, Carrier, Takedown};

/// The actions of the project written out: site, floors and roofs, wind,
/// takedown and combinations.
pub fn actions_report(model: &archisim_core::Model) -> String {
    let td = takedown(model);
    let wind = wind::building_wind(model);
    let mut cases = td.cases.clone();
    if wind.is_some() {
        cases.extend(WindDir::ALL.map(Case::Wind));
    }
    let mut s = String::new();
    s.push_str(&text::site_text(model));
    s.push('\n');
    s.push_str(&text::surfaces_text(model, &td));
    s.push('\n');
    if let Some(w) = &wind {
        s.push_str(&text::wind_text(w));
        s.push('\n');
    }
    s.push_str(&text::takedown_text(model, &td));
    s.push('\n');
    s.push_str(&text::combinations_text(&cases, model.site_altitude()));
    s
}

/// Everything: the actions and the analysis of the frame.
pub fn structure_report(model: &archisim_core::Model) -> String {
    let gone = phases::demolished_count(model);
    let model = &*phases::as_built(model);
    let mut s = actions_report(model);
    if gone > 0 {
        s.push_str(&format!(
            "Reforma: {gone} elementos que se derriban quedan fuera del cálculo.\n"
        ));
    }
    let a = analysis::analyse(model);
    s.push('\n');
    s.push_str(&text::analysis_text(model, &a));
    let q = seismic::quake(model, &a.takedown);
    s.push('\n');
    s.push_str(&text::quake_text(&q));
    if let Some(m) = &a.model.modal {
        s.push_str(&text::modal_text(&q, m));
    }
    let checks = all_checks(model, &a);
    if !checks.is_empty() {
        s.push('\n');
        s.push_str(&text::lightframe_text(model, &checks));
    }
    s
}

/// Every check of the elements: light frames, concrete beams, columns
/// and slabs, masonry walls and foundations.
pub fn all_checks(model: &archisim_core::Model, a: &analysis::Analysis) -> Vec<lightframe::Check> {
    let mut out = lightframe::check(model, &a.takedown, a.wind.as_ref());
    out.extend(members::check(model, a));
    out.extend(frame_members::check(model, a));
    out.extend(connections::check(model, a));
    out.extend(detailing::check(model, a));
    out.extend(stability::check(model, a));
    out.extend(slab::check(model, &a.takedown));
    out.extend(floors::check(model, &a.takedown));
    out.extend(fire::check(model, a));
    out.extend(stairs::stair_checks(model));
    out.extend(stairs::lintel_checks(model, &a.takedown));
    out.extend(masonry::check(model, &a.takedown));
    out.extend(foundations::check(model, &a.takedown));
    out.extend(retaining::check(model));
    out
}
