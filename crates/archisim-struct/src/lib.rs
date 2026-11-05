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
//!   exterior wall.
//!
//! It works on the model as drawn: it adds no elements and stores
//! nothing but the site data and the loads placed by hand (in the core).
//! It is a design aid for the architect; the project's structural
//! calculation is signed by its author, who must check its hypotheses.

pub mod actions;
pub mod cases;
pub mod geo;
pub mod gravity;
pub mod text;
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
