//! Coordination with the other disciplines, as the window keeps it: linked
//! IFC models (shown in plan and 3D, not imported), the issues file (BCF),
//! the last clash check and the last requirements check (IDS). No user
//! interface here: `app::coord_ui` draws it.
//!
//! What is linked and which files are open is kept beside the project, in
//! `<proyecto>.coordinacion.txt` (one line per item, paths relative to the
//! project), so the `.archisim` format does not change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use archisim_bcf::Bcf;
use archisim_coord::{
    compare, find_clashes, read_elements, Change, ClashOptions, Element as LinkedElement,
    ModelElements, VersionReport,
};
use archisim_core::Model;

use crate::pick::Element;

/// Colours of linked models, by discipline (sRGB).
pub fn discipline_color(d: archisim_coord::Discipline) -> [u8; 3] {
    use archisim_coord::Discipline::*;
    match d {
        Architecture => [176, 160, 140],
        Structure => [120, 150, 190],
        Mechanical => [110, 180, 120],
        Plumbing => [90, 160, 200],
        Electrical => [220, 180, 70],
        Site => [130, 170, 100],
        Other => [170, 170, 175],
    }
}

/// A segment of a linked model cut in plan, with its colour.
pub type PlanSegment = ([[f64; 2]; 2], [u8; 3]);

/// Colours that mark clashes and changes.
pub const CLASH_RED: [u8; 3] = [220, 40, 40];
pub const CLASH_AMBER: [u8; 3] = [245, 170, 20];
pub const NEW_GREEN: [u8; 3] = [40, 170, 70];
pub const CHANGED_ORANGE: [u8; 3] = [240, 130, 30];

/// One linked IFC file.
pub struct Link {
    pub path: PathBuf,
    pub elements: ModelElements,
    pub visible: bool,
    pub transparent: bool,
    pub color: [u8; 3],
    /// When the file was read, to notice a new version.
    modified: Option<SystemTime>,
    /// What changed the last time it was read again.
    pub changes: Option<VersionReport>,
    /// Paint new and changed elements (green and orange).
    pub show_changes: bool,
}

impl Link {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn load(path: &Path) -> Result<(ModelElements, Option<SystemTime>), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("no se puede leer {}: {e}", path.display()))?;
        let elements = read_elements(&text)?;
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        Ok((elements, modified))
    }
}

/// A clash found by the last check.
#[derive(Debug, Clone)]
pub struct ClashRow {
    /// The project's element (by GlobalId) and the linked one.
    pub ours: String,
    pub ours_label: String,
    pub link: usize,
    pub theirs: usize,
    pub theirs_label: String,
    pub point: [f64; 3],
    pub depth: f64,
    pub storey: Option<String>,
}

/// The issues file open.
pub struct Issues {
    pub path: Option<PathBuf>,
    pub bcf: Bcf,
    pub modified: bool,
    pub selected: Option<usize>,
}

/// The last requirements check.
pub struct Requirements {
    pub path: PathBuf,
    pub report: archisim_ids::IdsReport,
}

#[derive(Default)]
pub struct Coord {
    pub links: Vec<Link>,
    /// Changes whenever anything drawn changes (links, visibility,
    /// highlighted elements), so plan and 3D are drawn again.
    pub serial: u64,
    pub issues: Option<Issues>,
    pub clashes: Vec<ClashRow>,
    /// When the clash check last ran (`None`: never).
    pub clashes_checked: Option<String>,
    pub tolerance_mm: f64,
    pub requirements: Option<Requirements>,
    /// Linked elements painted in a colour (clash, issue), by
    /// (link, element index).
    pub highlight: HashMap<(usize, usize), [u8; 3]>,
    /// Only the elements of a clash or an issue in colour; the rest faint.
    pub isolate: bool,
    /// A linked element clicked in 3D: shown in amber.
    pub selected_linked: Option<(usize, usize)>,
    /// The project these settings belong to.
    project: Option<PathBuf>,
    /// Seconds (app time) of the last check for new versions.
    last_poll: f64,
}

/// One linked element to draw.
pub struct DrawPart<'a> {
    pub link: usize,
    pub index: usize,
    pub element: &'a LinkedElement,
    /// sRGB and opacity.
    pub color: [u8; 3],
    pub alpha: f32,
}

impl Coord {
    pub fn new() -> Coord {
        Coord {
            tolerance_mm: 10.0,
            ..Coord::default()
        }
    }

    fn bump(&mut self) {
        self.serial += 1;
    }

