//! The group «Entramado ligero» of the wall and slab types (roofs take
//! slab types): whether the type's structure layer is a light frame,
//! steel frame or timber frame, and with what studs, tracks, spacing,
//! noggins and boards. What it gives (the members in 3D, the cut list and
//! the panels) is worked out from the model by `archisim_geom::framing`.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use archisim_core::{
    parse_section_catalogue, steel_studs, steel_tracks, timber_sections, FrameMaterial,
    FrameSection, Framing, Layer, Model, Sheathing,
};
use eframe::egui::{self, Ui};

use super::props;

/// The library folder with the studio's own section catalogues (a
/// supplier's profiles, say): text files with one section a line, read by
/// `parse_section_catalogue`. The program itself carries only generic
/// sections, without brands.
pub const FOLDER: &str = "Perfiles de entramado";

/// How often the folder is looked at again.
const LOOK_AGAIN: Duration = Duration::from_secs(5);

pub fn catalogue_folder() -> PathBuf {
    crate::editor::library::library_dir().join(FOLDER)
}

/// The sections of every catalogue in the library folder, by file name
/// order (read again every few seconds, not every frame).
pub fn library_sections() -> Vec<(FrameMaterial, FrameSection)> {
    type Cache = Option<(Instant, Vec<(FrameMaterial, FrameSection)>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, list)) = cache.as_ref() {
        if at.elapsed() < LOOK_AGAIN {
            return list.clone();
        }
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(catalogue_folder())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| {
                ["csv", "txt", "tsv"]
                    .iter()
                    .any(|e| x.eq_ignore_ascii_case(e))
            })
        })
        .collect();
    files.sort();
    let list: Vec<_> = files
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .flat_map(|b| parse_section_catalogue(&String::from_utf8_lossy(&b)))
        .collect();
    *cache = Some((Instant::now(), list.clone()));
    list
}

/// The generic sections of `material` followed by those of the
/// catalogues: studs (or joists) and tracks. Lipped steel sections of a
/// catalogue go first among the studs, plain U first among the tracks;
/// both lists carry all of them.
pub fn sections_for(
    material: FrameMaterial,
    extra: &[(FrameMaterial, FrameSection)],
) -> (Vec<FrameSection>, Vec<FrameSection>) {
    let (mut studs, mut tracks) = match material {
        FrameMaterial::Steel => (steel_studs(), steel_tracks()),
        FrameMaterial::Timber => (timber_sections(), timber_sections()),
    };
    let mine: Vec<&FrameSection> = extra
        .iter()
        .filter(|(m, _)| *m == material)
        .map(|(_, s)| s)
        .collect();
    let add = |list: &mut Vec<FrameSection>, first: bool| {
        let (a, b): (Vec<&FrameSection>, Vec<&FrameSection>) = mine
            .iter()
            .partition(|s| material == FrameMaterial::Timber || (s.lip > 0.0) == first);
        for s in a.into_iter().chain(b) {
            if !list.iter().any(|x| x.name == s.name) {
                list.push(s.clone());
            }
        }
    };
    add(&mut studs, true);
    add(&mut tracks, false);
    (studs, tracks)
}

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
    extra: &[(FrameMaterial, FrameSection)],
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
            let (mut studs, mut tracks) = sections_for(f.material, extra);
            // A section that came from a catalogue no longer in the
            // library is still shown, as it is.
            for (list, now) in [(&mut studs, &f.stud), (&mut tracks, &f.track)] {
                if !list.iter().any(|s| s.name == now.name) {
                    list.push(now.clone());
                }
            }
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
            ui.label(
                egui::RichText::new(format!(
                    "Comprobación de los perfiles en Colaboración → Cálculo de la estructura. Tus catálogos de perfiles, en la carpeta «{FOLDER}» de la biblioteca."
                ))
                .small()
                .color(super::theme::Palette::current(ui.ctx()).muted),
            );
            if next != f {
                out = Some(Some(next));
            }
        },
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_sections_join_the_generic_ones() {
        let extra = parse_section_catalogue(
            "C-90;acero;90;40;12;1\nU-92;acero;92;35;0;1\nPino 45x95;madera;95;45\n",
        );
        let (studs, tracks) = sections_for(FrameMaterial::Steel, &extra);
        let n = steel_studs().len();
        assert_eq!(studs.len(), n + 2);
        assert_eq!(studs[n].name, "C-90", "lipped first among the studs");
        assert_eq!(
            tracks[steel_tracks().len()].name,
            "U-92",
            "plain U first among the tracks"
        );
        let (wood, _) = sections_for(FrameMaterial::Timber, &extra);
        assert!(wood.iter().any(|s| s.name == "Pino 45x95"));
        assert!(!wood.iter().any(|s| s.name == "C-90"));
    }
}
