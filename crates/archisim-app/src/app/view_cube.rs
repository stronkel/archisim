//! Over the 3D view (`docs/INTERFAZ.md` §12.5): the cube of views, quiet
//! and see-through in a corner, that turns with the camera (its faces go to
//! the plan and the elevations, its top corners to the corner views); the
//! looks of the picture; what the keys and buttons do with the camera of
//! the mode; and, while the view turns, the point it turns about.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use glam::Vec3;

use super::theme::Palette;
use super::ArchiSimApp;
use crate::view3d::look::Quality;
use crate::view3d::render::TextureMode;

/// Half the side of the cube on screen, in points.
const HALF: f32 = 21.0;

/// A face of the cube: its outward normal and its name.
const FACES: [(Vec3, &str); 5] = [
    (Vec3::Z, "Planta"),
    (Vec3::new(0.0, -1.0, 0.0), "Sur"),
    (Vec3::X, "Este"),
    (Vec3::Y, "Norte"),
    (Vec3::new(-1.0, 0.0, 0.0), "Oeste"),
];

/// Where a view of the cube puts the camera: the direction it stands in
/// (`yaw`, from the east, counter-clockwise) and how high it looks from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubeView {
    pub yaw: f32,
    pub pitch: f32,
}

impl CubeView {
    /// The view through a face (a plan, or an elevation looked at level,
    /// with its verticals straight) or from a top corner (`normal` with x
    /// and y of ±1).
    pub fn of(normal: Vec3) -> CubeView {
        if normal.z > 0.5 && normal.x == 0.0 && normal.y == 0.0 {
            // North up.
            return CubeView {
                yaw: -std::f32::consts::FRAC_PI_2,
                pitch: 1.5,
            };
        }
        CubeView {
            yaw: normal.y.atan2(normal.x),
            pitch: if normal.z > 0.5 {
                30.0_f32.to_radians()
            } else {
                0.0
            },
        }
    }
}

/// The corners of the face with outward normal `n`, in order around it.
fn face_corners(n: Vec3) -> [Vec3; 4] {
    // Two axes across the face.
    let (a, b) = if n.z.abs() > 0.5 {
        (Vec3::X, Vec3::Y)
    } else if n.x.abs() > 0.5 {
        (Vec3::Y, Vec3::Z)
    } else {
        (Vec3::X, Vec3::Z)
    };
    [n - a - b, n + a - b, n + a + b, n - a + b]
}

/// Whether `p` is inside the convex polygon `poly`.
fn inside(poly: &[Pos2], p: Pos2) -> bool {
    let mut sign = 0.0_f32;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}

impl ArchiSimApp {
    /// Takes a view of the cube: the camera turns and frames the model.
    pub(super) fn take_cube_view(&mut self, view: CubeView) {
        let cam = &mut self.v3d.camera;
        cam.yaw = view.yaw;
        cam.pitch = view.pitch;
        self.v3d.fit_requested = true;
    }

