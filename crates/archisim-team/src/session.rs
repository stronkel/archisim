//! A person's session on a shared project.

use crate::diff::{self, Op};
use crate::folder::*;
use archisim_core::Model;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Says whether an element is inside a reserved zone. Zones are drawn in
/// plan, so only something that knows the elements' geometry (the app)
/// can tell; the elements are looked up in the model being checked.
pub trait Locator {
    fn in_zone(&self, id: u64, zone: &Zone) -> bool;
}

/// For when no zone is reserved (or geometry does not matter).
pub struct NoZones;

impl Locator for NoZones {
    fn in_zone(&self, _: u64, _: &Zone) -> bool {
        false
    }
}

/// What a publish or receive did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Changes received: who, their note and how many elements.
    pub received: Vec<(String, String, usize)>,
    /// Elements published (0: nothing to publish).
    pub published: usize,
    /// Own changes left out: elements someone else reserved first, or that
    /// no longer fit (an element added to a wall someone removed).
    pub dropped: Vec<String>,
    /// Changes of others that could not be applied (the same on every
    /// computer, so everybody sees the same project).
    pub skipped: Vec<String>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.published > 0 {
            parts.push(if self.published == 1 {
                "publicado 1 elemento".to_string()
            } else {
                format!("publicados {} elementos", self.published)
            });
        }
        if !self.received.is_empty() {
            let n: usize = self.received.iter().map(|r| r.2).sum();
            let who: BTreeSet<&str> = self.received.iter().map(|r| r.0.as_str()).collect();
            let k = self.received.len();
            parts.push(format!(
                "{} de {} ({} elemento{})",
                if k == 1 {
                    "recibido 1 cambio".to_string()
                } else {
                    format!("recibidos {k} cambios")
                },
                who.into_iter().collect::<Vec<_>>().join(", "),
                n,
                if n == 1 { "" } else { "s" }
            ));
        }
        if parts.is_empty() {
            parts.push("nada nuevo".into());
        }
        write!(f, "{}", capitalise(&parts.join("; ")))?;
        write!(f, ".")?;
        for d in &self.dropped {
            write!(f, " {d}.")?;
        }
        for s in &self.skipped {
            write!(f, " {s}.")?;
        }
        Ok(())
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// What changed in the folder since the last look.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Refresh {
    /// Someone published.
    pub changes: bool,
    /// Someone reserved or released.
    pub reservations: bool,
    /// Someone moved, came or left.
    pub presence: bool,
    /// A new message.
    pub messages: bool,
    pub members: bool,
}

impl Refresh {
    pub fn any(&self) -> bool {
        self.changes || self.reservations || self.presence || self.messages || self.members
    }
}

/// A person's session on a shared project folder.
pub struct Session {
    pub root: PathBuf,
    pub team: TeamFile,
    pub me: Member,
    pub machine: String,
    members: Vec<Member>,
    base: Value,
    changes: BTreeMap<String, ChangeFile>,
    reservations: BTreeMap<String, ReservationFile>,
    presence: BTreeMap<String, Presence>,
    messages: BTreeMap<String, MessageFile>,
    state: WorkState,
    received: Value,
    stamps: BTreeMap<PathBuf, (u64, Option<SystemTime>)>,
}

/// The project file of a model, as JSON.
pub fn doc_of(model: &Model) -> Value {
    serde_json::from_str(&archisim_file::to_string(model)).unwrap_or(Value::Null)
}

/// The model of a project file.
pub fn model_of(doc: &Value) -> Result<Model, String> {
    archisim_file::from_str(&doc.to_string()).map_err(|e| e.to_string())
}

/// Whether a folder is a shared project (has `equipo.asequipo`).
pub fn is_team_folder(dir: &Path) -> bool {
    dir.join(TEAM_FILE).is_file()
}

/// The shared folder of a path: the folder itself, or the folder of an
/// `equipo.asequipo` file.
pub fn team_folder_of(path: &Path) -> Option<PathBuf> {
    if path.is_dir() && is_team_folder(path) {
        return Some(path.to_path_buf());
    }
    let is_file = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION));
    if is_file {
        return path.parent().map(Path::to_path_buf);
    }
    None
}

