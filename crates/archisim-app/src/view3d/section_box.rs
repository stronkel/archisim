//! The section box of the 3D view (Revit's "Caja de sección", ArchiCAD's
//! 3D cutaway): an axis-aligned box outside of which the model is cut
//! away, to look at interiors or at a perspective section. Each of its six
//! faces has a handle that drags it along its axis.

use glam::{Vec2, Vec3};

use super::camera::Camera;

/// Smallest size of the box along any axis, in metres.
pub const MIN_SIZE: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionBox {
    pub lo: Vec3,
    pub hi: Vec3,
}

/// A face of the box: its axis (0 = X, 1 = Y, 2 = Z) and whether it is the
/// high one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Face {
    pub axis: usize,
    pub high: bool,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face {
            axis: 0,
            high: false,
        },
        Face {
            axis: 0,
            high: true,
        },
        Face {
            axis: 1,
            high: false,
        },
        Face {
            axis: 1,
            high: true,
        },
        Face {
            axis: 2,
            high: false,
        },
        Face {
            axis: 2,
            high: true,
        },
    ];

    /// The outward direction of the face.
    pub fn normal(self) -> Vec3 {
        let mut n = Vec3::ZERO;
        n[self.axis] = if self.high { 1.0 } else { -1.0 };
        n
    }

    /// Its name in Spanish (for the hint).
    pub fn name(self) -> &'static str {
        match (self.axis, self.high) {
            (0, false) => "oeste",
            (0, true) => "este",
            (1, false) => "sur",
            (1, true) => "norte",
            (2, false) => "inferior",
            _ => "superior",
        }
    }
}

impl SectionBox {
    /// A box around `lo`–`hi`, a little bigger so nothing is cut at first.
    pub fn around(lo: Vec3, hi: Vec3) -> Self {
        let m = Vec3::splat(0.25);
        SectionBox {
            lo: lo - m,
            hi: hi + m,
        }
    }

    pub fn contains(&self, p: Vec3) -> bool {
        let e = 1e-4;
        p.cmpge(self.lo - Vec3::splat(e)).all() && p.cmple(self.hi + Vec3::splat(e)).all()
    }

    /// The middle of a face, where its handle is.
    pub fn handle(&self, f: Face) -> Vec3 {
        let mut c = (self.lo + self.hi) / 2.0;
        c[f.axis] = if f.high {
            self.hi[f.axis]
        } else {
            self.lo[f.axis]
        };
        c
    }

    /// The position of a face along its axis.
    pub fn face(&self, f: Face) -> f32 {
        if f.high {
            self.hi[f.axis]
        } else {
            self.lo[f.axis]
        }
    }

    /// Moves a face to `value` along its axis, keeping the box at least
    /// [`MIN_SIZE`] deep.
    pub fn set_face(&mut self, f: Face, value: f32) {
        let a = f.axis;
        if f.high {
            self.hi[a] = value.max(self.lo[a] + MIN_SIZE);
        } else {
            self.lo[a] = value.min(self.hi[a] - MIN_SIZE);
        }
    }

    /// The twelve edges, for drawing.
    pub fn edges(&self) -> [(Vec3, Vec3); 12] {
        let (l, h) = (self.lo, self.hi);
        let c = |x: bool, y: bool, z: bool| {
            Vec3::new(
                if x { h.x } else { l.x },
                if y { h.y } else { l.y },
                if z { h.z } else { l.z },
            )
        };
        [
            (c(false, false, false), c(true, false, false)),
            (c(false, true, false), c(true, true, false)),
            (c(false, false, true), c(true, false, true)),
            (c(false, true, true), c(true, true, true)),
            (c(false, false, false), c(false, true, false)),
            (c(true, false, false), c(true, true, false)),
            (c(false, false, true), c(false, true, true)),
            (c(true, false, true), c(true, true, true)),
            (c(false, false, false), c(false, false, true)),
            (c(true, false, false), c(true, false, true)),
            (c(false, true, false), c(false, true, true)),
            (c(true, true, false), c(true, true, true)),
        ]
    }

    /// How far along its axis a face moves when its handle is dragged by
    /// `delta` screen points, seen by `camera` in a view of `size` points:
    /// the drag projected on the axis as it appears on the screen.
    pub fn drag_amount(&self, f: Face, delta: Vec2, camera: &Camera, size: Vec2) -> f32 {
        let h = self.handle(f);
        let (Some(a), Some(b)) = (
            camera.project_to_screen(h, size),
            camera.project_to_screen(h + f.normal(), size),
        ) else {
            return 0.0;
        };
        // Screen points per metre along the face's outward direction.
        let s = b - a;
        let len2 = s.length_squared();
        if len2 < 1e-3 {
            // Seen end-on: it cannot be dragged sensibly.
            return 0.0;
        }
        let along = delta.dot(s) / len2;
        if f.high {
            along
        } else {
            -along
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_move_and_keep_the_box_open() {
        let mut b = SectionBox {
            lo: Vec3::ZERO,
            hi: Vec3::new(10.0, 8.0, 6.0),
        };
        let top = Face {
            axis: 2,
            high: true,
        };
        assert_eq!(b.handle(top), Vec3::new(5.0, 4.0, 6.0));
        b.set_face(top, 2.0);
        assert_eq!(b.hi.z, 2.0);
        b.set_face(top, -5.0);
        assert!((b.hi.z - MIN_SIZE).abs() < 1e-6);
        assert!(b.contains(Vec3::new(1.0, 1.0, 0.05)));
        assert!(!b.contains(Vec3::new(1.0, 1.0, 1.0)));
        assert_eq!(b.edges().len(), 12);
    }

    #[test]
    fn dragging_a_handle_follows_its_axis_on_screen() {
        let cam = Camera {
            target: Vec3::new(5.0, 4.0, 3.0),
            yaw: (-90.0f32).to_radians(),
            pitch: 0.0,
            distance: 30.0,
            perspective: false,
        };
        let b = SectionBox {
            lo: Vec3::ZERO,
            hi: Vec3::new(10.0, 8.0, 6.0),
        };
        let size = Vec2::new(800.0, 600.0);
        let east = Face {
            axis: 0,
            high: true,
        };
        // Seen from the south, east is to the right: dragging right pushes
        // the east face out, dragging up does nothing.
        assert!(b.drag_amount(east, Vec2::new(50.0, 0.0), &cam, size) > 0.5);
        assert!(b.drag_amount(east, Vec2::new(0.0, -50.0), &cam, size).abs() < 1e-3);
        let top = Face {
            axis: 2,
            high: true,
        };
        // Screen Y grows downwards: dragging up raises the top.
        assert!(b.drag_amount(top, Vec2::new(0.0, -50.0), &cam, size) > 0.5);
        // The south face is seen end-on.
        let south = Face {
            axis: 1,
            high: false,
        };
        assert_eq!(b.drag_amount(south, Vec2::new(30.0, 30.0), &cam, size), 0.0);
    }
}