    // ---- Links -------------------------------------------------------

    /// Links an IFC file. Returns a summary for the status bar.
    pub fn add_link(&mut self, path: &Path) -> Result<String, String> {
        if self.links.iter().any(|l| l.path == path) {
            return Err(format!("{} ya está vinculado", path.display()));
        }
        let (elements, modified) = Link::load(path)?;
        let color = discipline_color(elements.main_discipline());
        let summary = format!(
            "Vinculado {}: {} elementos de {}{}.",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            elements.elements.len(),
            elements.main_discipline().name().to_lowercase(),
            if elements.approximate > 0 {
                format!(", {} con geometría aproximada", elements.approximate)
            } else {
                String::new()
            }
        );
        self.links.push(Link {
            path: path.to_path_buf(),
            elements,
            visible: true,
            transparent: false,
            color,
            modified,
            changes: None,
            show_changes: false,
        });
        self.bump();
        Ok(summary)
    }

    pub fn remove_link(&mut self, i: usize) {
        if i < self.links.len() {
            self.links.remove(i);
            self.clashes.retain(|c| c.link != i);
            self.selected_linked = None;
            for c in &mut self.clashes {
                if c.link > i {
                    c.link -= 1;
                }
            }
            self.highlight.clear();
            self.bump();
        }
    }

    pub fn set_visible(&mut self, i: usize, v: bool) {
        if let Some(l) = self.links.get_mut(i) {
            l.visible = v;
            self.bump();
        }
    }

    pub fn set_transparent(&mut self, i: usize, v: bool) {
        if let Some(l) = self.links.get_mut(i) {
            l.transparent = v;
            self.bump();
        }
    }

    pub fn set_show_changes(&mut self, i: usize, v: bool) {
        if let Some(l) = self.links.get_mut(i) {
            l.show_changes = v;
            self.bump();
        }
    }

    /// Marks a linked element (clicked in 3D) and says what it is.
    pub fn select_linked(&mut self, link: usize, index: usize) -> Option<String> {
        let l = self.links.get(link)?;
        let e = l.elements.elements.get(index)?;
        let text = format!(
            "{} · {}{} · {}",
            l.name(),
            element_label(e),
            e.storey
                .as_deref()
                .map(|s| format!(" · {s}"))
                .unwrap_or_default(),
            e.global_id
        );
        self.selected_linked = Some((link, index));
        self.bump();
        Some(text)
    }

    /// Forgets the marked linked element; whether there was one.
    pub fn select_linked_none(&mut self) -> bool {
        let had = self.selected_linked.take().is_some();
        if had {
            self.bump();
        }
        had
    }

    pub fn set_isolate(&mut self, on: bool) {
        if self.isolate != on {
            self.isolate = on;
            self.bump();
        }
    }

    /// Highlighted elements were changed directly: draw again.
    pub fn highlight_changed(&mut self) {
        self.bump();
    }

    /// Paints linked elements (those of an issue or a clash) in a colour,
    /// by GlobalId; forgets the previous ones.
    pub fn highlight_guids(&mut self, guids: &[String], color: [u8; 3]) {
        self.highlight.clear();
        for (li, l) in self.links.iter().enumerate() {
            for (ei, e) in l.elements.elements.iter().enumerate() {
                if guids.contains(&e.global_id) {
                    self.highlight.insert((li, ei), color);
                }
            }
        }
        self.bump();
    }

    /// Reads a link again and works out what changed since it was read.
    pub fn reload(&mut self, i: usize) -> Result<String, String> {
        let Some(link) = self.links.get(i) else {
            return Err("ese vínculo ya no existe".into());
        };
        let (elements, modified) = Link::load(&link.path)?;
        let report = compare(&link.elements, &elements);
        let name = link.name();
        let summary = summary_of(&report);
        let link = &mut self.links[i];
        link.elements = elements;
        link.modified = modified;
        link.show_changes = !report.changes.is_empty();
        link.changes = Some(report);
        // Clash results pointed into the old elements.
        self.clashes.retain(|c| c.link != i);
        self.highlight.clear();
        self.selected_linked = None;
        self.bump();
        Ok(format!("{name} recargado: {summary}."))
    }

