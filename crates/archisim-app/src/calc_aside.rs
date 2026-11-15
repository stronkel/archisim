//! The long calculations, aside (`docs/IPAD.md` §12): the structure and
//! the electrical installation of a big building take seconds. They are
//! worked out on another thread on the desktop and in one of the
//! browser's workers on the web, and the window shows them when they come.
//!
//! The browser's worker has its own copy of the program and memory, so
//! what goes there and back is text: the project as its `.archisim`, and
//! the result as JSON (`run`, `structure_from`, `electrical_from`).

// What goes to the browser's workers is used only there.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::sync::{Arc, Mutex};

use archisim_core::Model;
use archisim_struct::lightframe::Status;
use archisim_struct::report::Block;
use serde_json::{json, Value};

/// What can be calculated aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calc {
    /// The structure: its annex's blocks, its text and how many fail.
    Structure,
    /// The electrical installation: its blocks, its text and its boards.
    Electrical,
}

impl Calc {
    pub fn name(self) -> &'static str {
        match self {
            Calc::Structure => "estructura",
            Calc::Electrical => "electricidad",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "estructura" => Some(Calc::Structure),
            "electricidad" => Some(Calc::Electrical),
            _ => None,
        }
    }
}

/// A calculation's result: its text, a number (failing checks, boards)
/// and its blocks.
pub type Outcome = (String, usize, Vec<Block>);

/// Works a calculation out, here.
pub fn calculate(calc: Calc, model: &Model) -> Outcome {
    match calc {
        Calc::Structure => {
            let blocks = archisim_struct::report::report(model);
            let text = archisim_struct::report::to_text(&blocks);
            let fails = blocks
                .iter()
                .filter(|b| {
                    matches!(
                        b,
                        Block::Check {
                            status: Status::Fail,
                            ..
                        }
                    )
                })
                .count();
            (text, fails, blocks)
        }
        Calc::Electrical => {
            let text = archisim_mep::electrical::report(model);
            let all = archisim_mep::electrical::analysis(model);
            let blocks = archisim_plan::services_report::electricity(model, &all);
            (text, all.installations.len(), blocks)
        }
    }
}

/// A calculation as text, for the browser's worker.
pub fn run(calc: Calc, model: &Model) -> String {
    let (text, n, blocks) = calculate(calc, model);
    json!({ "text": text, "n": n, "blocks": blocks_to_json(&blocks) }).to_string()
}

/// The result written by `run`.
pub fn read(json_text: &str) -> Result<Outcome, String> {
    let v: Value = serde_json::from_str(json_text).map_err(|e| e.to_string())?;
    let text = v["text"].as_str().unwrap_or_default().to_string();
    let n = v["n"].as_u64().unwrap_or(0) as usize;
    let blocks = blocks_from_json(&v["blocks"]);
    Ok((text, n, blocks))
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Pass => "pass",
        Status::Warn => "warn",
        Status::Fail => "fail",
        Status::Info => "info",
    }
}

fn status_of(name: &str) -> Status {
    match name {
        "pass" => Status::Pass,
        "warn" => Status::Warn,
        "fail" => Status::Fail,
        _ => Status::Info,
    }
}

pub fn blocks_to_json(blocks: &[Block]) -> Value {
    Value::Array(
        blocks
            .iter()
            .map(|b| match b {
                Block::Cover {
                    title,
                    project,
                    lines,
                } => json!({ "k": "cover", "title": title, "project": project, "lines": lines }),
                Block::Heading(level, text) => json!({ "k": "h", "level": level, "text": text }),
                Block::Para(text) => json!({ "k": "p", "text": text }),
                Block::Table { columns, rows } => json!({
                    "k": "table",
                    "columns": columns.iter().map(|(t, w)| json!([t, w])).collect::<Vec<_>>(),
                    "rows": rows,
                }),
                Block::Check { status, text } => {
                    json!({ "k": "check", "status": status_name(*status), "text": text })
                }
                Block::PageBreak => json!({ "k": "break" }),
            })
            .collect(),
    )
}

