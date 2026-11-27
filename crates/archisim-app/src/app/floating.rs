//! The drawing over the whole window, with the bars and palettes floating
//! on it (W10, `docs/WORKFEEL.md` §19, as Synaps and 1to10): rounded
//! cards with a soft shadow and air around them. Each card is an
//! [`egui::Area`] over the view, so what is under them is still the
//! drawing; the view is fitted to what the cards leave free
//! ([`ArchiSimApp::free_area`]).

use eframe::egui;
use egui::{Color32, CornerRadius, Id, Pos2, Rect, Stroke, Ui, Vec2};

use super::{context_bar, ribbon, theme, ArchiSimApp, ViewMode, FOLDED_WIDTH, RAIL_WIDTH};

/// Air between the cards and the edge of the window.
const EDGE: f32 = 12.0;
/// Air between two cards.
const GAP: f32 = 8.0;
/// The corners of the cards.
const ROUND: u8 = 18;
/// Inside the cards, left and right.
const PAD: f32 = 10.0;

/// The heights of the cards that grow with what they hold, as measured in
/// the last frame (the cards around them are placed with them).
#[derive(Debug, Clone, Copy)]
pub struct Measured {
    pub top: f32,
    pub bottom: f32,
    pub header: f32,
}

impl Default for Measured {
    fn default() -> Self {
        Measured {
            top: 34.0,
            bottom: 34.0,
            header: 150.0,
        }
    }
}

/// The ids of the cards, to tell them from the dialogs.
const CARDS: [&str; 8] = [
    "float-top",
    "float-bottom",
    "float-rail",
    "float-tools",
    "float-tools-folded",
    "float-navigator",
    "float-navigator-folded",
    "float-header",
];
const COMMAND_CARD: &str = "float-command";

impl ArchiSimApp {
    pub(super) fn floating_frame(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let p = theme::Palette::current(&ctx);
        let full = ui.max_rect();
        let m = self.floating;
        let left0 = full.left() + EDGE;
        let right0 = full.right() - EDGE;
        let body_top = full.top() + EDGE + m.top + GAP;
        let body_bottom = full.bottom() - EDGE - m.bottom - GAP;
        let rail_w = RAIL_WIDTH + 8.0;
        let tools_x = left0 + rail_w + GAP;
        let tools_w = if self.tools_folded {
            FOLDED_WIDTH
        } else {
            (self.side_width - RAIL_WIDTH).max(200.0)
        };
        let nav_w = if self.show_navigator {
            self.side_width
        } else {
            FOLDED_WIDTH
        };
        let left_edge = tools_x + tools_w + GAP;
        let right_edge = right0 - nav_w - GAP;

        // The view first, under everything: the whole window, except the
        // tables, which are read and so go where nothing covers them.
        let header_bottom = body_top + m.header + GAP;
        let free = Rect::from_min_max(
            Pos2::new(left_edge, header_bottom),
            Pos2::new(
                right_edge.max(left_edge + 1.0),
                body_bottom.max(header_bottom + 1.0),
            ),
        );
        self.free_area = Some(free);
        let view_rect = if self.mode == ViewMode::Schedule {
            free
        } else {
            full
        };
        ui.painter().rect_filled(full, CornerRadius::ZERO, p.ground);
        ui.scope_builder(egui::UiBuilder::new().max_rect(view_rect), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.active_view_ui(ui);
        });