    /// The cube of views, in the top right corner of the 3D view.
    pub(super) fn view_cube(&mut self, ctx: &egui::Context, rect: Rect) {
        let p = Palette::current(ctx);
        let (_, right, up) = self.v3d.camera.screen_axes();
        let back = -self.v3d.camera.forward();
        let hovered = self.v3d.cube_hover;
        let alpha = if hovered { 1.0 } else { 0.55 };
        let size = Vec2::new(112.0, if hovered { 126.0 } else { 102.0 });
        let origin = rect.right_top() + Vec2::new(-size.x - 6.0, 6.0);
        let mut chosen: Option<CubeView> = None;
        let mut turn: Option<f32> = None;
        let mut home = false;
        let mut over = false;
        egui::Area::new(egui::Id::unique("view-cube"))
            .order(egui::Order::Foreground)
            .fixed_pos(origin)
            .show(ctx, |ui| {
                let (area, resp) = ui.allocate_exact_size(size, Sense::click());
                over = resp.hovered() || ui.rect_contains_pointer(area);
                let painter = ui.painter_at(area.expand(2.0));
                let centre = area.left_top() + Vec2::new(size.x / 2.0, 48.0);
                let to_screen = |v: Vec3| centre + Vec2::new(v.dot(right), -v.dot(up)) * HALF;
                let pointer = ui.input(|i| i.pointer.hover_pos());
                let fade = |c: Color32| c.gamma_multiply(alpha);
                // The faces seen, to keep the compass clear of them.
                let seen: Vec<Vec<Pos2>> = FACES
                    .iter()
                    .filter(|(n, _)| n.dot(back) > 0.05)
                    .map(|(n, _)| face_corners(*n).iter().map(|c| to_screen(*c)).collect())
                    .collect();
                // The four points of the compass, on the ground around it,
                // pushed out from behind the cube.
                for (dir, name) in [
                    (Vec3::Y, "N"),
                    (Vec3::new(0.0, -1.0, 0.0), "S"),
                    (Vec3::X, "E"),
                    (Vec3::new(-1.0, 0.0, 0.0), "O"),
                ] {
                    // Not when it points straight at the eye or away.
                    if Vec2::new(dir.dot(right), dir.dot(up)).length() < 0.3 {
                        continue;
                    }
                    let mut r = 2.0;
                    let mut at = to_screen(dir * r - Vec3::Z);
                    while r < 3.2
                        && seen
                            .iter()
                            .any(|f| inside(f, at) || f.iter().any(|c| c.distance(at) < 4.0))
                    {
                        r += 0.25;
                        at = to_screen(dir * r - Vec3::Z);
                    }
                    painter.text(
                        at,
                        Align2::CENTER_CENTER,
                        name,
                        if name == "N" {
                            super::widgets::strong_font(9.5)
                        } else {
                            FontId::proportional(9.0)
                        },
                        fade(if name == "N" { p.text } else { p.muted }),
                    );
                }
                // The faces seen, the hovered one in the accent.
                let mut face_hit = None;
                for (n, name) in FACES {
                    let facing = n.dot(back);
                    if facing <= 0.05 {
                        continue;
                    }
                    let poly: Vec<Pos2> = face_corners(n).iter().map(|c| to_screen(*c)).collect();
                    let hit = pointer.is_some_and(|q| inside(&poly, q));
                    if hit {
                        face_hit = Some(n);
                    }
                    // Lighter towards the light, as a drawn cube.
                    let shade = 236.0 - (1.0 - facing) * 26.0 - if n.z > 0.5 { 0.0 } else { 8.0 };
                    let fill = if hit {
                        p.accent_soft
                    } else {
                        Color32::from_gray(shade as u8)
                    };
                    painter.add(egui::Shape::convex_polygon(
                        poly.clone(),
                        fade(fill),
                        Stroke::new(0.8, fade(Color32::from_gray(150))),
                    ));
                    // The name, when the face is seen enough to hold it.
                    if facing > 0.35 {
                        let c = to_screen(n);
                        painter.text(
                            c,
                            Align2::CENTER_CENTER,
                            name,
                            FontId::proportional(8.5),
                            fade(if hit {
                                p.accent
                            } else {
                                Color32::from_gray(70)
                            }),
                        );
                    }
                }
                // The top corners: the corner views.
                let mut corner_hit = None;
                if back.z > 0.05 {
                    for (x, y) in [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)] {
                        let c = Vec3::new(x, y, 1.0);
                        // Only those on the side seen.
                        if Vec3::new(x, y, 0.0).dot(back) < -0.2 {
                            continue;
                        }
                        let s = to_screen(c);
                        let hit = pointer.is_some_and(|q| q.distance(s) < 6.0);
                        if hit {
                            corner_hit = Some(c);
                        }
                        let r = if hit || hovered { 3.6 } else { 2.4 };
                        painter.circle(
                            s,
                            r,
                            fade(if hit {
                                p.accent
                            } else {
                                Color32::from_gray(165)
                            }),
                            Stroke::new(1.0, fade(Color32::WHITE)),
                        );
                    }
                }
                if resp.clicked() {
                    if let Some(c) = corner_hit {
                        chosen = Some(CubeView::of(c));
                    } else if let Some(n) = face_hit {
                        chosen = Some(CubeView::of(n));
                    }
                }
                if hovered {
                    // Turn 90° either way and the view of always.
                    let row = Rect::from_min_size(
                        area.left_top() + Vec2::new(4.0, 104.0),
                        Vec2::new(size.x - 8.0, 20.0),
                    );
                    let w = row.width() / 3.0;
                    for (i, label) in ["‹ 90°", "Inicio", "90° ›"].into_iter().enumerate() {
                        let b = Rect::from_min_size(
                            row.left_top() + Vec2::new(i as f32 * w, 0.0),
                            Vec2::new(w - 2.0, 20.0),
                        )
                        .shrink2(Vec2::new(1.0, 0.0));
                        let r =
                            ui.interact(b, ui.scope_id().with(("cube-button", i)), Sense::click());
                        painter.rect_filled(
                            b,
                            CornerRadius::same(5),
                            if r.hovered() {
                                p.accent_soft
                            } else {
                                Color32::from_white_alpha(210)
                            },
                        );
                        painter.text(
                            b.center(),
                            Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(9.5),
                            p.text,
                        );
                        let r = r.on_hover_text(match i {
                            0 => "Girar la vista 90° a la izquierda",
                            1 => "La vista de siempre, desde el sureste (Inicio)",
                            _ => "Girar la vista 90° a la derecha",
                        });
                        if r.clicked() {
                            match i {
                                0 => turn = Some(std::f32::consts::FRAC_PI_2),
                                1 => home = true,
                                _ => turn = Some(-std::f32::consts::FRAC_PI_2),
                            }
                        }
                    }
                }
                if over {
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
        if self.v3d.cube_hover != over {
            ctx.request_repaint();
        }
        self.v3d.cube_hover = over;
        if let Some(v) = chosen {
            self.take_cube_view(v);
        }
        if let Some(a) = turn {
            self.v3d.camera.yaw += a;
        }
        if home {
            let d = crate::view3d::camera::Camera::default();
            self.take_cube_view(CubeView {
                yaw: d.yaw,
                pitch: d.pitch,
            });
        }
    }

    /// The looks of the picture, top left of the 3D view: shaded without
    /// textures, with them, or realistic (finer shadows and the shade of
    /// the corners).
    pub(super) fn look_bar(&mut self, ctx: &egui::Context, rect: Rect) {
        let p = Palette::current(ctx);
        let v = &self.v3d;
        let current = if v.style == 2 {
            4
        } else if v.style == 1 {
            3
        } else if v.textures == TextureMode::Off {
            0
        } else if v.quality == Quality::Pretty {
            2
        } else {
            1
        };
        let mut chosen = None;
        egui::Area::new(egui::Id::unique("look-bar"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.left_top() + Vec2::new(8.0, 8.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_white_alpha(225))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (i, (name, hint)) in [
                                ("Sombreado", "Los colores de los materiales, sin texturas: lo más ligero"),
                                ("Texturas", "Ladrillo, madera, piedra… con las sombras del sol"),
                                ("Realista", "Sombras más finas y la penumbra de los rincones: para enseñar y para capturas"),
                                ("Aristas", "Las caras en el color de su material, pálido y plano, y todas las aristas en tinta: para leer la forma"),
                                ("Líneas ocultas", "Papel blanco y las aristas que se ven en tinta, como un dibujo"),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                let on = current == i;
                                let b = egui::Button::new(
                                    egui::RichText::new(name).size(11.5).color(if on {
                                        p.text
                                    } else {
                                        Color32::from_gray(90)
                                    }),
                                )
                                .fill(if on {
                                    Color32::WHITE
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(b).on_hover_text(hint).clicked() {
                                    chosen = Some(i);
                                }
                            }
                        });
                    });
            });
        if let Some(i) = chosen {
            let v = &mut self.v3d;
            v.style = match i {
                3 => 1,
                4 => 2,
                _ => 0,
            };
            match i {
                3 | 4 => {}
                0 => v.textures = TextureMode::Off,
                1 => {
                    if v.textures == TextureMode::Off {
                        v.textures = super::view3d_ui::default_textures();
                    }
                    v.quality = Quality::Normal;
                }
                _ => {
                    if v.textures == TextureMode::Off {
                        v.textures = super::view3d_ui::default_textures();
                    }
                    v.quality = Quality::Pretty;
                }
            }
        }
    }

