//! Clashes as BCF issues: one topic per pair of elements, with a view
//! that frames the clash, both elements selected and coloured. Running the
//! check again on the same BCF keeps the history: pairs still clashing
//! get a comment, pairs that no longer clash are marked resolved, new
//! ones are added.

use std::collections::{HashMap, HashSet};

use archisim_bcf::{now, Bcf, Camera, Comment, Extensions, FileRef, Topic, Viewpoint};
use archisim_ids::model::class_display;

use crate::clash::Clash;
use crate::elements::{Element, ModelElements};
use crate::versions::metres;

pub const CLASH_TYPE: &str = "Interferencia";
const OPEN: &str = "Abierta";
const RESOLVED: &str = "Resuelta";
const CLOSED: &str = "Cerrada";

/// Who is writing (BCF authors are e-mail addresses or names).
#[derive(Debug, Clone)]
pub struct Author(pub String);

fn label(e: &Element) -> String {
    let name = if e.name.is_empty() {
        e.type_name.clone()
    } else {
        e.name.clone()
    };
    if name.is_empty() {
        class_display(&e.class)
    } else {
        format!("{} «{name}»", class_display(&e.class))
    }
}

/// The two GlobalIds of a clash, in a fixed order.
fn pair_key(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}

/// The pair a clash topic is about: the selection of its first view.
fn topic_pair(t: &Topic) -> Option<(String, String)> {
    if t.topic_type.as_deref() != Some(CLASH_TYPE) {
        return None;
    }
    let sel = &t.viewpoints.first()?.selection;
    (sel.len() == 2).then(|| pair_key(&sel[0], &sel[1]))
}

/// A camera looking at a point from above and to the side, far enough to
/// see both elements.
pub fn camera_for(point: [f64; 3], a: &Element, b: &Element) -> Camera {
    let size = [a.bounds, b.bounds]
        .iter()
        .flatten()
        .map(|(lo, hi)| (0..3).map(|i| hi[i] - lo[i]).fold(0.0, f64::max))
        .fold(1.0, f64::max)
        .min(12.0);
    let d = size * 1.2 + 1.5;
    let dir = [-0.5773503, 0.5773503, -0.5773503];
    Camera::Perspective {
        position: [
            point[0] - dir[0] * d,
            point[1] - dir[1] * d,
            point[2] - dir[2] * d,
        ],
        direction: dir,
        up: [0.0, 0.0, 1.0],
        field_of_view: 60.0,
        aspect_ratio: 1.6,
    }
}

