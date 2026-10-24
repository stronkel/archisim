//! What the cut away walls take away (Los Sims' «walls cut away»): a
//! height over the active level above which the building is cut, except
//! the furniture, which stays whole; and, in the «walls in front» mode,
//! the walls between the camera and what is looked at cut low while the
//! ones behind stay up to their top.
//!
//! Every vertex of the scene carries what its element is (`Vertex::cut`):
//! its kind, a point of it in plan and its base. The shader and the
//! picking on the processor apply the same rule to it (`CutRule::top_of`,
//! and `cut_top` in `shaders.wgsl`).

use glam::Vec2;

/// An element cut at the height like the rest (slabs, roofs…).
pub const PLAIN: f32 = 0.0;
/// A wall, or what stands in one (doors, windows), or a column: in the
/// «walls in front» mode, low in front and whole behind.
pub const WALL: f32 = 1.0;
/// Furniture, stairs and things placed on the floor: whole if they stand
/// below the cut, gone if above (on an upper floor).
pub const OBJECT: f32 = 2.0;

/// Taller than anything: not cut.
pub const NOT_CUT: f32 = 1.0e9;

/// The walls in front of what is looked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Front {
    /// Horizontal direction towards the camera.
    pub toward: Vec2,
    /// `toward · target`: walls whose point is further towards the camera
    /// are in front.
    pub at: f32,
    /// Height the walls in front are cut at.
    pub low: f32,
    /// Walls standing above this (on upper floors) are cut as the rest.
    pub above: f32,
}

/// The cut of the walls of a 3D view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutRule {
    /// Height everything (but the furniture) is cut at.
    pub top: f32,
    pub front: Option<Front>,
}

impl CutRule {
    /// The height above which an element (the `cut` of its vertices) is
    /// cut away: `NOT_CUT` if it is not, `-NOT_CUT` if all of it goes.
    pub fn top_of(&self, cut: [f32; 4]) -> f32 {
        let [kind, x, y, base] = cut;
        if kind > 1.5 {
            return if base > self.top + 1e-3 {
                -NOT_CUT
            } else {
                NOT_CUT
            };
        }
        if kind > 0.5 {
            if let Some(f) = self.front {
                if base > f.above {
                    return self.top;
                }
                return if f.toward.dot(Vec2::new(x, y)) > f.at {
                    f.low
                } else {
                    NOT_CUT
                };
            }
        }
        self.top
    }

    /// As the shader reads it: the cut (top, on, low, front on) and the
    /// walls in front (towards x, y, at, above).
    pub fn uniforms(rule: Option<&CutRule>) -> ([f32; 4], [f32; 4]) {
        match rule {
            None => ([0.0; 4], [0.0; 4]),
            Some(r) => match r.front {
                None => ([r.top, 1.0, 0.0, 0.0], [0.0; 4]),
                Some(f) => (
                    [r.top, 1.0, f.low, 1.0],
                    [f.toward.x, f.toward.y, f.at, f.above],
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn furniture_stays_whole_and_front_walls_go_low() {
        let cut = CutRule {
            top: 1.2,
            front: None,
        };
        // A wardrobe on the floor: whole; one upstairs: gone.
        assert_eq!(cut.top_of([OBJECT, 0.0, 0.0, 0.0]), NOT_CUT);
        assert_eq!(cut.top_of([OBJECT, 0.0, 0.0, 3.0]), -NOT_CUT);
        // A wall and a slab: at the height.
        assert_eq!(cut.top_of([WALL, 0.0, 0.0, 0.0]), 1.2);
        assert_eq!(cut.top_of([PLAIN, 0.0, 0.0, 0.0]), 1.2);
        // Looking from the south (the camera towards -y) at (0, 0).
        let front = CutRule {
            top: 1.2,
            front: Some(Front {
                toward: Vec2::new(0.0, -1.0),
                at: 0.0,
                low: 0.3,
                above: 0.5,
            }),
        };
        // The south wall is in front: low; the north one stays up.
        assert_eq!(front.top_of([WALL, 0.0, -2.0, 0.0]), 0.3);
        assert_eq!(front.top_of([WALL, 0.0, 2.0, 0.0]), NOT_CUT);
        // A wall upstairs goes with the rest.
        assert_eq!(front.top_of([WALL, 0.0, 2.0, 3.0]), 1.2);
        // The slab above is cut.
        assert_eq!(front.top_of([PLAIN, 0.0, 0.0, 2.7]), 1.2);
    }
}