        let card = |id: &str| card_frame(&p, id);
        // Top and bottom bars, as wide as the window.
        let w = right0 - left0;
        let top = show_card(
            &ctx,
            "float-top",
            Pos2::new(left0, full.top() + EDGE),
            card("top"),
            |ui| {
                ui.set_width(w - 2.0 * PAD);
                crate::profile::time("barra superior", || self.top_bar(ui));
            },
        );
        let bottom_y = full.bottom() - EDGE - m.bottom;
        let bottom = show_card(
            &ctx,
            "float-bottom",
            Pos2::new(left0, bottom_y),
            card("bottom"),
            |ui| {
                ui.set_width(w - 2.0 * PAD);
                crate::profile::time("barra inferior", || self.bottom_bar(ui));
            },
        );
        let body_h = (body_bottom - body_top).max(80.0);
        // The disciplines, a narrow column as tall as they are.
        show_card(
            &ctx,
            "float-rail",
            Pos2::new(left0, body_top),
            card_frame(&p, "rail").inner_margin(egui::Margin::symmetric(4, 6)),
            |ui| {
                ui.set_width(RAIL_WIDTH);
                crate::profile::time("disciplinas", || self.discipline_rail(ui));
                ui.add_space(4.0);
            },
        );
        // Tools and Properties, or the strip that unfolds them.
        if self.tools_folded {
            show_card(
                &ctx,
                "float-tools-folded",
                Pos2::new(tools_x, body_top),
                card_frame(&p, "f").inner_margin(egui::Margin::symmetric(2, 8)),
                |ui| {
                    ui.set_width(FOLDED_WIDTH - 6.0);
                    if super::fold_button(
                        ui,
                        super::icons::Icon::ChevronRight,
                        "Mostrar las herramientas y las propiedades",
                    ) {
                        self.tools_folded = false;
                    }
                },
            );
        } else {
            show_card(
                &ctx,
                "float-tools",
                Pos2::new(tools_x, body_top),
                card("tools"),
                |ui| {
                    let size = Vec2::new(tools_w - 2.0 * PAD, body_h - 12.0);
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    crate::profile::time("herramientas y propiedades", || self.tool_column(ui));
                },
            );
        }
        // The navigator, on the right.
        let nav_x = right0 - nav_w;
        if self.show_navigator {
            show_card(
                &ctx,
                "float-navigator",
                Pos2::new(nav_x, body_top),
                card("nav"),
                |ui| {
                    let size = Vec2::new(nav_w - 2.0 * PAD, body_h - 12.0);
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    crate::profile::time("navegador", || self.navigator(ui));
                },
            );
        } else {
            show_card(
                &ctx,
                "float-navigator-folded",
                Pos2::new(nav_x, body_top),
                card_frame(&p, "f").inner_margin(egui::Margin::symmetric(2, 8)),
                |ui| {
                    ui.set_width(FOLDED_WIDTH - 6.0);
                    if super::fold_button(
                        ui,
                        super::icons::Icon::ChevronLeft,
                        "Mostrar el navegador",
                    ) {
                        self.show_navigator = true;
                    }
                },
            );
        }
        // The tabs of views, the ribbon and its options, between the two.
        let header_w = (right_edge - left_edge).max(200.0);
        let header = show_card(
            &ctx,
            "float-header",
            Pos2::new(left_edge, body_top),
            card("header").inner_margin(egui::Margin::symmetric(6, 4)),
            |ui| {
                ui.set_width(header_w - 12.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                crate::profile::time("pestañas de vistas", || self.tab_strip(ui));
                if self.mode != ViewMode::Schedule {
                    thin_line(ui, &p);
                    let h = if self.prefs.ribbon_compact {
                        ribbon::RIBBON_COMPACT_HEIGHT
                    } else {
                        ribbon::RIBBON_HEIGHT
                    };
                    ui.allocate_ui(Vec2::new(ui.available_width(), h), |ui| {
                        ui.set_height(h);
                        crate::profile::time("cinta", || self.ribbon(ui));
                    });
                    thin_line(ui, &p);
                    ui.allocate_ui(
                        Vec2::new(ui.available_width(), context_bar::OPTIONS_BAR_HEIGHT),
                        |ui| {
                            ui.set_height(context_bar::OPTIONS_BAR_HEIGHT);
                            crate::profile::time("barra de opciones", || self.options_bar(ui));
                        },
                    );
                }
            },
        );
        // The command line, when it is pinned, above the bottom bar.
        if !matches!(self.mode, ViewMode::ThreeD | ViewMode::Schedule) && self.command_line_shown()
        {
            let h = super::COMMAND_LINE_HEIGHT;
            show_card(
                &ctx,
                COMMAND_CARD,
                Pos2::new(left_edge, body_bottom - h - 8.0),
                card("cmd").inner_margin(egui::Margin::symmetric(10, 3)),
                |ui| {
                    ui.set_width(header_w - 20.0);
                    ui.set_height(h - 6.0);
                    self.command_line(ui);
                },
            );
        }
        let measured = Measured {
            top: top.height(),
            bottom: bottom.height(),
            header: header.height(),
        };
        if (measured.top - m.top).abs() > 0.5
            || (measured.bottom - m.bottom).abs() > 0.5
            || (measured.header - m.header).abs() > 0.5
        {
            self.floating = measured;
            ctx.request_repaint();
        }
        keep_dialogs_above(&ctx);
        if matches!(self.mode, ViewMode::Plan | ViewMode::ThreeD) {
            self.street_bar(&ctx, free);
        }
    }

