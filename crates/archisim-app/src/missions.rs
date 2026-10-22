//! Onboarding missions (`docs/INTERFAZ.md` §6, `docs/INICIACION.md`):
//! short guided tasks ("tu primer muro", "tu primera planta", "tu primer
//! plano") that show where each thing is. Each step says what to do,
//! names the control to use (the window rings it) and knows when it is
//! done by looking at the program, never by asking.
//!
//! No interface here: what the program looks like now is an [`Obs`], made
//! by the window every frame; the card and the rings are in
//! `app/missions_ui`.

use crate::app::Tool;

/// The kind of view in the active tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewKind {
    #[default]
    Plan,
    ThreeD,
    Section,
    Sheet,
    Schedule,
}

/// What the program looks like now, as far as the missions care.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Obs {
    pub tool: Option<Tool>,
    /// The discipline shown, by name ("Arquitectura").
    pub discipline: &'static str,
    pub walls: usize,
    pub slabs: usize,
    pub doors: usize,
    pub windows: usize,
    pub rooms: usize,
    pub levels: usize,
    pub sheets: usize,
    pub viewports: usize,
    pub view: ViewKind,
    /// The active level, counted from the lowest (0).
    pub level_index: usize,
    /// The window of levels is open.
    pub levels_open: bool,
    /// The navigator shows the project (views, sheets), not the model.
    pub nav_project: bool,
    /// The wall tool draws rectangles.
    pub wall_rectangle: bool,
    /// The slab tool fills a room of walls with one click.
    pub slab_wand: bool,
}

/// A step: what to do, the control to use (the key its widget marks
/// itself with, or "" for the drawing) and when it is done, from what the
/// program looks like now and when the mission began.
pub struct Step {
    pub say: &'static str,
    pub target: &'static str,
    pub done: fn(now: &Obs, start: &Obs) -> bool,
}

pub struct Mission {
    /// Kept in the preferences once done.
    pub id: &'static str,
    pub title: &'static str,
    /// One line on what it teaches.
    pub about: &'static str,
    pub steps: &'static [Step],
    /// What to try next, once done.
    pub then: &'static str,
}

const fn step(say: &'static str, target: &'static str, done: fn(&Obs, &Obs) -> bool) -> Step {
    Step { say, target, done }
}

pub const MISSIONS: &[Mission] = &[
    Mission {
        id: "muro",
        title: "Tu primer muro",
        about: "Disciplinas, fichas, métodos de dibujo y medidas exactas.",
        steps: &[
            step(
                "En la barra de la izquierda, elige la disciplina Arquitectura.",
                "rail:Arquitectura",
                |o, _| o.discipline == "Arquitectura",
            ),
            step(
                "Pulsa la ficha Muro (o la tecla M). Debajo, en Propiedades, está su tipo.",
                "tile:Muro",
                |o, _| o.tool == Some(Tool::Wall),
            ),
            step(
                "En la barra bajo las pestañas, elige Rectángulo: cuatro muros de una vez.",
                "method:Rectángulo",
                |o, _| o.tool == Some(Tool::Wall) && o.wall_rectangle,
            ),
            step(
                "Haz clic en una esquina de la planta y otro en la opuesta. Para medidas exactas, tras el primer clic teclea 8x6 (metros) e Intro.",
                "",
                |o, s| o.walls >= s.walls + 4,
            ),
        ],
        then: "Con la Flecha (Esc) selecciona un muro: arrastra sus extremos o cambia su tipo en Propiedades.",
    },
    Mission {
        id: "huecos",
        title: "Puertas y ventanas",
        about: "Colocar en un muro, girar y ajustar con las cotas azules.",
        steps: &[
            step(
                "Pulsa la ficha Puerta (P).",
                "tile:Puerta",
                |o, _| o.tool == Some(Tool::Door),
            ),
            step(
                "Pasa el cursor por un muro: verás la puerta. Clic para colocarla; Espacio la gira antes o después.",
                "",
                |o, s| o.doors > s.doors,
            ),
            step(
                "Ahora la ficha Ventana (V).",
                "tile:Ventana",
                |o, _| o.tool == Some(Tool::Window),
            ),
            step(
                "Clic en un muro para colocarla. Con ella seleccionada, pulsa una cota azul y teclea la distancia.",
                "",
                |o, s| o.windows > s.windows,
            ),
        ],
        then: "En Propiedades cambia el tipo de la ventana y su antepecho; el dibujo y el 3D lo siguen.",
    },
    Mission {
        id: "forjado",
        title: "Tu primer forjado",
        about: "La varita mágica: un clic dentro de un recinto de muros.",
        steps: &[
            step(
                "Pulsa la ficha Forjado (F).",
                "tile:Forjado",
                |o, _| o.tool == Some(Tool::Slab),
            ),
            step(
                "En la barra bajo las pestañas, elige Varita.",
                "method:Varita",
                |o, _| o.tool == Some(Tool::Slab) && o.slab_wand,
            ),
            step(
                "Haz clic dentro de los muros: el forjado llega hasta ellos.",
                "",
                |o, s| o.slabs > s.slabs,
            ),
        ],
        then: "La herramienta Local (ficha Local) hace lo mismo con los recintos: nombre, uso y superficie.",
    },
    Mission {
        id: "3d",
        title: "Míralo en 3D",
        about: "El navegador del proyecto y las pestañas de vistas.",
        steps: &[
            step(
                "En el navegador de la derecha, pasa a la pestaña Proyecto.",
                "seg:Proyecto",
                |o, _| o.nav_project,
            ),
            step(
                "Abre la Vista 3D (o pulsa F3).",
                "nav:Vista 3D",
                |o, _| o.view == ViewKind::ThreeD,
            ),
            step(
                "Arrastra para girar y usa la rueda para acercarte. Vuelve después a la planta con su pestaña, arriba.",
                "tab:plan",
                |o, _| o.view == ViewKind::Plan,
            ),
        ],
        then: "Las herramientas también funcionan en la ventana 3D: prueba a dibujar un muro en ella.",
    },
    Mission {
        id: "planta",
        title: "Tu primera planta",
        about: "Añadir plantas y moverse entre ellas.",
        steps: &[
            step(
                "En el navegador (pestaña Proyecto), pulsa «Plantas…».",
                "button:Plantas…",
                |o, s| o.levels_open || o.levels > s.levels,
            ),
            step(
                "Pulsa «Añadir planta encima»: tendrá la altura de la de abajo.",
                "button:Añadir planta encima",
                |o, s| o.levels > s.levels,
            ),
            step(
                "Cierra la ventana y abre la planta nueva en el navegador, en Plantas.",
                "nav:planta-alta",
                |o, _| o.view == ViewKind::Plan && o.level_index > 0 && !o.levels_open,
            ),
        ],
        then: "En gris ves el calco de la planta de abajo para dibujar encima (Vista → Calco).",
    },
    Mission {
        id: "plano",
        title: "Tu primer plano",
        about: "Colocar una planta en un plano para imprimirla.",
        steps: &[
            step(
                "En el navegador (pestaña Proyecto), pulsa «+ Nuevo plano».",
                "button:+ Nuevo plano",
                |o, s| o.sheets > s.sheets,
            ),
            step(
                "Arrastra una planta desde el navegador hasta el papel: queda a su escala.",
                "nav:planta",
                |o, s| o.viewports > s.viewports,
            ),
        ],
        then: "Archivo → Exportar PDF o DXF; el cajetín se rellena en las propiedades del plano.",
    },
];