    /// What the keys and the buttons do with the camera of the mode, bottom
    /// right of the 3D view; and, while the view turns about a point, that
    /// point in orange.
    pub(super) fn camera_hint(&self, painter: &egui::Painter, rect: Rect, turning: bool) {
        let v = &self.v3d;
        if turning && !v.flying {
            if let Some(pivot) = v.orbit_pivot {
                let size = rect.size();
                if let Some(s) = v
                    .camera
                    .project_to_screen(pivot, glam::Vec2::new(size.x, size.y))
                {
                    let c = rect.min + Vec2::new(s.x, s.y);
                    let orange = Color32::from_rgb(232, 119, 46);
                    painter.circle_stroke(c, 7.0, Stroke::new(1.6, orange));
                    painter.circle_filled(c, 2.2, orange);
                }
            }
        }
        let (name, keys) = if v.walk_eye.is_some() {
            (
                "Recorrido",
                "W A S D: andar · Mayús: correr · derecho: mirar · Esc: salir",
            )
        } else if self.maquette {
            (
                "Cámara de maqueta",
                "W A S D: mover por el suelo · derecho: girar · rueda: acercar",
            )
        } else {
            (
                "Cámara libre",
                "Derecho: girar alrededor de lo que hay debajo · derecho + W A S D: volar · rueda: acercar",
            )
        };
        let font = FontId::proportional(11.0);
        let bold = super::widgets::strong_font(11.0);
        let g1 = painter.layout_no_wrap(name.to_string(), bold, Color32::WHITE);
        let g2 = painter.layout_no_wrap(keys.to_string(), font, Color32::from_gray(214));
        let w = g1.size().x + g2.size().x + 30.0;
        let r = Rect::from_min_size(
            rect.right_bottom() + Vec2::new(-w - 10.0, -34.0),
            Vec2::new(w, 24.0),
        );
        painter.rect_filled(
            r,
            CornerRadius::same(8),
            Color32::from_rgba_unmultiplied(27, 30, 35, 200),
        );
        let y = r.center().y;
        let x = r.left() + 10.0;
        painter.galley(
            Pos2::new(x, y - g1.size().y / 2.0),
            g1.clone(),
            Color32::WHITE,
        );
        painter.galley(
            Pos2::new(x + g1.size().x + 10.0, y - g2.size().y / 2.0),
            g2,
            Color32::from_gray(214),
        );
    }
}