    /// The view of the active tab, in all the room `ui` has.
    pub(super) fn active_view_ui(&mut self, ui: &mut Ui) {
        match self.mode {
            ViewMode::Plan => crate::profile::time("planta", || self.plan_view(ui)),
            ViewMode::ThreeD => crate::profile::time("3D", || self.view_3d(ui)),
            ViewMode::Section => crate::profile::time("sección", || self.section_view(ui)),
            ViewMode::Schedule => {
                if let Some(super::tabs::TabKind::Schedule(w)) =
                    self.tabs.get(self.active_tab).map(|t| t.kind)
                {
                    self.schedule_view(ui, w);
                }
            }
            ViewMode::Sheet(s) => self.sheet_view(ui, s),
            ViewMode::Drafting(d) => self.drafting_view(ui, d),
        }
        if matches!(
            self.mode,
            ViewMode::Plan | ViewMode::Section | ViewMode::Sheet(_) | ViewMode::Drafting(_)
        ) {
            let ctx = ui.ctx().clone();
            let area = self.fit_area(self.canvas);
            self.mini_bar(&ctx, area);
        }
    }

    /// Where a view is fitted: the part of `rect` the floating cards
    /// leave free, or all of it.
    pub(super) fn fit_area(&self, rect: Rect) -> Rect {
        self.free_area
            .map(|f| f.intersect(rect))
            .filter(|r| r.width() > 80.0 && r.height() > 80.0)
            .unwrap_or(rect)
    }
}

fn card_frame(p: &theme::Palette, _id: &str) -> egui::Frame {
    egui::Frame::new()
        .fill(p.panel)
        .corner_radius(CornerRadius::same(ROUND))
        .stroke(Stroke::new(1.0, p.line))
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 22,
            spread: 0,
            color: Color32::from_black_alpha(26),
        })
        .inner_margin(egui::Margin::symmetric(PAD as i8, 4))
}

/// A card at `at`, returning the rectangle it took.
fn show_card(
    ctx: &egui::Context,
    id: &str,
    at: Pos2,
    frame: egui::Frame,
    add: impl FnOnce(&mut Ui),
) -> Rect {
    egui::Area::new(Id::unique(id))
        .fixed_pos(at)
        .order(egui::Order::Middle)
        .constrain(false)
        .show(ctx, |ui| {
            frame.show(ui, add);
        })
        .response
        .rect
}

fn thin_line(ui: &mut Ui, p: &theme::Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .hline(r.x_range(), r.center().y, Stroke::new(1.0, p.line));
}

/// The cards are areas like the dialogs: a click on a card brings it over
/// them. Then the dialogs are brought back on top, in their order.
fn keep_dialogs_above(ctx: &egui::Context) {
    let pressed = ctx.input(|i| i.pointer.any_pressed());
    if !pressed {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else {
        return;
    };
    let ours = |l: &egui::LayerId| {
        CARDS
            .iter()
            .chain([COMMAND_CARD].iter())
            .any(|c| l.id == Id::unique(*c))
    };
    let on_card = ctx.layer_id_at(pos).is_some_and(|l| ours(&l));
    if !on_card {
        return;
    }
    let others: Vec<egui::LayerId> = ctx.memory(|m| {
        m.layer_ids()
            .filter(|l| l.order == egui::Order::Middle && !ours(l) && m.areas().is_visible(l))
            .collect()
    });
    ctx.memory_mut(|m| {
        for l in others {
            m.areas_mut().move_to_top(l);
        }
    });
}