pub fn blocks_from_json(v: &Value) -> Vec<Block> {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|a| a.iter().map(s).collect())
            .unwrap_or_default()
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|b| {
                    Some(match b["k"].as_str()? {
                        "cover" => Block::Cover {
                            title: s(&b["title"]),
                            project: s(&b["project"]),
                            lines: strings(&b["lines"]),
                        },
                        "h" => {
                            Block::Heading(b["level"].as_u64().unwrap_or(1) as u8, s(&b["text"]))
                        }
                        "p" => Block::Para(s(&b["text"])),
                        "table" => Block::Table {
                            columns: b["columns"]
                                .as_array()
                                .map(|c| {
                                    c.iter()
                                        .map(|c| (s(&c[0]), c[1].as_f64().unwrap_or(1.0)))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            rows: b["rows"]
                                .as_array()
                                .map(|r| r.iter().map(strings).collect())
                                .unwrap_or_default(),
                        },
                        "check" => Block::Check {
                            status: status_of(b["status"].as_str().unwrap_or_default()),
                            text: s(&b["text"]),
                        },
                        _ => Block::PageBreak,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A calculation being worked out aside, for a revision of the model.
pub struct Aside {
    pub revision: u64,
    pub calc: Calc,
    slot: Arc<Mutex<Option<Result<Outcome, String>>>>,
}

impl Aside {
    /// Starts it: on another thread on the desktop, in a worker in the
    /// browser (here, at once, in the tests).
    pub fn start(calc: Calc, model: Model, revision: u64, ctx: &eframe::egui::Context) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        if cfg!(test) {
            *slot.lock().expect("new") = Some(Ok(calculate(calc, &model)));
        } else {
            #[cfg(target_arch = "wasm32")]
            crate::web::calc_aside(calc, &model, out, ctx.clone());
            #[cfg(not(target_arch = "wasm32"))]
            {
                let ctx = ctx.clone();
                crate::platform::spawn(move || {
                    let r = calculate(calc, &model);
                    if let Ok(mut o) = out.lock() {
                        *o = Some(Ok(r));
                    }
                    ctx.request_repaint();
                });
            }
        }
        Aside {
            revision,
            calc,
            slot,
        }
    }

    /// The result, once (`None` while it is being worked out).
    pub fn take(&self) -> Option<Result<Outcome, String>> {
        self.slot.lock().ok()?.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_go_and_come_back_as_text() {
        let blocks = vec![
            Block::Cover {
                title: "Anejo".into(),
                project: "Casa".into(),
                lines: vec!["a".into(), "b".into()],
            },
            Block::Heading(2, "Vigas".into()),
            Block::Para("Texto".into()),
            Block::Table {
                columns: vec![("Viga".into(), 2.0), ("Flecha".into(), 1.0)],
                rows: vec![vec!["V1".into(), "L/400".into()]],
            },
            Block::Check {
                status: Status::Fail,
                text: "No cumple".into(),
            },
            Block::PageBreak,
        ];
        let json = json!({ "text": "t", "n": 3, "blocks": blocks_to_json(&blocks) }).to_string();
        let (text, n, back) = read(&json).unwrap();
        assert_eq!((text.as_str(), n), ("t", 3));
        assert_eq!(back, blocks);
    }

    /// The demo's structure, worked out as the browser's worker does, is
    /// what the window works out.
    #[test]
    fn a_calculation_as_text_is_the_calculation() {
        // The demo of `archisim demo out/`, if it has been made.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/demo.archisim");
        let Ok(m) = archisim_file::open(&path) else {
            return;
        };
        for calc in [Calc::Structure, Calc::Electrical] {
            let here = calculate(calc, &m);
            let there = read(&run(calc, &m)).unwrap();
            assert_eq!(here, there, "{}", calc.name());
        }
    }
}