fn read_team(root: &Path) -> Result<TeamFile, String> {
    let team: TeamFile = read_json(&root.join(TEAM_FILE))?.ok_or_else(|| {
        format!(
            "{} no es un proyecto compartido (falta {TEAM_FILE})",
            shown(root)
        )
    })?;
    if team.format != FORMAT {
        return Err(format!("{TEAM_FILE} no es de ArchiSim"));
    }
    if team.version > VERSION {
        return Err(
            "el proyecto compartido es de una versión más nueva de ArchiSim: actualiza el programa"
                .into(),
        );
    }
    Ok(team)
}

/// The people of a shared project, in the order they joined.
pub fn members(root: &Path) -> Result<Vec<Member>, String> {
    read_team(root)?;
    let mut out = Vec::new();
    for slug in slugs_in(root, MEMBERS) {
        if let Some(m) = read_json::<Member>(&file_of(root, MEMBERS, &slug))? {
            out.push(m);
        }
    }
    out.sort_by_key(|m| (m.joined, m.slug.clone()));
    Ok(out)
}

impl Session {
    /// Shares a project: writes it into `root` (an empty folder, or one
    /// without a shared project) as the base everybody starts from, with
    /// `name` as its first member. Returns the session and the model to
    /// work on.
    pub fn create(
        root: &Path,
        model: &Model,
        name: &str,
        machine: &str,
    ) -> Result<(Session, Model), String> {
        if is_team_folder(root) {
            return Err(format!(
                "{} ya es un proyecto compartido: ábrelo",
                shown(root)
            ));
        }
        if name.trim().is_empty() {
            return Err("falta tu nombre".into());
        }
        std::fs::create_dir_all(root)
            .map_err(|e| format!("no se puede crear {}: {e}", shown(root)))?;
        write_atomic(&root.join(BASE), archisim_file::to_string(model).as_bytes())?;
        let team = TeamFile {
            format: FORMAT.into(),
            version: VERSION,
            name: model.project_name.clone(),
            created: now(),
            created_by: name.trim().into(),
        };
        // The marker last: the folder is a shared project once it is whole.
        write_json(&root.join(TEAM_FILE), &team)?;
        let (session, model, _) = Session::open(root, name, machine)?;
        Ok((session, model))
    }

    /// Opens a shared project as `name` (joining it if they are new).
    /// Returns the session, the model to work on (their working copy, with
    /// whatever they had not published) and what was received.
    pub fn open(
        root: &Path,
        name: &str,
        machine: &str,
    ) -> Result<(Session, Model, Report), String> {
        let team = read_team(root)?;
        let base_text = std::fs::read_to_string(root.join(BASE))
            .map_err(|e| format!("no se puede leer {BASE}: {e}"))?;
        let base_model = archisim_file::from_str(&base_text).map_err(|e| format!("{BASE}: {e}"))?;
        let base = doc_of(&base_model);
        let slug = slug(name);
        let mut s = Session {
            root: root.to_path_buf(),
            team,
            me: Member {
                slug: slug.clone(),
                name: name.trim().into(),
                color: COLORS[0].into(),
                slot: 1,
                joined: now(),
            },
            machine: machine.into(),
            members: Vec::new(),
            base,
            changes: BTreeMap::new(),
            reservations: BTreeMap::new(),
            presence: BTreeMap::new(),
            messages: BTreeMap::new(),
            state: WorkState::default(),
            received: Value::Null,
            stamps: BTreeMap::new(),
        };
        s.reload_all()?;
        match s.members.iter().find(|m| m.slug == slug) {
            Some(m) => s.me = m.clone(),
            None => s.join()?,
        }
        s.state = read_json(&file_of(root, WORK, &slug))?.unwrap_or_default();
        let (received, skipped) = s.build(&s.state.seen.clone())?;
        s.received = received;
        let work_path = root.join(WORK).join(format!("{slug}.archisim"));
        let working = match std::fs::read_to_string(&work_path) {
            Ok(text) => {
                let m = archisim_file::from_str(&text)
                    .map_err(|e| format!("tu copia de trabajo: {e}"))?;
                doc_of(&m)
            }
            Err(_) => s.received.clone(),
        };
        let mut model = model_of(&working)?;
        s.prepare(&mut model);
        // Catch up with what others published while away.
        let (model, mut report) = s.receive(&model, &NoZones)?;
        report.skipped.splice(0..0, skipped);
        Ok((s, model, report))
    }

