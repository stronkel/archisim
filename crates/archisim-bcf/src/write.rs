//! Writing BCF 3.0 (by default) or 2.1 archives, as Revit, ArchiCAD,
//! Solibri and BIMcollab read them.

use crate::model::*;
use crate::zip;

/// Escapes text for XML content and attributes.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

struct Xml {
    s: String,
    depth: usize,
}

impl Xml {
    fn new() -> Xml {
        Xml {
            s: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n".into(),
            depth: 0,
        }
    }
    fn pad(&mut self) {
        for _ in 0..self.depth {
            self.s.push_str("  ");
        }
    }
    fn open(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.pad();
        self.s.push('<');
        self.s.push_str(tag);
        for (k, v) in attrs {
            self.s += &format!(" {k}=\"{}\"", esc(v));
        }
        self.s.push_str(">\n");
        self.depth += 1;
    }
    fn close(&mut self, tag: &str) {
        self.depth -= 1;
        self.pad();
        self.s += &format!("</{tag}>\n");
    }
    fn empty(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.pad();
        self.s.push('<');
        self.s.push_str(tag);
        for (k, v) in attrs {
            self.s += &format!(" {k}=\"{}\"", esc(v));
        }
        self.s.push_str("/>\n");
    }
    fn text(&mut self, tag: &str, value: &str) {
        self.pad();
        self.s += &format!("<{tag}>{}</{tag}>\n", esc(value));
    }
    fn opt(&mut self, tag: &str, value: &Option<String>) {
        if let Some(v) = value {
            self.text(tag, v);
        }
    }
    fn num(&mut self, tag: &str, v: f64) {
        self.text(tag, &fmt_num(v));
    }
    fn v3(&mut self, tag: &str, v: V3) {
        self.open(tag, &[]);
        self.num("X", v[0]);
        self.num("Y", v[1]);
        self.num("Z", v[2]);
        self.close(tag);
    }
}

/// The shortest text that reads back as the same number.
fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v}")
    }
}

/// The archive bytes of a BCF file, in `bcf.version`.
pub fn write_bcf(bcf: &Bcf) -> Vec<u8> {
    let v3_0 = bcf.version == BcfVersion::V3_0;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    let mut x = Xml::new();
    let attrs = [
        ("VersionId", bcf.version.id()),
        ("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"),
    ];
    if v3_0 {
        // 3.0: an empty element (no DetailedVersion, no text).
        x.empty("Version", &attrs);
    } else {
        x.open("Version", &attrs);
        x.text("DetailedVersion", bcf.version.id());
        x.close("Version");
    }
    files.push(("bcf.version".into(), x.s.into_bytes()));

    if bcf.project_id.is_some() || bcf.project_name.is_some() {
        let mut x = Xml::new();
        x.open("ProjectInfo", &[]);
        let id = bcf.project_id.clone().unwrap_or_else(new_guid);
        x.open("Project", &[("ProjectId", &id)]);
        x.opt("Name", &bcf.project_name);
        x.close("Project");
        if !v3_0 {
            x.empty("ExtensionSchema", &[]);
        }
        x.close("ProjectInfo");
        files.push(("project.bcfp".into(), x.s.into_bytes()));
    }

    if v3_0 {
        let e = &bcf.extensions;
        let mut x = Xml::new();
        x.open("Extensions", &[]);
        for (list, item, values) in [
            ("TopicTypes", "TopicType", &e.topic_types),
            ("TopicStatuses", "TopicStatus", &e.topic_statuses),
            ("Priorities", "Priority", &e.priorities),
            ("TopicLabels", "TopicLabel", &e.topic_labels),
            ("Users", "User", &e.users),
            ("Stages", "Stage", &e.stages),
        ] {
            if values.is_empty() {
                continue;
            }
            x.open(list, &[]);
            for v in values {
                x.text(item, v);
            }
            x.close(list);
        }
        x.close("Extensions");
        files.push(("extensions.xml".into(), x.s.into_bytes()));
    }

    // BCF 3.0 requires a type and a status on every topic.
    let first = |l: &[String], or: &str| l.first().cloned().unwrap_or_else(|| or.to_string());
    let default_type = first(&bcf.extensions.topic_types, "Issue");
    let default_status = first(&bcf.extensions.topic_statuses, "Open");
    for t in &bcf.topics {
        if v3_0 && (t.topic_type.is_none() || t.status.is_none()) {
            let mut t = t.clone();
            t.topic_type.get_or_insert(default_type.clone());
            t.status.get_or_insert(default_status.clone());
            write_topic(&t, v3_0, &mut files);
        } else {
            write_topic(t, v3_0, &mut files);
        }
    }
    zip::write(&files)
}