    /// Reads again, at most every few seconds, the links whose file has
    /// changed on disk. Returns what it says to the person.
    pub fn poll(&mut self, now: f64) -> Vec<Result<String, String>> {
        if now - self.last_poll < 3.0 {
            return Vec::new();
        }
        self.last_poll = now;
        let stale: Vec<usize> = self
            .links
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                let m = std::fs::metadata(&l.path).and_then(|m| m.modified()).ok();
                m.is_some() && m != l.modified
            })
            .map(|(i, _)| i)
            .collect();
        stale.into_iter().map(|i| self.reload(i)).collect()
    }

    /// The linked elements to draw, with their colour: the link's, or
    /// the colour of a clash or a change.
    pub fn parts(&self) -> Vec<DrawPart<'_>> {
        let mut out = Vec::new();
        for (li, l) in self.links.iter().enumerate() {
            if !l.visible {
                continue;
            }
            let changed: HashMap<&str, [u8; 3]> = if l.show_changes {
                l.changes
                    .iter()
                    .flat_map(|r| &r.changes)
                    .filter_map(|c| match c.change {
                        Change::Added => Some((c.global_id.as_str(), NEW_GREEN)),
                        Change::Modified { .. } => Some((c.global_id.as_str(), CHANGED_ORANGE)),
                        Change::Removed => None,
                    })
                    .collect()
            } else {
                HashMap::new()
            };
            for (ei, e) in l.elements.elements.iter().enumerate() {
                let hl = if self.selected_linked == Some((li, ei)) {
                    Some(CLASH_AMBER)
                } else {
                    self.highlight.get(&(li, ei)).copied()
                };
                let color = hl
                    .or_else(|| changed.get(e.global_id.as_str()).copied())
                    .unwrap_or(l.color);
                let alpha = if l.transparent && hl.is_none() {
                    0.35
                } else {
                    1.0
                };
                out.push(DrawPart {
                    link: li,
                    index: ei,
                    element: e,
                    color,
                    alpha,
                });
            }
        }
        out
    }

    /// Section of the visible links by a horizontal plane at `z`: line
    /// segments in plan, with their colour, for the plan view.
    pub fn plan_cut(&self, z: f64) -> Vec<PlanSegment> {
        let mut out = Vec::new();
        for part in self.parts() {
            let Some((lo, hi)) = part.element.bounds else {
                continue;
            };
            if lo[2] > z || hi[2] < z {
                continue;
            }
            for t in &part.element.mesh.triangles {
                if let Some(seg) = cut_triangle(t, z) {
                    out.push((seg, part.color));
                }
            }
        }
        out
    }

    // ---- Clashes -----------------------------------------------------

    /// Checks the project (as it is exported to IFC) against every
    /// visible link.
    pub fn check_clashes(&mut self, model: &Model) -> Result<String, String> {
        if !self.links.iter().any(|l| l.visible) {
            return Err("No hay ningún modelo vinculado visible con el que comprobar.".into());
        }
        let ours = project_elements(model)?;
        let opts = ClashOptions {
            tolerance: self.tolerance_mm.max(0.5) / 1000.0,
            ..ClashOptions::default()
        };
        let mut rows = Vec::new();
        for (li, l) in self.links.iter().enumerate() {
            if !l.visible {
                continue;
            }
            for c in find_clashes(&ours, &l.elements, &opts) {
                let a = &ours.elements[c.a];
                let b = &l.elements.elements[c.b];
                rows.push(ClashRow {
                    ours: a.global_id.clone(),
                    ours_label: element_label(a),
                    link: li,
                    theirs: c.b,
                    theirs_label: element_label(b),
                    point: c.point,
                    depth: c.depth,
                    storey: a.storey.clone().or_else(|| b.storey.clone()),
                });
            }
        }
        rows.sort_by(|x, y| y.depth.total_cmp(&x.depth));
        let n = rows.len();
        self.clashes = rows;
        self.clashes_checked = Some(archisim_bcf::now());
        self.highlight.clear();
        for c in &self.clashes {
            self.highlight.insert((c.link, c.theirs), CLASH_RED);
        }
        self.bump();
        Ok(match n {
            0 => "Sin interferencias con los modelos vinculados.".into(),
            1 => "1 interferencia con los modelos vinculados.".into(),
            n => format!("{n} interferencias con los modelos vinculados."),
        })
    }

    /// Puts the clashes of the last check in the issues file (a new one if
    /// none is open), keeping the history of each pair.
    pub fn clashes_to_issues(&mut self, model: &Model, author: &str) -> Result<String, String> {
        let ours = project_elements(model)?;
        let opts = ClashOptions {
            tolerance: self.tolerance_mm.max(0.5) / 1000.0,
            ..ClashOptions::default()
        };
        let issues = self.issues.get_or_insert_with(|| Issues {
            path: None,
            bcf: Bcf::default(),
            modified: true,
            selected: None,
        });
        let (mut added, mut kept, mut resolved) = (0, 0, 0);
        for l in self.links.iter().filter(|l| l.visible) {
            let found = find_clashes(&ours, &l.elements, &opts);
            let files = [
                file_ref(&format!("{}.ifc", model.project_name)),
                file_ref(&l.name()),
            ];
            let (a, k, r) = archisim_coord::clashes_to_bcf(
                &mut issues.bcf,
                &found,
                &ours,
                &l.elements,
                &files,
                &archisim_coord::Author(author.to_string()),
            );
            added += a;
            kept += k;
            resolved += r;
        }
        issues.modified = true;
        Ok(format!(
            "Incidencias: {added} nuevas, {kept} siguen abiertas, {resolved} resueltas."
        ))
    }

    // ---- Issues ------------------------------------------------------

    pub fn open_issues(&mut self, path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let bcf = archisim_bcf::read_bcf(&bytes).map_err(|e| e.to_string())?;
        let n = bcf.topics.len();
        self.issues = Some(Issues {
            path: Some(path.to_path_buf()),
            bcf,
            modified: false,
            selected: None,
        });
        Ok(format!("{n} incidencias en {}.", path.display()))
    }

    pub fn save_issues(&mut self, path: Option<&Path>) -> Result<String, String> {
        let Some(issues) = &mut self.issues else {
            return Err("No hay incidencias que guardar.".into());
        };
        let path = match (path, &issues.path) {
            (Some(p), _) => p.to_path_buf(),
            (None, Some(p)) => p.clone(),
            (None, None) => return Err("Falta elegir dónde guardar las incidencias.".into()),
        };
        std::fs::write(&path, archisim_bcf::write_bcf(&issues.bcf))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        issues.path = Some(path.clone());
        issues.modified = false;
        Ok(format!("Incidencias guardadas en {}.", path.display()))
    }

    // ---- Requirements -----------------------------------------------

    pub fn check_requirements(&mut self, model: &Model, path: &Path) -> Result<String, String> {
        let xml = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let ifc = archisim_ifc::export_ifc(model, &archisim_ifc::ExportOptions::default());
        let report = archisim_ids::check_texts(&xml, &ifc)?;
        let ok = report.specifications.iter().filter(|s| s.passed).count();
        let total = report.specifications.len();
        self.requirements = Some(Requirements {
            path: path.to_path_buf(),
            report,
        });
        Ok(format!(
            "Requisitos: cumple {ok} de {total} especificaciones."
        ))
    }

    /// The failures of the requirements check as issues (added to the open
    /// issues file, or a new one).
    pub fn requirements_to_issues(
        &mut self,
        model: &Model,
        author: &str,
    ) -> Result<String, String> {
        let Some(req) = &self.requirements else {
            return Err("Comprueba primero los requisitos.".into());
        };
        let new = archisim_coord::ids_to_bcf(
            &req.report,
            &file_ref(&format!("{}.ifc", model.project_name)),
            &archisim_coord::Author(author.to_string()),
        );
        let n = new.topics.len();
        let issues = self.issues.get_or_insert_with(|| Issues {
            path: None,
            bcf: Bcf {
                extensions: archisim_bcf::Extensions::archisim(),
                ..Bcf::default()
            },
            modified: true,
            selected: None,
        });
        let mut index = issues
            .bcf
            .topics
            .iter()
            .filter_map(|t| t.index)
            .max()
            .unwrap_or(0);
        for mut t in new.topics {
            index += 1;
            t.index = Some(index);
            issues.bcf.topics.push(t);
        }
        issues.modified = true;
        Ok(format!("{n} incidencias de requisitos añadidas."))
    }

    // ---- Kept beside the project -----------------------------------

    fn sidecar(project: &Path) -> PathBuf {
        let stem = project
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        project.with_file_name(format!("{stem}.coordinacion.txt"))
    }

    /// A project was opened (or a new one started): its links and files.
    /// Returns problems to tell (a link whose file is missing…).
    pub fn project_changed(&mut self, project: Option<&Path>) -> Vec<String> {
        *self = Coord::new();
        self.project = project.map(Path::to_path_buf);
        let Some(project) = project else {
            return Vec::new();
        };
        let Ok(text) = std::fs::read_to_string(Self::sidecar(project)) else {
            return Vec::new();
        };
        let base = project.parent().unwrap_or(Path::new("."));
        let mut problems = Vec::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            let at = |rel: &str| base.join(rel);
            match f.as_slice() {
                ["vinculo", rel, rest @ ..] => {
                    let path = at(rel);
                    match self.add_link(&path) {
                        Ok(_) => {
                            let i = self.links.len() - 1;
                            let l = &mut self.links[i];
                            l.visible = rest.first() != Some(&"0");
                            l.transparent = rest.get(1) == Some(&"1");
                            if let Some(c) = rest.get(2).and_then(|h| parse_hex(h)) {
                                l.color = c;
                            }
                        }
                        Err(e) => problems.push(format!("Vínculo sin abrir: {e}")),
                    }
                }
                ["incidencias", rel] => {
                    if let Err(e) = self.open_issues(&at(rel)) {
                        problems.push(format!("Incidencias sin abrir: {e}"));
                    }
                }
                ["tolerancia", mm] => {
                    if let Ok(v) = mm.parse() {
                        self.tolerance_mm = v;
                    }
                }
                _ => {}
            }
        }
        problems
    }

    /// Writes the links and files beside a project just saved (removes
    /// the file when there is nothing to keep).
    pub fn save_beside(&mut self, project: &Path) -> std::io::Result<()> {
        self.project = Some(project.to_path_buf());
        let base = project.parent().unwrap_or(Path::new("."));
        let rel = |p: &Path| -> String {
            p.strip_prefix(base)
                .unwrap_or(p)
                .to_string_lossy()
                .replace('\\', "/")
        };
        let mut s = String::from(
            "# ArchiSim: modelos vinculados e incidencias de este proyecto.\n\
             # Una línea por elemento; rutas relativas a la carpeta del proyecto.\n",
        );
        let mut any = false;
        for l in &self.links {
            any = true;
            s += &format!(
                "vinculo\t{}\t{}\t{}\t{:02X}{:02X}{:02X}\n",
                rel(&l.path),
                u8::from(l.visible),
                u8::from(l.transparent),
                l.color[0],
                l.color[1],
                l.color[2]
            );
        }
        if let Some(p) = self.issues.as_ref().and_then(|i| i.path.as_ref()) {
            any = true;
            s += &format!("incidencias\t{}\n", rel(p));
        }
        if (self.tolerance_mm - 10.0).abs() > 1e-9 {
            any = true;
            s += &format!("tolerancia\t{}\n", self.tolerance_mm);
        }
        let file = Self::sidecar(project);
        if any {
            std::fs::write(file, s)
        } else {
            match std::fs::remove_file(file) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }
    }
}