/// Updates (or fills a new) BCF with the clashes found between `left`
/// and `right`. Returns how many topics are new, still open and resolved.
pub fn clashes_to_bcf(
    bcf: &mut Bcf,
    clashes: &[Clash],
    left: &ModelElements,
    right: &ModelElements,
    files: &[FileRef],
    author: &Author,
) -> (usize, usize, usize) {
    if bcf.extensions == Extensions::default() {
        bcf.extensions = Extensions::archisim();
    }
    let date = now();
    let mut existing: HashMap<(String, String), usize> = HashMap::new();
    for (i, t) in bcf.topics.iter().enumerate() {
        if let Some(k) = topic_pair(t) {
            existing.insert(k, i);
        }
    }
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let (mut added, mut kept, mut resolved) = (0, 0, 0);
    let mut next_index = bcf.topics.iter().filter_map(|t| t.index).max().unwrap_or(0) + 1;
    for c in clashes {
        let (ea, eb) = (&left.elements[c.a], &right.elements[c.b]);
        let key = pair_key(&ea.global_id, &eb.global_id);
        if !seen.insert(key.clone()) {
            continue;
        }
        let storey = ea.storey.as_ref().or(eb.storey.as_ref());
        let text = format!(
            "{} penetra {} en {}{}.",
            label(ea),
            metres(c.depth),
            label(eb),
            storey.map(|s| format!(" ({s})")).unwrap_or_default()
        );
        if let Some(&i) = existing.get(&key) {
            let t = &mut bcf.topics[i];
            let was = t.status.clone().unwrap_or_default();
            if was == RESOLVED || was == CLOSED {
                t.status = Some(OPEN.into());
                t.comments
                    .push(comment(&date, author, format!("Vuelve a aparecer: {text}")));
            } else {
                t.comments
                    .push(comment(&date, author, format!("Sigue: {text}")));
            }
            t.modified_date = Some(date.clone());
            t.modified_author = Some(author.0.clone());
            kept += 1;
            continue;
        }
        let mut vp = Viewpoint::new(archisim_bcf::new_guid());
        vp.camera = Some(camera_for(c.point, ea, eb));
        vp.selection = vec![ea.global_id.clone(), eb.global_id.clone()];
        vp.coloring = vec![
            ("FF0000".into(), vec![ea.global_id.clone()]),
            ("FFC000".into(), vec![eb.global_id.clone()]),
        ];
        let labels: Vec<String> = [ea.discipline, eb.discipline]
            .iter()
            .map(|d| d.name().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        bcf.topics.push(Topic {
            guid: archisim_bcf::new_guid(),
            topic_type: Some(CLASH_TYPE.into()),
            status: Some(OPEN.into()),
            title: format!("{} × {}", label(ea), label(eb)),
            priority: Some(if c.depth > 0.1 { "Alta" } else { "Normal" }.into()),
            index: Some(next_index),
            labels,
            creation_date: date.clone(),
            creation_author: author.0.clone(),
            description: Some(format!(
                "{text}\nPunto: X {:.3}, Y {:.3}, Z {:.3}.",
                c.point[0], c.point[1], c.point[2]
            )),
            files: files.to_vec(),
            viewpoints: vec![vp],
            ..Topic::default()
        });
        next_index += 1;
        added += 1;
    }
    // Clash topics between these two models no longer found.
    let in_models: HashSet<&str> = left
        .elements
        .iter()
        .chain(&right.elements)
        .map(|e| e.global_id.as_str())
        .collect();
    for (key, &i) in &existing {
        if seen.contains(key) {
            continue;
        }
        let t = &mut bcf.topics[i];
        let ours = in_models.contains(key.0.as_str()) || in_models.contains(key.1.as_str());
        let open = !matches!(t.status.as_deref(), Some(RESOLVED) | Some(CLOSED));
        if ours && open {
            t.status = Some(RESOLVED.into());
            t.modified_date = Some(date.clone());
            t.modified_author = Some(author.0.clone());
            t.comments.push(comment(
                &date,
                author,
                "Ya no se detecta: resuelta al comprobar de nuevo.".into(),
            ));
            resolved += 1;
        }
    }
    (added, kept, resolved)
}

fn comment(date: &str, author: &Author, text: String) -> Comment {
    Comment {
        guid: archisim_bcf::new_guid(),
        date: date.to_string(),
        author: author.0.clone(),
        text,
        viewpoint: None,
        modified_date: None,
        modified_author: None,
    }
}

/// A BCF with the failures of an IDS check: one topic per specification
/// that fails, with its failing elements selected.
pub fn ids_to_bcf(report: &archisim_ids::IdsReport, model_file: &FileRef, author: &Author) -> Bcf {
    let date = now();
    let mut bcf = Bcf {
        extensions: Extensions::archisim(),
        ..Bcf::default()
    };
    for (i, s) in report
        .specifications
        .iter()
        .filter(|s| !s.passed)
        .enumerate()
    {
        let mut vp = Viewpoint::new(archisim_bcf::new_guid());
        vp.selection = s
            .failures
            .iter()
            .filter_map(|f| f.global_id.clone())
            .collect();
        vp.coloring = vec![("FF0000".into(), vp.selection.clone())];
        let mut description = String::new();
        if let Some(n) = &s.note {
            description += n;
            description.push('\n');
        }
        for f in s.failures.iter().take(50) {
            description += &format!("· {}: {}\n", f.element, f.reasons.join("; "));
        }
        if s.failures.len() > 50 {
            description += &format!("· … y {} más\n", s.failures.len() - 50);
        }
        bcf.topics.push(Topic {
            guid: archisim_bcf::new_guid(),
            topic_type: Some("Requisito".into()),
            status: Some(OPEN.into()),
            title: format!("No cumple: {}", s.name),
            priority: Some("Normal".into()),
            index: Some(i as i64 + 1),
            creation_date: date.clone(),
            creation_author: author.0.clone(),
            description: Some(description.trim_end().to_string()),
            files: vec![model_file.clone()],
            viewpoints: if vp.selection.is_empty() {
                vec![]
            } else {
                vec![vp]
            },
            ..Topic::default()
        });
    }
    bcf
}