fn write_topic(t: &Topic, v3_0: bool, files: &mut Vec<(String, Vec<u8>)>) {
    let folder = &t.guid;
    let mut x = Xml::new();
    x.open("Markup", &[]);
    if !t.files.is_empty() {
        x.open("Header", &[]);
        if v3_0 {
            x.open("Files", &[]);
        }
        for f in &t.files {
            let mut attrs: Vec<(&str, &str)> = Vec::new();
            if let Some(p) = &f.ifc_project {
                attrs.push(("IfcProject", p));
            }
            if let Some(p) = &f.ifc_spatial_element {
                attrs.push(("IfcSpatialStructureElement", p));
            }
            attrs.push((
                if v3_0 { "IsExternal" } else { "isExternal" },
                if f.is_external { "true" } else { "false" },
            ));
            x.open("File", &attrs);
            x.opt("Filename", &f.filename);
            x.opt("Date", &f.date);
            x.opt("Reference", &f.reference);
            x.close("File");
        }
        if v3_0 {
            x.close("Files");
        }
        x.close("Header");
    }
    let mut attrs: Vec<(&str, &str)> = vec![("Guid", &t.guid)];
    if let Some(ty) = &t.topic_type {
        attrs.push(("TopicType", ty));
    }
    if let Some(st) = &t.status {
        attrs.push(("TopicStatus", st));
    }
    x.open("Topic", &attrs);
    if v3_0 {
        if !t.reference_links.is_empty() {
            x.open("ReferenceLinks", &[]);
            for r in &t.reference_links {
                x.text("ReferenceLink", r);
            }
            x.close("ReferenceLinks");
        }
    } else {
        for r in &t.reference_links {
            x.text("ReferenceLink", r);
        }
    }
    x.text("Title", &t.title);
    x.opt("Priority", &t.priority);
    if let Some(i) = t.index {
        x.text("Index", &i.to_string());
    }
    if v3_0 {
        if !t.labels.is_empty() {
            x.open("Labels", &[]);
            for l in &t.labels {
                x.text("Label", l);
            }
            x.close("Labels");
        }
    } else {
        for l in &t.labels {
            x.text("Labels", l);
        }
    }
    x.text("CreationDate", &t.creation_date);
    x.text("CreationAuthor", &t.creation_author);
    x.opt("ModifiedDate", &t.modified_date);
    x.opt("ModifiedAuthor", &t.modified_author);
    x.opt("DueDate", &t.due_date);
    x.opt("AssignedTo", &t.assigned_to);
    x.opt("Stage", &t.stage);
    x.opt("Description", &t.description);
    if !t.related_topics.is_empty() {
        if v3_0 {
            x.open("RelatedTopics", &[]);
        }
        for r in &t.related_topics {
            x.empty("RelatedTopic", &[("Guid", r)]);
        }
        if v3_0 {
            x.close("RelatedTopics");
        }
    }
    if v3_0 {
        write_comments(&mut x, t);
        write_viewpoint_refs(&mut x, t, true);
        x.close("Topic");
    } else {
        x.close("Topic");
        write_comments(&mut x, t);
        write_viewpoint_refs(&mut x, t, false);
    }
    x.close("Markup");
    files.push((format!("{folder}/markup.bcf"), x.s.into_bytes()));

    for (i, vp) in t.viewpoints.iter().enumerate() {
        let (name, snap) = viewpoint_names(i, vp);
        files.push((
            format!("{folder}/{name}"),
            write_viewpoint(vp, v3_0).into_bytes(),
        ));
        if let Some(png) = &vp.snapshot {
            files.push((format!("{folder}/{snap}"), png.clone()));
        }
    }
}

/// The first viewpoint keeps the conventional names (`viewpoint.bcfv`,
/// `snapshot.png`), as 2.1 readers expect.
fn viewpoint_names(i: usize, vp: &Viewpoint) -> (String, String) {
    if i == 0 {
        ("viewpoint.bcfv".into(), "snapshot.png".into())
    } else {
        (format!("{}.bcfv", vp.guid), format!("{}.png", vp.guid))
    }
}

fn write_comments(x: &mut Xml, t: &Topic) {
    if t.comments.is_empty() {
        return;
    }
    // BCF 3.0 keeps comments inside <Topic> (depth 2); 2.1, beside it.
    let inside_topic = x.depth >= 2;
    if inside_topic {
        x.open("Comments", &[]);
    }
    for c in &t.comments {
        x.open("Comment", &[("Guid", &c.guid)]);
        x.text("Date", &c.date);
        x.text("Author", &c.author);
        x.text("Comment", &c.text);
        if let Some(v) = &c.viewpoint {
            x.empty("Viewpoint", &[("Guid", v)]);
        }
        x.opt("ModifiedDate", &c.modified_date);
        x.opt("ModifiedAuthor", &c.modified_author);
        x.close("Comment");
    }
    if inside_topic {
        x.close("Comments");
    }
}

