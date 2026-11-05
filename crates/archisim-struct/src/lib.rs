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
//! - `masonry`, `foundations`: load-bearing brick and the footings;
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
pub mod cases;
pub mod cold;
pub mod concrete;
pub mod foundations;
pub mod frame;
pub mod geo;
pub mod gravity;
pub mod lightframe;
pub mod masonry;
pub mod members;
pub mod report;
pub mod sections;
pub mod seismic;
pub mod slab;
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
    let mut s = actions_report(model);
    let a = analysis::analyse(model);
    s.push('\n');
    s.push_str(&text::analysis_text(model, &a));
    let q = seismic::quake(model, &a.takedown);
    s.push('\n');
    s.push_str(&text::quake_text(&q));
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
    out.extend(slab::check(model, &a.takedown));
    out.extend(masonry::check(model, &a.takedown));
    out.extend(foundations::check(model, &a.takedown));
    out
}