    fn join(&mut self) -> Result<(), String> {
        let used_colors: BTreeSet<&str> = self.members.iter().map(|m| m.color.as_str()).collect();
        let color = COLORS
            .iter()
            .find(|c| !used_colors.contains(*c))
            .copied()
            .unwrap_or(COLORS[self.members.len() % COLORS.len()]);
        let taken: BTreeSet<u64> = self.members.iter().map(|m| m.slot).collect();
        let ids = diff::all_ids(&self.base);
        let mut seed = now()
            ^ std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u64::from(d.subsec_nanos()))
            ^ hash(&self.me.slug)
            ^ hash(&self.machine);
        let slot = loop {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let slot = 1 + (seed >> 33) % (MAX_SLOT - 1);
            if !taken.contains(&slot) && !ids.iter().any(|id| id >> BLOCK_BITS == slot) {
                break slot;
            }
        };
        self.me.color = color.into();
        self.me.slot = slot;
        self.me.joined = now();
        write_json(&file_of(&self.root, MEMBERS, &self.me.slug), &self.me)?;
        self.members.push(self.me.clone());
        Ok(())
    }

    /// The project as anyone receives it now (the base and every change
    /// published), without joining: for `archisim equipo` and to check a
    /// shared folder. Also returns the changes left out, if any.
    pub fn snapshot(root: &Path) -> Result<(Session, Model, Vec<String>), String> {
        let team = read_team(root)?;
        let base_text = std::fs::read_to_string(root.join(BASE))
            .map_err(|e| format!("no se puede leer {BASE}: {e}"))?;
        let base_model = archisim_file::from_str(&base_text).map_err(|e| format!("{BASE}: {e}"))?;
        let mut s = Session {
            root: root.to_path_buf(),
            team,
            me: Member {
                slug: String::new(),
                name: String::new(),
                color: COLORS[COLORS.len() - 1].into(),
                slot: 0,
                joined: 0,
            },
            machine: String::new(),
            members: Vec::new(),
            base: doc_of(&base_model),
            changes: BTreeMap::new(),
            reservations: BTreeMap::new(),
            presence: BTreeMap::new(),
            messages: BTreeMap::new(),
            state: WorkState::default(),
            received: Value::Null,
            stamps: BTreeMap::new(),
        };
        s.reload_all()?;
        let latest = s.latest();
        let (doc, skipped) = s.build(&latest)?;
        s.state.seen = latest;
        s.received = doc;
        let model = model_of(&s.received)?;
        Ok((s, model, skipped))
    }

    // ---- Reading the folder -------------------------------------------

    fn reload_all(&mut self) -> Result<(), String> {
        self.stamps.clear();
        self.refresh()?;
        Ok(())
    }

    /// Reads again what others wrote since the last look (only the files
    /// that changed).
    pub fn refresh(&mut self) -> Result<Refresh, String> {
        let mut r = Refresh::default();
        let root = self.root.clone();
        for slug in slugs_in(&root, MEMBERS) {
            let p = file_of(&root, MEMBERS, &slug);
            if self.changed(&p) {
                if let Some(m) = read_json::<Member>(&p)? {
                    match self.members.iter_mut().find(|x| x.slug == m.slug) {
                        Some(x) => *x = m,
                        None => self.members.push(m),
                    }
                    r.members = true;
                }
            }
        }
        self.members.sort_by_key(|m| (m.joined, m.slug.clone()));
        for slug in slugs_in(&root, CHANGES) {
            let p = file_of(&root, CHANGES, &slug);
            if self.changed(&p) {
                if let Some(f) = read_json::<ChangeFile>(&p)? {
                    r.changes |= self.changes.get(&slug) != Some(&f);
                    self.changes.insert(slug, f);
                }
            }
        }
        for slug in slugs_in(&root, RESERVATIONS) {
            let p = file_of(&root, RESERVATIONS, &slug);
            if self.changed(&p) {
                if let Some(f) = read_json::<ReservationFile>(&p)? {
                    r.reservations |= self.reservations.get(&slug) != Some(&f);
                    self.reservations.insert(slug, f);
                }
            }
        }
        for slug in slugs_in(&root, PRESENCE) {
            let p = file_of(&root, PRESENCE, &slug);
            if self.changed(&p) {
                if let Some(f) = read_json::<Presence>(&p)? {
                    r.presence |= self.presence.get(&slug) != Some(&f);
                    self.presence.insert(slug, f);
                }
            }
        }
        for slug in slugs_in(&root, MESSAGES) {
            let p = file_of(&root, MESSAGES, &slug);
            if self.changed(&p) {
                if let Some(f) = read_json::<MessageFile>(&p)? {
                    r.messages |= self.messages.get(&slug) != Some(&f);
                    self.messages.insert(slug, f);
                }
            }
        }
        Ok(r)
    }

    fn changed(&mut self, p: &Path) -> bool {
        let stamp = std::fs::metadata(p)
            .map(|m| (m.len(), m.modified().ok()))
            .unwrap_or((0, None));
        if self.stamps.get(p) == Some(&stamp) {
            return false;
        }
        self.stamps.insert(p.to_path_buf(), stamp);
        true
    }

    // ---- People -----------------------------------------------------------

    pub fn members(&self) -> &[Member] {
        &self.members
    }

    pub fn member(&self, slug: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.slug == slug)
    }

    /// Whether a person has the program open now (seen in the last two
    /// minutes).
    pub fn is_online(&self, slug: &str) -> bool {
        if slug == self.me.slug {
            return true;
        }
        self.presence
            .get(slug)
            .is_some_and(|p| p.open && now().saturating_sub(p.time) < 120)
    }

    pub fn presence_of(&self, slug: &str) -> Option<&Presence> {
        self.presence.get(slug)
    }

    /// Whether the same person seems open on another computer right now.
    pub fn open_elsewhere(&self) -> bool {
        self.presence.get(&self.me.slug).is_some_and(|p| {
            p.open && p.machine != self.machine && now().saturating_sub(p.time) < 120
        })
    }

    /// Says where I am (the level shown, the cursor if shared).
    pub fn touch(
        &mut self,
        level: Option<u64>,
        cursor: Option<[f64; 2]>,
        open: bool,
    ) -> Result<(), String> {
        let p = Presence {
            time: now(),
            machine: self.machine.clone(),
            level,
            cursor,
            open,
        };
        let path = file_of(&self.root, PRESENCE, &self.me.slug);
        write_json(&path, &p)?;
        self.changed(&path);
        self.presence.insert(self.me.slug.clone(), p);
        Ok(())
    }

    // ---- Changes ----------------------------------------------------------

    /// All changes, in the order they apply: by clock, then author.
    fn ordered(&self) -> Vec<(&str, &Change)> {
        let mut all: Vec<(&str, &Change)> = self
            .changes
            .iter()
            .flat_map(|(a, f)| f.changes.iter().map(move |c| (a.as_str(), c)))
            .collect();
        all.sort_by(|x, y| (x.1.clock, x.0, x.1.seq).cmp(&(y.1.clock, y.0, y.1.seq)));
        all
    }

    /// The project with the base and the changes up to `upto` (last seq
    /// of each person). Changes that would break the project are left out,
    /// with a note.
    fn build(&self, upto: &BTreeMap<String, u64>) -> Result<(Value, Vec<String>), String> {
        let list: Vec<(&str, &Change)> = self
            .ordered()
            .into_iter()
            .filter(|(a, c)| upto.get(*a).is_some_and(|s| c.seq <= *s))
            .collect();
        if let Some((a, _)) = list
            .iter()
            .find(|(_, c)| c.version > archisim_file::CURRENT_VERSION)
        {
            return Err(format!(
                "{} publica con una versión más nueva de ArchiSim: actualiza el programa para recibir sus cambios",
                self.name_of(a)
            ));
        }
        let mut doc = self.base.clone();
        for (_, c) in &list {
            diff::apply(&mut doc, &c.ops);
        }
        if model_of(&doc).is_ok() {
            return Ok((doc, Vec::new()));
        }
        // Something does not fit: one change at a time, leaving out those
        // that break the project.
        let mut doc = self.base.clone();
        let mut skipped = Vec::new();
        for (a, c) in &list {
            let mut next = doc.clone();
            diff::apply(&mut next, &c.ops);
            match model_of(&next) {
                Ok(_) => doc = next,
                Err(e) => skipped.push(format!(
                    "No se pudo aplicar «{}» de {} ({e})",
                    c.message,
                    self.name_of(a)
                )),
            }
        }
        Ok((doc, skipped))
    }

    fn name_of(&self, slug: &str) -> String {
        self.member(slug)
            .map_or_else(|| slug.to_string(), |m| m.name.clone())
    }

    /// The latest change of each person.
    fn latest(&self) -> BTreeMap<String, u64> {
        self.changes
            .iter()
            .map(|(a, f)| {
                (
                    a.clone(),
                    f.changes.iter().map(|c| c.seq).max().unwrap_or(0),
                )
            })
            .collect()
    }

    /// Changes others published that I have not received.
    pub fn unreceived(&self) -> usize {
        self.changes
            .iter()
            .filter(|(a, _)| **a != self.me.slug)
            .map(|(a, f)| {
                let seen = self.state.seen.get(a).copied().unwrap_or(0);
                f.changes.iter().filter(|c| c.seq > seen).count()
            })
            .sum()
    }

    /// My changes not yet published.
    pub fn local_ops(&self, model: &Model) -> Vec<Op> {
        diff::diff(&self.received, &doc_of(model))
    }

    /// The project as last received (without my unpublished changes).
    pub fn received(&self) -> &Value {
        &self.received
    }

    /// One more than every clock seen.
    fn next_clock(&self) -> u64 {
        let changes = self
            .changes
            .values()
            .flat_map(|f| f.changes.iter().map(|c| c.clock));
        let claims = self.reservations.values().flat_map(|f| {
            f.elements
                .iter()
                .map(|c| c.clock)
                .chain(f.zones.iter().map(|z| z.clock))
        });
        changes.chain(claims).max().unwrap_or(0) + 1
    }

    /// Makes new elements of `model` take ids from my block.
    pub fn prepare(&self, model: &mut Model) {
        let doc = doc_of(model);
        let next = diff::all_ids(&doc)
            .into_iter()
            .filter(|id| self.me.owns_id(*id))
            .max()
            .map_or(self.me.first_id(), |m| m + 1);
        model.set_next_id(next);
    }

    /// Receives what others published, keeping my unpublished changes on
    /// top. Returns the model to work on.
    pub fn receive(&mut self, model: &Model, loc: &dyn Locator) -> Result<(Model, Report), String> {
        self.sync(model, None, loc)
    }

    /// Receives, then publishes my changes with a note.
    pub fn publish(
        &mut self,
        model: &Model,
        message: &str,
        loc: &dyn Locator,
    ) -> Result<(Model, Report), String> {
        self.sync(model, Some(message), loc)
    }

    fn sync(
        &mut self,
        model: &Model,
        publish: Option<&str>,
        loc: &dyn Locator,
    ) -> Result<(Model, Report), String> {
        self.refresh()?;
        let mine = self.local_ops(model);
        let latest = self.latest();
        let (fresh, skipped) = self.build(&latest)?;
        let mut report = Report {
            skipped,
            ..Default::default()
        };
        for (a, c) in self.ordered() {
            if a == self.me.slug {
                continue;
            }
            let seen = self.state.seen.get(a).copied().unwrap_or(0);
            if c.seq > seen {
                report.received.push((
                    self.name_of(a),
                    c.message.clone(),
                    diff::touched(&c.ops).len(),
                ));
            }
        }
        // My changes on top of what came in, without those on elements
        // someone else reserved first.
        let mut blocked = BTreeSet::new();
        let kept: Vec<Op> = mine
            .into_iter()
            .filter(|op| match op.element() {
                Some(id) => match self.owner(id, loc) {
                    Some(o) if o.slug != self.me.slug => {
                        blocked.insert((id, o.name.clone()));
                        false
                    }
                    _ => true,
                },
                None => true,
            })
            .collect();
        let mut by_owner: BTreeMap<String, usize> = BTreeMap::new();
        for (_, who) in &blocked {
            *by_owner.entry(who.clone()).or_default() += 1;
        }
        for (who, n) in by_owner {
            report.dropped.push(format!(
                "{n} elemento{} que reservó antes {who} se quedan como los tiene {who}",
                if n == 1 { "" } else { "s" }
            ));
        }
        let mut working = fresh.clone();
        diff::apply(&mut working, &kept);
        if model_of(&working).is_err() {
            // One change at a time, leaving out what no longer fits.
            working = fresh.clone();
            let mut lost = 0;
            for op in &kept {
                let mut next = working.clone();
                diff::apply(&mut next, std::slice::from_ref(op));
                if model_of(&next).is_ok() {
                    working = next;
                } else {
                    lost += 1;
                }
            }
            if lost > 0 {
                report.dropped.push(format!(
                    "{lost} cambio{} tuyo{} ya no encaja{} con lo recibido (por ejemplo, algo sobre un elemento que otro ha borrado) y se ha descartado",
                    if lost == 1 { "" } else { "s" },
                    if lost == 1 { "" } else { "s" },
                    if lost == 1 { "" } else { "n" },
                ));
            }
        }
        let mut seen = latest;
        if let Some(message) = publish {
            let ops = diff::diff(&fresh, &working);
            if !ops.is_empty() {
                let file = self
                    .changes
                    .entry(self.me.slug.clone())
                    .or_insert_with(|| ChangeFile {
                        author: self.me.slug.clone(),
                        changes: Vec::new(),
                    });
                let seq = file.changes.iter().map(|c| c.seq).max().unwrap_or(0) + 1;
                report.published = diff::touched(&ops).len().max(1);
                let change = Change {
                    seq,
                    clock: 0,
                    time: now(),
                    message: if message.trim().is_empty() {
                        "Cambios".into()
                    } else {
                        message.trim().into()
                    },
                    version: archisim_file::CURRENT_VERSION,
                    ops,
                };
                let clock = self.next_clock();
                let file = self.changes.get_mut(&self.me.slug).expect("just inserted");
                file.changes.push(Change { clock, ..change });
                let path = file_of(&self.root, CHANGES, &self.me.slug);
                write_json(&path, &*file)?;
                self.changed(&path);
                seen.insert(self.me.slug.clone(), seq);
                // What everybody will build: the base and every change.
                let (built, _) = self.build(&seen)?;
                self.received = built;
            } else {
                self.received = fresh;
            }
        } else {
            self.received = fresh;
        }
        self.state.seen = seen;
        let mut out = model_of(&working)?;
        self.prepare(&mut out);
        self.save_work(&out)?;
        Ok((out, report))
    }

    /// Keeps my working copy (with the changes not yet published) in the
    /// shared folder, under my name.
    pub fn save_work(&mut self, model: &Model) -> Result<(), String> {
        let path = self
            .root
            .join(WORK)
            .join(format!("{}.archisim", self.me.slug));
        write_atomic(&path, archisim_file::to_string(model).as_bytes())?;
        self.state.machine = self.machine.clone();
        self.state.time = now();
        write_json(&file_of(&self.root, WORK, &self.me.slug), &self.state)
    }

    /// Who changed an element and when (received changes, newest first):
    /// (name, time, note).
    pub fn history(&self, id: u64) -> Vec<(String, u64, String)> {
        let mut out: Vec<(String, u64, String)> = self
            .ordered()
            .into_iter()
            .filter(|(a, c)| {
                self.state.seen.get(*a).is_some_and(|s| c.seq <= *s)
                    && c.ops.iter().any(|o| o.element() == Some(id))
            })
            .map(|(a, c)| (self.name_of(a), c.time, c.message.clone()))
            .collect();
        out.reverse();
        out
    }

    /// Published changes, newest first: (name, time, note, elements).
    pub fn log(&self) -> Vec<(String, u64, String, usize)> {
        let mut out: Vec<_> = self
            .ordered()
            .into_iter()
            .map(|(a, c)| {
                (
                    self.name_of(a),
                    c.time,
                    c.message.clone(),
                    diff::touched(&c.ops).len(),
                )
            })
            .collect();
        out.reverse();
        out
    }

    // ---- Reservations -------------------------------------------------

    /// The explicit claims, by element, with the winner of each: whoever
    /// claimed first (by clock, then name).
    fn claims(&self) -> BTreeMap<u64, (u64, &str)> {
        let mut out: BTreeMap<u64, (u64, &str)> = BTreeMap::new();
        for (who, f) in &self.reservations {
            for c in &f.elements {
                let cand = (c.clock, who.as_str());
                match out.get(&c.id) {
                    Some(cur) if *cur <= cand => {}
                    _ => {
                        out.insert(c.id, cand);
                    }
                }
            }
        }
        out
    }

    /// Who has an element: an explicit reservation first, then a zone.
    pub fn owner(&self, id: u64, loc: &dyn Locator) -> Option<&Member> {
        if let Some((_, who)) = self.claims().get(&id) {
            return self.member(who);
        }
        let mut best: Option<(u64, &str)> = None;
        for (who, f) in &self.reservations {
            for z in &f.zones {
                if loc.in_zone(id, z) && best.is_none_or(|b| (z.clock, who.as_str()) < b) {
                    best = Some((z.clock, who.as_str()));
                }
            }
        }
        best.and_then(|(_, who)| self.member(who))
    }

    /// Elements reserved explicitly by each person (winning claims only).
    pub fn reserved_by(&self) -> BTreeMap<String, BTreeSet<u64>> {
        let mut out: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
        for (id, (_, who)) in self.claims() {
            out.entry(who.to_string()).or_default().insert(id);
        }
        out
    }

    /// My own claims that someone else won (we reserved at the same time
    /// and they were first).
    pub fn lost_claims(&self) -> Vec<(u64, String)> {
        let claims = self.claims();
        self.reservations
            .get(&self.me.slug)
            .map(|f| {
                f.elements
                    .iter()
                    .filter_map(|c| match claims.get(&c.id) {
                        Some((_, who)) if *who != self.me.slug => Some((c.id, self.name_of(who))),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every zone with whoever reserved it.
    pub fn zones(&self) -> Vec<(&Member, &Zone)> {
        let mut out = Vec::new();
        for (who, f) in &self.reservations {
            if let Some(m) = self.member(who) {
                out.extend(f.zones.iter().map(|z| (m, z)));
            }
        }
        out
    }

    fn my_reservations(&mut self) -> &mut ReservationFile {
        let slug = self.me.slug.clone();
        self.reservations
            .entry(slug.clone())
            .or_insert_with(|| ReservationFile {
                author: slug,
                ..Default::default()
            })
    }

    fn write_reservations(&mut self) -> Result<(), String> {
        let path = file_of(&self.root, RESERVATIONS, &self.me.slug);
        let f = self.my_reservations().clone();
        write_json(&path, &f)?;
        self.changed(&path);
        Ok(())
    }

    /// Reserves elements. Those someone else has are left out and returned
    /// with who has them.
    pub fn reserve(
        &mut self,
        ids: &[u64],
        loc: &dyn Locator,
    ) -> Result<Vec<(u64, String)>, String> {
        self.refresh()?;
        let mut refused = Vec::new();
        let mut new = Vec::new();
        for &id in ids {
            match self.owner(id, loc) {
                Some(o) if o.slug == self.me.slug => {
                    // Already mine by a zone: claim it too, so it stays
                    // mine if the zone is released.
                    if !self.claims().contains_key(&id) {
                        new.push(id);
                    }
                }
                Some(o) => refused.push((id, o.name.clone())),
                None => new.push(id),
            }
        }
        if !new.is_empty() {
            let clock = self.next_clock();
            let time = now();
            let mine = self.my_reservations();
            for id in new {
                if !mine.elements.iter().any(|c| c.id == id) {
                    mine.elements.push(Claim { id, clock, time });
                }
            }
            self.write_reservations()?;
        }
        Ok(refused)
    }

    /// Releases elements.
    pub fn release(&mut self, ids: &[u64]) -> Result<(), String> {
        let set: BTreeSet<u64> = ids.iter().copied().collect();
        self.my_reservations()
            .elements
            .retain(|c| !set.contains(&c.id));
        self.write_reservations()
    }

    /// Releases every element and zone.
    pub fn release_all(&mut self) -> Result<(), String> {
        let mine = self.my_reservations();
        mine.elements.clear();
        mine.zones.clear();
        self.write_reservations()
    }

    /// Reserves a zone. Refused if it overlaps someone else's zone.
    pub fn reserve_zone(
        &mut self,
        name: &str,
        level: u64,
        polygon: Vec<[f64; 2]>,
    ) -> Result<(), String> {
        self.refresh()?;
        for (m, z) in self.zones() {
            if m.slug != self.me.slug && z.level == level && zones_overlap(&z.polygon, &polygon) {
                return Err(format!("se solapa con la zona «{}» de {}", z.name, m.name));
            }
        }
        let clock = self.next_clock();
        let zone = Zone {
            name: if name.trim().is_empty() {
                "Zona".into()
            } else {
                name.trim().into()
            },
            level,
            polygon,
            clock,
            time: now(),
        };
        self.my_reservations().zones.push(zone);
        self.write_reservations()
    }

    /// Releases one of my zones by its position in my list.
    pub fn release_zone(&mut self, index: usize) -> Result<(), String> {
        let mine = self.my_reservations();
        if index < mine.zones.len() {
            mine.zones.remove(index);
        }
        self.write_reservations()
    }

    pub fn my_zones(&self) -> &[Zone] {
        self.reservations
            .get(&self.me.slug)
            .map_or(&[], |f| f.zones.as_slice())
    }

    // ---- Messages -------------------------------------------------------

    /// Sends a note (a request to release `ids`, if any) to someone.
    pub fn send(&mut self, to: &str, text: &str, ids: &[u64]) -> Result<(), String> {
        let slug = self.me.slug.clone();
        let f = self
            .messages
            .entry(slug.clone())
            .or_insert_with(|| MessageFile {
                author: slug.clone(),
                messages: Vec::new(),
            });
        let n = f.messages.iter().map(|m| m.n).max().unwrap_or(0) + 1;
        f.messages.push(Message {
            n,
            to: to.into(),
            time: now(),
            text: text.into(),
            ids: ids.to_vec(),
        });
        let f = f.clone();
        let path = file_of(&self.root, MESSAGES, &slug);
        write_json(&path, &f)?;
        self.changed(&path);
        Ok(())
    }

    /// Messages for me, newest first: (from, message, unread).
    pub fn inbox(&self) -> Vec<(&Member, &Message, bool)> {
        let mut out = Vec::new();
        for (who, f) in &self.messages {
            let Some(m) = self.member(who) else { continue };
            let read = self.state.read.get(who).copied().unwrap_or(0);
            for msg in f.messages.iter().filter(|x| x.to == self.me.slug) {
                out.push((m, msg, msg.n > read));
            }
        }
        out.sort_by_key(|m| std::cmp::Reverse(m.1.time));
        out
    }

    pub fn unread(&self) -> usize {
        self.inbox().iter().filter(|m| m.2).count()
    }

    /// Marks every message as read.
    pub fn mark_read(&mut self) -> Result<(), String> {
        for (who, f) in &self.messages {
            if let Some(n) = f
                .messages
                .iter()
                .filter(|m| m.to == self.me.slug)
                .map(|m| m.n)
                .max()
            {
                self.state.read.insert(who.clone(), n);
            }
        }
        write_json(&file_of(&self.root, WORK, &self.me.slug), &self.state)
    }
}

fn hash(s: &str) -> u64 {
    // FNV-1a.
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

/// Whether two zones of the same level overlap (no polygon: the whole level).
pub fn zones_overlap(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return true;
    }
    a.iter().any(|p| inside(b, *p))
        || b.iter().any(|p| inside(a, *p))
        || edges(a).any(|(p, q)| edges(b).any(|(r, s)| cross(p, q, r, s)))
}

fn edges(p: &[[f64; 2]]) -> impl Iterator<Item = ([f64; 2], [f64; 2])> + '_ {
    (0..p.len()).map(move |i| (p[i], p[(i + 1) % p.len()]))
}

/// Whether a point is inside a polygon (even-odd).
pub fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut c = false;
    for (a, b) in edges(poly) {
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                c = !c;
            }
        }
    }
    c
}

fn cross(p: [f64; 2], q: [f64; 2], r: [f64; 2], s: [f64; 2]) -> bool {
    let o = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let (d1, d2, d3, d4) = (o(r, s, p), o(r, s, q), o(p, q, r), o(p, q, s));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}