fn parse_hex(h: &str) -> Option<[u8; 3]> {
    let v = u32::from_str_radix(h, 16).ok()?;
    (h.len() == 6).then_some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn summary_of(r: &VersionReport) -> String {
    let added = r.count(|c| *c == Change::Added);
    let removed = r.count(|c| *c == Change::Removed);
    let modified = r.changes.len() - added - removed;
    if r.changes.is_empty() {
        "sin cambios".into()
    } else {
        format!("{added} nuevos, {removed} borrados, {modified} modificados")
    }
}

fn file_ref(name: &str) -> archisim_bcf::FileRef {
    archisim_bcf::FileRef {
        is_external: true,
        filename: Some(name.to_string()),
        date: Some(archisim_bcf::now()),
        ..Default::default()
    }
}

/// The project as coordination sees it: its own IFC export, read back.
pub fn project_elements(model: &Model) -> Result<ModelElements, String> {
    let ifc = archisim_ifc::export_ifc(model, &archisim_ifc::ExportOptions::default());
    read_elements(&ifc)
}

pub fn element_label(e: &LinkedElement) -> String {
    let name = if e.name.is_empty() {
        &e.type_name
    } else {
        &e.name
    };
    let class = archisim_ids::model::class_display(&e.class);
    if name.is_empty() {
        class
    } else {
        format!("{class} «{name}»")
    }
}

/// The project's element with a GlobalId (walls, slabs, doors and
/// windows).
pub fn element_by_guid(model: &Model, guid: &str) -> Option<Element> {
    if let Some(w) = model.walls().find(|w| w.guid.as_str() == guid) {
        return Some(Element::Wall(w.id));
    }
    if let Some(s) = model.slabs().find(|s| s.guid.as_str() == guid) {
        return Some(Element::Slab(s.id));
    }
    model
        .openings()
        .find(|o| o.guid.as_str() == guid)
        .map(|o| Element::Opening(o.id))
}

/// The GlobalId of a project element (annotations have none).
pub fn guid_of(model: &Model, e: Element) -> Option<String> {
    match e {
        Element::Wall(id) => model.wall(id).map(|w| w.guid.to_string()),
        Element::Slab(id) => model.slab(id).map(|s| s.guid.to_string()),
        Element::Opening(id) => model.opening(id).map(|o| o.guid.to_string()),
        _ => None,
    }
}

/// Where a horizontal plane cuts a triangle, as a segment in plan.
fn cut_triangle(t: &[[f64; 3]; 3], z: f64) -> Option<[[f64; 2]; 2]> {
    let mut pts = Vec::with_capacity(2);
    for i in 0..3 {
        let (a, b) = (t[i], t[(i + 1) % 3]);
        let (da, db) = (a[2] - z, b[2] - z);
        if (da < 0.0) != (db < 0.0) {
            let f = da / (da - db);
            pts.push([a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]);
        }
    }
    (pts.len() == 2).then(|| [pts[0], pts[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A structural IFC with one beam crossing the room's south wall.
    fn beam_ifc(y0: f64) -> String {
        format!(
            "ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('e.ifc','',(''),(''),'','Prueba','');\
             FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
             #1=IFCPROJECT('0p000000000000000000p1',$,'Estructura',$,$,$,$,$,#2);\
             #2=IFCUNITASSIGNMENT((#3));#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\
             #4=IFCDIRECTION((0.,0.,1.));\
             #10=IFCCARTESIANPOINT((1.,{y0},2.));#11=IFCAXIS2PLACEMENT3D(#10,$,$);#12=IFCLOCALPLACEMENT($,#11);\
             #13=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.3,2.);#14=IFCEXTRUDEDAREASOLID(#13,$,#4,0.4);\
             #15=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#14));#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#15));\
             #20=IFCBEAM('0b000000000000000000b1',$,'Viga',$,$,#12,#16,$,$);\
             ENDSEC;END-ISO-10303-21;"
        )
    }

    #[test]
    fn link_clash_issue_and_reload() {
        let dir = std::env::temp_dir().join(format!("archisim-coord-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link_path = dir.join("estructura.ifc");
        std::fs::write(&link_path, beam_ifc(0.0)).unwrap();
        let (model, _, _) = crate::pick::tests::room();

        let mut c = Coord::new();
        let said = c.add_link(&link_path).unwrap();
        assert!(said.contains("1 elementos de estructura"), "{said}");
        assert!(c.add_link(&link_path).is_err());
        // In plan, at 2.2 m the beam is cut: a rectangle of 4 segments
        // (or more, triangulated sides give two per face).
        assert!(c.plan_cut(2.2).len() >= 4);
        assert!(c.plan_cut(1.0).is_empty());

        // The beam crosses the south wall of the room.
        let said = c.check_clashes(&model).unwrap();
        assert_eq!(c.clashes.len(), 1, "{said}");
        let row = &c.clashes[0];
        assert!(matches!(
            element_by_guid(&model, &row.ours),
            Some(Element::Wall(_))
        ));
        assert_eq!(c.highlight.get(&(0, 0)), Some(&CLASH_RED));
        // As issues, saved and read back.
        c.clashes_to_issues(&model, "arquitecto").unwrap();
        let bcf_path = dir.join("incidencias.bcf");
        c.save_issues(Some(&bcf_path)).unwrap();
        assert_eq!(
            c.open_issues(&bcf_path).unwrap(),
            format!("1 incidencias en {}.", bcf_path.display())
        );

        // Kept beside the project, opened again.
        let project = dir.join("casa.archisim");
        c.tolerance_mm = 5.0;
        c.save_beside(&project).unwrap();
        let mut again = Coord::new();
        assert!(again.project_changed(Some(&project)).is_empty());
        assert_eq!(again.links.len(), 1);
        assert_eq!(again.tolerance_mm, 5.0);
        assert!(again.issues.is_some());

        // The engineer moves the beam out of the wall: reload shows it.
        std::fs::write(&link_path, beam_ifc(2.0)).unwrap();
        let said = again.reload(0).unwrap();
        assert!(
            said.contains("0 nuevos, 0 borrados, 1 modificados"),
            "{said}"
        );
        assert!(again.links[0].show_changes);
        assert_eq!(again.parts()[0].color, CHANGED_ORANGE);
        again.check_clashes(&model).unwrap();
        assert!(again.clashes.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