fn write_viewpoint_refs(x: &mut Xml, t: &Topic, v3_0: bool) {
    if t.viewpoints.is_empty() {
        return;
    }
    if v3_0 {
        x.open("Viewpoints", &[]);
    }
    for (i, vp) in t.viewpoints.iter().enumerate() {
        let (name, snap) = viewpoint_names(i, vp);
        x.open(
            if v3_0 { "ViewPoint" } else { "Viewpoints" },
            &[("Guid", &vp.guid)],
        );
        x.text("Viewpoint", &name);
        if vp.snapshot.is_some() {
            x.text("Snapshot", &snap);
        }
        if let Some(i) = vp.index {
            x.text("Index", &i.to_string());
        }
        x.close(if v3_0 { "ViewPoint" } else { "Viewpoints" });
    }
    if v3_0 {
        x.close("Viewpoints");
    }
}

fn components(x: &mut Xml, guids: &[String]) {
    for g in guids {
        x.empty("Component", &[("IfcGuid", g)]);
    }
}

pub(crate) fn write_viewpoint(vp: &Viewpoint, v3_0: bool) -> String {
    let mut x = Xml::new();
    x.open("VisualizationInfo", &[("Guid", &vp.guid)]);
    x.open("Components", &[]);
    if !v3_0 {
        x.empty(
            "ViewSetupHints",
            &[
                ("SpacesVisible", bool_s(vp.spaces_visible)),
                ("SpaceBoundariesVisible", "false"),
                ("OpeningsVisible", bool_s(vp.openings_visible)),
            ],
        );
    }
    if !vp.selection.is_empty() {
        x.open("Selection", &[]);
        components(&mut x, &vp.selection);
        x.close("Selection");
    }
    x.open(
        "Visibility",
        &[("DefaultVisibility", bool_s(vp.default_visibility))],
    );
    if v3_0 {
        x.empty(
            "ViewSetupHints",
            &[
                ("SpacesVisible", bool_s(vp.spaces_visible)),
                ("SpaceBoundariesVisible", "false"),
                ("OpeningsVisible", bool_s(vp.openings_visible)),
            ],
        );
    }
    if !vp.exceptions.is_empty() {
        x.open("Exceptions", &[]);
        components(&mut x, &vp.exceptions);
        x.close("Exceptions");
    }
    x.close("Visibility");
    if !vp.coloring.is_empty() {
        x.open("Coloring", &[]);
        for (color, guids) in &vp.coloring {
            x.open("Color", &[("Color", color)]);
            if v3_0 {
                x.open("Components", &[]);
            }
            components(&mut x, guids);
            if v3_0 {
                x.close("Components");
            }
            x.close("Color");
        }
        x.close("Coloring");
    }
    x.close("Components");
    match &vp.camera {
        Some(Camera::Perspective {
            position,
            direction,
            up,
            field_of_view,
            aspect_ratio,
        }) => {
            x.open("PerspectiveCamera", &[]);
            x.v3("CameraViewPoint", *position);
            x.v3("CameraDirection", *direction);
            x.v3("CameraUpVector", *up);
            x.num("FieldOfView", *field_of_view);
            if v3_0 {
                x.num("AspectRatio", *aspect_ratio);
            }
            x.close("PerspectiveCamera");
        }
        Some(Camera::Orthogonal {
            position,
            direction,
            up,
            view_to_world_scale,
            aspect_ratio,
        }) => {
            x.open("OrthogonalCamera", &[]);
            x.v3("CameraViewPoint", *position);
            x.v3("CameraDirection", *direction);
            x.v3("CameraUpVector", *up);
            x.num("ViewToWorldScale", *view_to_world_scale);
            if v3_0 {
                x.num("AspectRatio", *aspect_ratio);
            }
            x.close("OrthogonalCamera");
        }
        None => {}
    }
    if !vp.lines.is_empty() {
        x.open("Lines", &[]);
        for l in &vp.lines {
            x.open("Line", &[]);
            x.v3("StartPoint", l.start);
            x.v3("EndPoint", l.end);
            x.close("Line");
        }
        x.close("Lines");
    }
    if !vp.clipping_planes.is_empty() {
        x.open("ClippingPlanes", &[]);
        for p in &vp.clipping_planes {
            x.open("ClippingPlane", &[]);
            x.v3("Location", p.location);
            x.v3("Direction", p.direction);
            x.close("ClippingPlane");
        }
        x.close("ClippingPlanes");
    }
    x.close("VisualizationInfo");
    x.s
}

fn bool_s(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}
