//! The group «Entramado ligero» of the wall and slab types (roofs take
//! slab types): whether the type's structure layer is a light frame,
//! steel frame or timber frame, and with what studs, tracks, spacing,
//! noggins and boards. What it gives (the members in 3D, the cut list and
//! the panels) is worked out from the model by `archisim_geom::framing`.

use archisim_core::{
    steel_studs, steel_tracks, timber_sections, FrameMaterial, FrameSection, Framing, Layer, Model,
    Sheathing,
};
use eframe::egui::{self, Ui};

use super::props;

/// The boards that can go over a frame.
fn boards() -> Vec<Sheathing> {
    vec![
        Sheathing::osb(),
        Sheathing {
            name: "OSB/3 15 mm".into(),
            thickness: 0.015,
            board: (2.5, 1.25),
        },
        Sheathing {
            name: "Contrachapado fenólico 18 mm".into(),
            thickness: 0.018,
            board: (2.5, 1.25),
        },
        Sheathing {
            name: "Placa de yeso 12,5 mm".into(),
            thickness: 0.0125,
            board: (2.6, 1.2),
        },
        Sheathing {
            name: "Placa de cemento 12,5 mm".into(),
            thickness: 0.0125,
            board: (2.4, 1.2),
        },
    ]
}

/// The group for type `id` with `layers`. Returns the new framing (or
/// `Some(None)` to take it away) when something changed.
pub fn framing_group(
    ui: &mut Ui,
    model: &Model,
    id: u64,
    layers: &[Layer],
    floor: bool,
) -> Option<Option<Framing>> {
    let mut out = None;
    let now = model.type_framing(id).cloned();
    let depth = Model::frame_layer(layers).map_or(0.15, |k| layers[k].thickness);
    props::group(
        ui,
        &format!("types-framing-{id}"),
        "Entramado ligero",
        |ui| {
            let mut on = now.is_some();
            props::row(
            ui,
            "Entramado",
            "La capa de estructura del tipo es un entramado ligero: ArchiSim coloca los montantes, canales, dinteles, viguetas o cabios en cada elemento de este tipo y saca su despiece (Archivo → Exportar)",
            |ui, _| {
                super::widgets::switch(ui, &mut on, "");
            },
        );
            if on != now.is_some() {
                out = Some(on.then(|| Framing::for_depth(FrameMaterial::Steel, depth)));
                return;
            }
            let Some(f) = now.clone() else {
                ui.label(
                egui::RichText::new(
                    "Actívalo para levantar los muros, forjados y cubiertas de este tipo en steel frame o en madera.",
                )
                .small()
                .color(super::theme::Palette::current(ui.ctx()).muted),
            );
                return;
            };
            let mut next = f.clone();
            props::row(ui, "Sistema", "", |ui, w| {
                next.material = props::choice(
                    ui,
                    ("fr-mat", id),
                    w,
                    f.material,
                    &[
                        (FrameMaterial::Steel, "Steel frame (acero)"),
                        (FrameMaterial::Timber, "Madera C24"),
                    ],
                );
            });
            if next.material != f.material {
                let mut g = Framing::for_depth(next.material, depth);
                g.spacing = f.spacing;
                g.noggins = f.noggins;
                g.sheathing = f.sheathing.clone();
                out = Some(Some(g));
                return;
            }
            let (studs, tracks): (Vec<FrameSection>, Vec<FrameSection>) = match f.material {
                FrameMaterial::Steel => (steel_studs(), steel_tracks()),
                FrameMaterial::Timber => (timber_sections(), timber_sections()),
            };
            let pick =
                |ui: &mut Ui, salt: &str, w: f32, list: &[FrameSection], now: &FrameSection| {
                    let at = list
                        .iter()
                        .position(|s| s.name == now.name)
                        .unwrap_or(usize::MAX);
                    let names: Vec<(usize, &str)> = list
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (i, s.name.as_str()))
                        .collect();
                    let v = props::choice(ui, (salt, id), w, at, &names);
                    (v != at).then(|| list.get(v).cloned()).flatten()
                };
            let (main, edge) = if floor {
                ("Viguetas / cabios", "Zunchos")
            } else {
                ("Montantes", "Canales / durmientes")
            };
            props::row(ui, main, "Perfil de las piezas principales (canto × ala × pestaña × espesor en acero; ancho × canto en madera)", |ui, w| {
            if let Some(s) = pick(ui, "fr-stud", w, &studs, &f.stud) {
                next.stud = s;
            }
        });
            props::row(
                ui,
                edge,
                "Perfil de los bordes, travesaños y antepechos",
                |ui, w| {
                    if let Some(s) = pick(ui, "fr-track", w, &tracks, &f.track) {
                        next.track = s;
                    }
                },
            );
            props::row(
                ui,
                "Separación",
                "Entre ejes de montantes, viguetas o cabios",
                |ui, w| {
                    let mm = (f.spacing * 1000.0).round() as u32;
                    let v = props::choice(
                        ui,
                        ("fr-sp", id),
                        w,
                        mm,
                        &[
                            (300, "300 mm"),
                            (400, "400 mm"),
                            (488, "488 mm"),
                            (600, "600 mm"),
                            (625, "625 mm"),
                        ],
                    );
                    next.spacing = v as f64 / 1000.0;
                },
            );
            props::row(
            ui,
            "Travesaños",
            "Una fila de travesaños (bloqueos) a media altura entre montantes, o de riostras a media luz entre viguetas",
            |ui, _| {
                super::widgets::switch(ui, &mut next.noggins, "");
            },
        );
            props::row(ui, "Tablero", "El tablero de arriostramiento sobre una cara (para la compra y el peso de los paneles)", |ui, w| {
            let list = boards();
            let at = f
                .sheathing
                .as_ref()
                .and_then(|s| list.iter().position(|b| b.name == s.name))
                .map_or(0, |i| i + 1);
            let mut names: Vec<(usize, &str)> = vec![(0, "Sin tablero")];
            names.extend(list.iter().enumerate().map(|(i, b)| (i + 1, b.name.as_str())));
            let v = props::choice(ui, ("fr-board", id), w, at, &names);
            if v != at {
                next.sheathing = if v == 0 { None } else { list.get(v - 1).cloned() };
            }
        });
            if f.stud.depth > depth + 0.006 {
                ui.label(
                    egui::RichText::new(format!(
                    "El perfil tiene más canto ({:.0} mm) que la capa de estructura ({:.0} mm).",
                    f.stud.depth * 1000.0,
                    depth * 1000.0
                ))
                    .small()
                    .color(super::theme::Palette::current(ui.ctx()).bad),
                );
            }
            if next != f {
                out = Some(Some(next));
            }
        },
    );
    out
}