impl ArchiSimApp {
    /// How the 3D view moves (§12.5): three speeds and the helps against
    /// getting lost. In the bottom bar of the 3D view and in Vista.
    pub(super) fn navigation_controls(&mut self, ui: &mut egui::Ui) {
        use super::widgets::SwitchExt as _;
        let p = Palette::current(ui.ctx());
        let nav = &mut self.prefs.nav;
        let word = |v: u8| match v {
            1..=2 => "muy lenta",
            3..=4 => "lenta",
            5..=6 => "media",
            7..=8 => "rápida",
            _ => "muy rápida",
        };
        ui.set_min_width(300.0);
        ui.label(egui::RichText::new("Navegación 3D").strong());
        for (name, hint, v) in [
            (
                "Paseo",
                "W A S D en la Maqueta, y con el botón derecho en Técnica",
                &mut nav.walk,
            ),
            (
                "Giro",
                "Lo que gira la vista al arrastrar con el botón derecho",
                &mut nav.turn,
            ),
            (
                "Rueda",
                "Lo que acerca o aleja cada paso de la rueda",
                &mut nav.wheel,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.add_sized([52.0, 18.0], egui::Label::new(name))
                    .on_hover_text(hint);
                ui.add(egui::Slider::new(v, 1..=10).show_value(false));
                ui.label(egui::RichText::new(word(*v)).small().color(p.muted));
            });
        }
        ui.switch(
            &mut nav.pivot_under_cursor,
            "Girar alrededor de lo que hay bajo el cursor",
        )
        .on_hover_text("Sin nada debajo, alrededor de lo seleccionado o del centro de la vista");
        ui.switch(&mut nav.keep_near, "No alejarse más allá del modelo")
            .on_hover_text("La vista no se va más lejos de unas veces el tamaño del modelo, al girar ni con la rueda");
        ui.switch(&mut nav.invert, "Invertir el ratón al mirar");
        ui.label(
            egui::RichText::new("Se guarda en este ordenador.")
                .small()
                .color(p.muted),
        );
    }

    /// The button of the navigation in the bottom bar of the 3D view.
    pub(super) fn navigation_button(&mut self, ui: &mut egui::Ui) {
        let speed = self.prefs.nav.walk;
        let r = ui
            .button(format!("Navegación · {speed}"))
            .on_hover_text("Velocidad de paseo, de giro y de la rueda, y ayudas para no perderse");
        egui::Popup::menu(&r)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| self.navigation_controls(ui));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_views_stand_where_they_say() {
        // The plan from above with north up.
        let plan = CubeView::of(Vec3::Z);
        assert!(plan.pitch > 1.4);
        // The south elevation: from the south, level (straight verticals).
        let south = CubeView::of(Vec3::new(0.0, -1.0, 0.0));
        assert!((south.yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(south.pitch, 0.0);
        // The south-east corner from above, as the view of always.
        let se = CubeView::of(Vec3::new(1.0, -1.0, 1.0));
        assert!((se.yaw + std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        assert!(se.pitch > 0.4 && se.pitch < 0.6);
    }

    #[test]
    fn points_inside_a_face() {
        let sq = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!(inside(&sq, Pos2::new(5.0, 5.0)));
        assert!(!inside(&sq, Pos2::new(15.0, 5.0)));
        let corners = face_corners(Vec3::X);
        assert!(corners.iter().all(|c| c.x == 1.0));
    }
}