/// The mission in hand and how far it has got.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub mission: usize,
    pub step: usize,
    /// What the program looked like when the mission began (counts are
    /// compared with it: "one more wall").
    pub start: Obs,
}

impl Progress {
    pub fn begin(mission: usize, now: &Obs) -> Self {
        let mut p = Progress {
            mission,
            step: 0,
            start: *now,
        };
        p.advance(now);
        p
    }

    pub fn mission(&self) -> &'static Mission {
        &MISSIONS[self.mission]
    }

    pub fn current(&self) -> Option<&'static Step> {
        self.mission().steps.get(self.step)
    }

    pub fn finished(&self) -> bool {
        self.step >= self.mission().steps.len()
    }

    /// Moves past every step already done (one may do several at once, or
    /// have done them before). True when this finishes the mission.
    pub fn advance(&mut self, now: &Obs) -> bool {
        let was = self.finished();
        while let Some(s) = self.current() {
            if (s.done)(now, &self.start) {
                self.step += 1;
            } else {
                break;
            }
        }
        !was && self.finished()
    }

    /// Skips the step (when it cannot be done here, or is known).
    pub fn skip(&mut self, now: &Obs) {
        if !self.finished() {
            self.step += 1;
            // Counts of the next steps start from here.
            self.start = Obs {
                levels_open: self.start.levels_open,
                ..*now
            };
        }
    }
}

/// The next mission not done, after `from` (or the first).
pub fn next_mission(done: &[String], from: Option<usize>) -> Option<usize> {
    let start = from.map_or(0, |i| i + 1);
    (start..MISSIONS.len())
        .chain(0..start)
        .find(|&i| !done.iter().any(|d| d == MISSIONS[i].id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missions_are_well_made() {
        let mut ids = std::collections::BTreeSet::new();
        for m in MISSIONS {
            assert!(ids.insert(m.id), "{} repeated", m.id);
            assert!(!m.steps.is_empty());
            // Nothing is done at the start of a fresh project.
            let fresh = Obs {
                discipline: "Concepto",
                levels: 1,
                ..Default::default()
            };
            let p = Progress::begin(MISSIONS.iter().position(|x| x.id == m.id).unwrap(), &fresh);
            assert_eq!(p.step, 0, "{}", m.id);
        }
    }

    #[test]
    fn steps_follow_what_is_done() {
        let mut now = Obs {
            discipline: "Concepto",
            levels: 1,
            walls: 2,
            ..Default::default()
        };
        let mut p = Progress::begin(0, &now);
        assert_eq!(p.current().unwrap().target, "rail:Arquitectura");
        // Choosing the wall tool (from the search, say) does its step too.
        now.discipline = "Arquitectura";
        now.tool = Some(Tool::Wall);
        assert!(!p.advance(&now));
        assert_eq!(p.step, 2);
        now.wall_rectangle = true;
        p.advance(&now);
        assert_eq!(p.step, 3);
        // Two walls were there before: four more are needed.
        now.walls = 5;
        assert!(!p.advance(&now));
        now.walls = 6;
        assert!(p.advance(&now));
        assert!(p.finished());
        // Done once only.
        assert!(!p.advance(&now));
    }

    #[test]
    fn skipping_counts_from_there() {
        let mut now = Obs::default();
        let i = MISSIONS.iter().position(|m| m.id == "plano").unwrap();
        let mut p = Progress::begin(i, &now);
        now.sheets = 1;
        p.advance(&now);
        assert_eq!(p.step, 1);
        p.skip(&now);
        assert!(p.finished());
    }

    #[test]
    fn next_mission_skips_the_done() {
        let done = vec!["muro".to_string(), "huecos".to_string()];
        assert_eq!(next_mission(&done, None), Some(2));
        assert_eq!(next_mission(&done, Some(2)), Some(3));
        let all: Vec<String> = MISSIONS.iter().map(|m| m.id.to_string()).collect();
        assert_eq!(next_mission(&all, None), None);
    }
}
