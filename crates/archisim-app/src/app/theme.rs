//! The look of the window: two themes (light and dark) sharing one accent,
//! and the IBM Plex typefaces. The drawing itself (the plan) is always
//! white paper, whatever the theme: what is seen is what is printed.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow,
    Stroke, TextStyle, Theme, ThemePreference, Vec2,
};

/// The family for headings and the active tab (semibold Plex Sans).
pub const STRONG: &str = "plex-semibold";

/// The colours of the window chrome (everything but the drawing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Behind the panels (tab strip, gaps).
    pub ground: Color32,
    /// Panels: tools, navigator, properties, top and bottom bars.
    pub panel: Color32,
    /// Raised areas inside a panel: tool tiles, cards, fields.
    pub raised: Color32,
    pub line: Color32,
    pub text: Color32,
    /// Labels and secondary text (readable on `panel`).
    pub muted: Color32,
    /// Decoration and tools not available yet.
    pub faint: Color32,
    pub accent: Color32,
    /// Accent tint behind the active tool or the selected row.
    pub accent_soft: Color32,
    pub good: Color32,
    pub bad: Color32,
    /// Background of the floating hints over the drawing.
    pub hud: Color32,
}

pub const ACCENT: Color32 = Color32::from_rgb(30, 111, 217);

impl Palette {
    pub fn light() -> Self {
        Self {
            ground: Color32::from_rgb(237, 239, 242),
            panel: Color32::WHITE,
            raised: Color32::from_rgb(246, 247, 249),
            line: Color32::from_rgb(223, 226, 231),
            text: Color32::from_rgb(27, 30, 35),
            muted: Color32::from_rgb(92, 99, 113),
            faint: Color32::from_rgb(154, 161, 172),
            accent: ACCENT,
            accent_soft: Color32::from_rgb(228, 238, 251),
            good: Color32::from_rgb(36, 128, 80),
            bad: Color32::from_rgb(192, 48, 40),
            hud: Color32::from_rgba_unmultiplied(27, 30, 35, 228),
        }
    }

    pub fn dark() -> Self {
        Self {
            ground: Color32::from_rgb(21, 23, 27),
            panel: Color32::from_rgb(29, 32, 37),
            raised: Color32::from_rgb(37, 41, 48),
            line: Color32::from_rgb(52, 57, 64),
            text: Color32::from_rgb(232, 234, 237),
            muted: Color32::from_rgb(164, 171, 182),
            faint: Color32::from_rgb(110, 118, 130),
            accent: Color32::from_rgb(76, 147, 240),
            accent_soft: Color32::from_rgb(33, 52, 80),
            good: Color32::from_rgb(92, 190, 130),
            bad: Color32::from_rgb(240, 110, 100),
            hud: Color32::from_rgba_unmultiplied(12, 14, 17, 235),
        }
    }

    pub fn of(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// The palette of the theme `ui` is drawn with.
    pub fn current(ctx: &egui::Context) -> Self {
        Self::of(ctx.global_style().visuals.dark_mode)
    }
}

/// Installs the typefaces (once per context): IBM Plex Sans for the
/// interface and Plex Mono for measurements, with egui's own fonts behind
/// them for symbols Plex does not have.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add(
        &mut fonts,
        "plex-sans",
        include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
    );
    add(
        &mut fonts,
        STRONG,
        include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    );
    add(
        &mut fonts,
        "plex-mono",
        include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
    );
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let with_fallback = |first: &str| {
        let mut v = vec![first.to_owned()];
        v.extend(fallback.iter().cloned());
        v
    };
    fonts
        .families
        .insert(FontFamily::Proportional, with_fallback("plex-sans"));
    fonts
        .families
        .insert(FontFamily::Monospace, with_fallback("plex-mono"));
    fonts
        .families
        .insert(FontFamily::Name(STRONG.into()), with_fallback(STRONG));
    ctx.set_fonts(fonts);
}

/// Applies both themes' styles and chooses one.
pub fn apply(ctx: &egui::Context, dark: bool) {
    ctx.set_style_of(Theme::Light, style(Palette::light(), false));
    ctx.set_style_of(Theme::Dark, style(Palette::dark(), true));
    ctx.set_theme(if dark {
        ThemePreference::Dark
    } else {
        ThemePreference::Light
    });
}

fn style(p: Palette, dark: bool) -> egui::Style {
    let mut s = egui::Style {
        text_styles: [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (
                TextStyle::Heading,
                FontId::new(15.0, FontFamily::Name(STRONG.into())),
            ),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into(),
        ..Default::default()
    };
    s.spacing.item_spacing = Vec2::new(8.0, 6.0);
    s.spacing.button_padding = Vec2::new(8.0, 4.0);
    s.spacing.interact_size = Vec2::new(40.0, 26.0);
    s.spacing.menu_margin = Margin::same(6);
    s.spacing.window_margin = Margin::same(12);
    s.spacing.combo_width = 120.0;

    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let r6 = CornerRadius::same(6);
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.window_stroke = Stroke::new(1.0, p.line);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 110 } else { 38 }),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 90 } else { 30 }),
    };
    v.faint_bg_color = p.raised;
    v.extreme_bg_color = p.panel;
    v.text_edit_bg_color = Some(p.panel);
    v.code_bg_color = p.raised;
    v.hyperlink_color = p.accent;
    v.error_fg_color = p.bad;
    v.override_text_color = None;
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0, p.accent);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r6;

    w.inactive.bg_fill = p.raised;
    w.inactive.weak_bg_fill = p.raised;
    w.inactive.bg_stroke = Stroke::new(1.0, p.line);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r6;

    w.hovered.bg_fill = p.accent_soft;
    w.hovered.weak_bg_fill = p.accent_soft;
    w.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    w.hovered.fg_stroke = Stroke::new(1.5, p.text);
    w.hovered.corner_radius = r6;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.accent_soft;
    w.active.weak_bg_fill = p.accent_soft;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(2.0, p.accent);
    w.active.corner_radius = r6;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.raised;
    w.open.weak_bg_fill = p.raised;
    w.open.bg_stroke = Stroke::new(1.0, p.accent);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = r6;

    s.visuals = v;
    s
}
