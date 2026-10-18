//! Turning a door or window round in its wall, as Revit's flip controls
//! and its Space bar do.
//!
//! A door is turned by two things: the side of the wall its leaf opens to,
//! and the jamb its hinges are on. The hand (UNE-EN 12519) is read from the
//! side it opens to, so moving the leaf to the other side while keeping the
//! hinges on the same jamb changes the hand's name too.

use archisim_core::{Hand, WallSide};

/// Whether the hinges are on the jamb at the start of the wall, for a door
/// opening to `side` with `hand` (the same rule as the plan).
pub fn hinge_at_start(side: WallSide, hand: Hand) -> bool {
    (side == WallSide::Interior) == (hand == Hand::Left)
}

/// The hand that puts the hinges at the start of the wall (or not) for a
/// door opening to `side`.
pub fn hand_for(side: WallSide, at_start: bool) -> Hand {
    if (side == WallSide::Interior) == at_start {
        Hand::Left
    } else {
        Hand::Right
    }
}

fn other_side(side: WallSide) -> WallSide {
    match side {
        WallSide::Interior => WallSide::Exterior,
        WallSide::Exterior => WallSide::Interior,
    }
}

/// Opens to the other side of the wall, hinges on the same jamb (the
/// arrow across the wall).
pub fn flip_side(side: WallSide, hand: Hand) -> (WallSide, Hand) {
    let other = other_side(side);
    (other, hand_for(other, hinge_at_start(side, hand)))
}

/// Hinges on the other jamb, same side (the arrow along the wall).
pub fn flip_hand(side: WallSide, hand: Hand) -> (WallSide, Hand) {
    (
        side,
        match hand {
            Hand::Left => Hand::Right,
            Hand::Right => Hand::Left,
        },
    )
}

/// Space: the next of the four ways the door can be turned, going round
/// like a clock: hinges to the other jamb, then the leaf to the other side,
/// and so on.
pub fn next(side: WallSide, hand: Hand) -> (WallSide, Hand) {
    let at_start = hinge_at_start(side, hand);
    // (interior, start) → (interior, end) → (exterior, end) →
    // (exterior, start) → (interior, start).
    let (side, at_start) = match (side, at_start) {
        (WallSide::Interior, true) => (WallSide::Interior, false),
        (WallSide::Interior, false) => (WallSide::Exterior, false),
        (WallSide::Exterior, false) => (WallSide::Exterior, true),
        (WallSide::Exterior, true) => (WallSide::Interior, true),
    };
    (side, hand_for(side, at_start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use WallSide::*;

    #[test]
    fn space_goes_round_the_four_ways() {
        let mut state = (Interior, Hand::Left);
        let mut seen = vec![state];
        for _ in 0..4 {
            state = next(state.0, state.1);
            seen.push(state);
        }
        assert_eq!(seen[4], seen[0], "back where it started");
        let mut ways: Vec<_> = seen[..4]
            .iter()
            .map(|(s, h)| (*s == Interior, hinge_at_start(*s, *h)))
            .collect();
        ways.sort();
        ways.dedup();
        assert_eq!(ways.len(), 4, "{seen:?}");
        // Each press changes one thing only: the jamb or the side.
        for w in seen.windows(2) {
            let side_changed = w[0].0 != w[1].0;
            let jamb_changed = hinge_at_start(w[0].0, w[0].1) != hinge_at_start(w[1].0, w[1].1);
            assert!(side_changed != jamb_changed, "{w:?}");
        }
    }

    #[test]
    fn arrows_keep_the_jamb_or_the_side() {
        for side in [Interior, Exterior] {
            for hand in [Hand::Left, Hand::Right] {
                let (s, h) = flip_side(side, hand);
                assert_ne!(s, side);
                assert_eq!(hinge_at_start(s, h), hinge_at_start(side, hand));
                // The hand's name changes with the side it is read from.
                assert_ne!(h, hand);
                let (s, h) = flip_hand(side, hand);
                assert_eq!(s, side);
                assert_ne!(hinge_at_start(s, h), hinge_at_start(side, hand));
            }
        }
    }
}
