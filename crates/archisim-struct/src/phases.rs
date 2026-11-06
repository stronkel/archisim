//! Refurbishment: the structure is calculated as it will stand, so what
//! the project demolishes carries nothing and is not checked. The
//! existing elements that stay are calculated like the new ones (their
//! materials and reinforcement are those drawn: the architect must give
//! the ones found on site).

use std::borrow::Cow;

use archisim_core::{Model, Phase};

/// The model without what the project demolishes. Borrowed when nothing
/// is demolished.
pub fn as_built(model: &Model) -> Cow<'_, Model> {
    let gone = |id: u64| model.phase(id) == Phase::Demolished;
    let any = model.openings().any(|o| gone(o.id.0))
        || model.walls().any(|w| gone(w.id.0))
        || model.slabs().any(|s| gone(s.id.0))
        || model.columns().any(|c| gone(c.id.0))
        || model.beams().any(|b| gone(b.id.0))
        || model.roofs().any(|r| gone(r.id.0))
        || model.stairs().any(|s| gone(s.id.0))
        || model.footings().any(|f| gone(f.id.0))
        || model.railings().any(|r| gone(r.id.0))
        || model.curtain_walls().any(|c| gone(c.id.0))
        || model.objects().any(|o| gone(o.id.0));
    if !any {
        return Cow::Borrowed(model);
    }
    let mut m = model.clone();
    macro_rules! drop_gone {
        ($list:ident, $remove:ident) => {
            let ids: Vec<_> = m.$list().map(|e| e.id).filter(|id| gone(id.0)).collect();
            for id in ids {
                let _ = m.$remove(id);
            }
        };
    }
    // Doors and windows and footings first, then what hosts them.
    drop_gone!(openings, remove_opening);
    drop_gone!(footings, remove_footing);
    drop_gone!(railings, remove_railing);
    drop_gone!(objects, remove_object);
    drop_gone!(stairs, remove_stair);
    drop_gone!(curtain_walls, remove_curtain_wall);
    drop_gone!(beams, remove_beam);
    drop_gone!(columns, remove_column);
    drop_gone!(walls, remove_wall);
    drop_gone!(slabs, remove_slab);
    drop_gone!(roofs, remove_roof);
    Cow::Owned(m)
}

/// How many elements the calculation leaves out because they are
/// demolished.
pub fn demolished_count(model: &Model) -> usize {
    let gone = |id: u64| model.phase(id) == Phase::Demolished;
    model.walls().filter(|w| gone(w.id.0)).count()
        + model.slabs().filter(|s| gone(s.id.0)).count()
        + model.columns().filter(|c| gone(c.id.0)).count()
        + model.beams().filter(|b| gone(b.id.0)).count()
        + model.roofs().filter(|r| gone(r.id.0)).count()
        + model.stairs().filter(|s| gone(s.id.0)).count()
        + model.footings().filter(|f| gone(f.id.0)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demolished_walls_carry_nothing() {
        let mut m = Model::new("Reforma");
        crate::members::tests::portal(&mut m);
        let n = m.walls().count() + m.columns().count();
        assert!(matches!(as_built(&m), Cow::Borrowed(_)));
        let c = m.columns().next().unwrap().id;
        m.set_phase(c.0, Phase::Demolished).unwrap();
        let built = as_built(&m);
        assert_eq!(built.walls().count() + built.columns().count(), n - 1);
        assert_eq!(demolished_count(&m), 1);
    }
}
