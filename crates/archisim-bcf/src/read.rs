//! Reading BCF 2.1 and 3.0 archives.

use std::collections::HashMap;

use crate::model::*;
use crate::zip;

#[derive(Debug, Clone, PartialEq)]
pub struct BcfError(pub String);

impl std::fmt::Display for BcfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BcfError {}

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn text(n: Option<Node>) -> Option<String> {
    n.and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn texts(n: Option<Node>, item: &str) -> Vec<String> {
    n.map(|n| children(n, item).filter_map(|c| text(Some(c))).collect())
        .unwrap_or_default()
}

fn attr(n: Node, name: &str) -> Option<String> {
    n.attribute(name).map(str::to_string)
}

fn parse(bytes: &[u8], _what: &str) -> Result<String, BcfError> {
    let s = String::from_utf8_lossy(bytes);
    // Drop a byte order mark: roxmltree does not want it.
    Ok(s.trim_start_matches('\u{feff}').to_string())
}

fn doc<'i>(xml: &'i str, what: &str) -> Result<roxmltree::Document<'i>, BcfError> {
    roxmltree::Document::parse(xml).map_err(|e| BcfError(format!("{what}: XML no válido ({e})")))
}

/// Reads a `.bcf` / `.bcfzip` file.
pub fn read_bcf(bytes: &[u8]) -> Result<Bcf, BcfError> {
    let files = zip::read(bytes).map_err(|e| BcfError(e.to_string()))?;
    let by_name: HashMap<String, &Vec<u8>> = files
        .iter()
        .map(|(n, c)| (n.trim_start_matches("./").to_string(), c))
        .collect();
    let mut bcf = Bcf::default();
    match by_name.get("bcf.version") {
        Some(v) => {
            let xml = parse(v, "bcf.version")?;
            let d = doc(&xml, "bcf.version")?;
            let id = d.root_element().attribute("VersionId").unwrap_or("2.1");
            bcf.version = if id.starts_with('3') {
                BcfVersion::V3_0
            } else {
                BcfVersion::V2_1
            };
        }
        None => return Err(BcfError("no es un fichero BCF (falta bcf.version)".into())),
    }
    let project = by_name
        .get("project.bcfp")
        .or_else(|| by_name.get("project.bcfproject"));
    if let Some(p) = project {
        let xml = parse(p, "project.bcfp")?;
        let d = doc(&xml, "project.bcfp")?;
        if let Some(pr) = d.descendants().find(|n| n.has_tag_name("Project")) {
            bcf.project_id = attr(pr, "ProjectId");
            bcf.project_name = text(child(pr, "Name"));
        }
    }
    if let Some(x) = by_name.get("extensions.xml") {
        let xml = parse(x, "extensions.xml")?;
        let d = doc(&xml, "extensions.xml")?;
        let r = d.root_element();
        bcf.extensions = Extensions {
            topic_types: texts(child(r, "TopicTypes"), "TopicType"),
            topic_statuses: texts(child(r, "TopicStatuses"), "TopicStatus"),
            priorities: texts(child(r, "Priorities"), "Priority"),
            topic_labels: texts(child(r, "TopicLabels"), "TopicLabel"),
            stages: texts(child(r, "Stages"), "Stage"),
            users: texts(child(r, "Users"), "User"),
        };
    }
    // Topics: one folder per topic with its markup.bcf.
    let mut folders: Vec<&str> = by_name
        .keys()
        .filter_map(|n| n.strip_suffix("/markup.bcf"))
        .collect();
    folders.sort_unstable();
    for folder in folders {
        let get = |name: &str| by_name.get(&format!("{folder}/{name}")).copied();
        let markup = get("markup.bcf").expect("listed");
        let xml = parse(markup, "markup.bcf")?;
        let topic = read_markup(&xml, folder, &get)?;
        bcf.topics.push(topic);
    }
    bcf.topics.sort_by(|a, b| {
        a.index
            .cmp(&b.index)
            .then_with(|| a.creation_date.cmp(&b.creation_date))
    });
    Ok(bcf)
}

fn read_markup<'f>(
    xml: &str,
    folder: &str,
    get: &dyn Fn(&str) -> Option<&'f Vec<u8>>,
) -> Result<Topic, BcfError> {
    let d = doc(xml, &format!("{folder}/markup.bcf"))?;
    let m = d.root_element();
    let t =
        child(m, "Topic").ok_or_else(|| BcfError(format!("{folder}: incidencia sin <Topic>")))?;
    let mut topic = Topic {
        guid: attr(t, "Guid").unwrap_or_else(|| folder.to_string()),
        topic_type: attr(t, "TopicType").filter(|s| !s.is_empty()),
        status: attr(t, "TopicStatus").filter(|s| !s.is_empty()),
        title: text(child(t, "Title")).unwrap_or_default(),
        priority: text(child(t, "Priority")),
        index: text(child(t, "Index")).and_then(|s| s.parse().ok()),
        creation_date: text(child(t, "CreationDate")).unwrap_or_default(),
        creation_author: text(child(t, "CreationAuthor")).unwrap_or_default(),
        modified_date: text(child(t, "ModifiedDate")),
        modified_author: text(child(t, "ModifiedAuthor")),
        due_date: text(child(t, "DueDate")),
        assigned_to: text(child(t, "AssignedTo")),
        stage: text(child(t, "Stage")),
        description: text(child(t, "Description")),
        ..Topic::default()
    };
    // Labels: <Labels><Label/></Labels> in 3.0, repeated <Labels> in 2.1.
    for l in children(t, "Labels") {
        let inner: Vec<String> = children(l, "Label").filter_map(|c| text(Some(c))).collect();
        if inner.is_empty() {
            topic.labels.extend(text(Some(l)));
        } else {
            topic.labels.extend(inner);
        }
    }
    // Reference links: <ReferenceLinks><ReferenceLink/> (3.0) or
    // repeated <ReferenceLink> (2.1).
    for r in children(t, "ReferenceLink") {
        topic.reference_links.extend(text(Some(r)));
    }
    if let Some(rl) = child(t, "ReferenceLinks") {
        topic
            .reference_links
            .extend(texts(Some(rl), "ReferenceLink"));
    }
    for rt in t.descendants().filter(|n| n.has_tag_name("RelatedTopic")) {
        topic.related_topics.extend(attr(rt, "Guid"));
    }
    if let Some(h) = child(m, "Header") {
        let files = child(h, "Files").unwrap_or(h);
        for f in children(files, "File") {
            topic.files.push(FileRef {
                ifc_project: attr(f, "IfcProject"),
                ifc_spatial_element: attr(f, "IfcSpatialStructureElement"),
                is_external: f
                    .attribute("IsExternal")
                    .or_else(|| f.attribute("isExternal"))
                    .is_none_or(|v| v != "false"),
                filename: text(child(f, "Filename")),
                date: text(child(f, "Date")),
                reference: text(child(f, "Reference")),
            });
        }
    }
    // Comments: inside <Topic><Comments> (3.0) or beside it (2.1).
    let comment_nodes: Vec<Node> = match child(t, "Comments") {
        Some(c) => children(c, "Comment").collect(),
        None => children(m, "Comment").collect(),
    };
    for c in comment_nodes {
        topic.comments.push(Comment {
            guid: attr(c, "Guid").unwrap_or_else(new_guid),
            date: text(child(c, "Date")).unwrap_or_default(),
            author: text(child(c, "Author")).unwrap_or_default(),
            text: child(c, "Comment")
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string(),
            viewpoint: child(c, "Viewpoint").and_then(|v| attr(v, "Guid")),
            modified_date: text(child(c, "ModifiedDate")),
            modified_author: text(child(c, "ModifiedAuthor")),
        });
    }
    // Viewpoints: <Topic><Viewpoints><ViewPoint> (3.0) or <Viewpoints>
    // beside the topic (2.1).
    let vp_nodes: Vec<Node> = match child(t, "Viewpoints") {
        Some(v) => children(v, "ViewPoint").collect(),
        None => children(m, "Viewpoints").collect(),
    };
    for v in vp_nodes {
        let guid = attr(v, "Guid").unwrap_or_else(new_guid);
        let file = text(child(v, "Viewpoint")).unwrap_or_else(|| "viewpoint.bcfv".into());
        let mut vp = match get(&file) {
            Some(bytes) => {
                let xml = parse(bytes, &file)?;
                read_viewpoint(&xml, &guid)
                    .map_err(|e| BcfError(format!("{folder}/{file}: {e}")))?
            }
            None => Viewpoint::new(guid.clone()),
        };
        vp.guid = guid;
        vp.index = text(child(v, "Index")).and_then(|s| s.parse().ok());
        if let Some(snap) = text(child(v, "Snapshot")) {
            vp.snapshot = get(&snap).cloned();
        }
        topic.viewpoints.push(vp);
    }
    Ok(topic)
}

fn v3(n: Option<Node>) -> Option<V3> {
    let n = n?;
    let c = |k: &str| text(child(n, k)).and_then(|s| s.parse::<f64>().ok());
    Some([c("X")?, c("Y")?, c("Z")?])
}

fn num(n: Option<Node>) -> Option<f64> {
    text(n).and_then(|s| s.parse().ok())
}

fn guids(n: Option<Node>) -> Vec<String> {
    n.map(|n| {
        n.descendants()
            .filter(|c| c.has_tag_name("Component"))
            .filter_map(|c| attr(c, "IfcGuid"))
            .collect()
    })
    .unwrap_or_default()
}

pub(crate) fn read_viewpoint(xml: &str, guid: &str) -> Result<Viewpoint, BcfError> {
    let d = doc(xml, "punto de vista")?;
    let r = d.root_element();
    let mut vp = Viewpoint::new(guid.to_string());
    if let Some(c) = child(r, "Components") {
        vp.selection = guids(child(c, "Selection"));
        if let Some(vis) = child(c, "Visibility") {
            // The schema default is false; without <Visibility>, all shows.
            vp.default_visibility = vis.attribute("DefaultVisibility") == Some("true");
            vp.exceptions = guids(child(vis, "Exceptions"));
            if let Some(h) = child(vis, "ViewSetupHints").or_else(|| child(c, "ViewSetupHints")) {
                vp.spaces_visible = h.attribute("SpacesVisible") == Some("true");
                vp.openings_visible = h.attribute("OpeningsVisible") == Some("true");
            }
        }
        if let Some(col) = child(c, "Coloring") {
            for color in children(col, "Color") {
                vp.coloring.push((
                    attr(color, "Color").unwrap_or_else(|| "FF0000".into()),
                    guids(Some(color)),
                ));
            }
        }
    }
    let up_default = [0.0, 0.0, 1.0];
    if let Some(p) = child(r, "PerspectiveCamera") {
        vp.camera = Some(Camera::Perspective {
            position: v3(child(p, "CameraViewPoint")).unwrap_or_default(),
            direction: v3(child(p, "CameraDirection")).unwrap_or([0.0, 1.0, 0.0]),
            up: v3(child(p, "CameraUpVector")).unwrap_or(up_default),
            field_of_view: num(child(p, "FieldOfView")).unwrap_or(60.0),
            aspect_ratio: num(child(p, "AspectRatio")).unwrap_or(1.5),
        });
    } else if let Some(o) = child(r, "OrthogonalCamera") {
        vp.camera = Some(Camera::Orthogonal {
            position: v3(child(o, "CameraViewPoint")).unwrap_or_default(),
            direction: v3(child(o, "CameraDirection")).unwrap_or([0.0, 0.0, -1.0]),
            up: v3(child(o, "CameraUpVector")).unwrap_or([0.0, 1.0, 0.0]),
            view_to_world_scale: num(child(o, "ViewToWorldScale")).unwrap_or(10.0),
            aspect_ratio: num(child(o, "AspectRatio")).unwrap_or(1.5),
        });
    }
    if let Some(cp) = child(r, "ClippingPlanes") {
        for p in children(cp, "ClippingPlane") {
            if let (Some(location), Some(direction)) =
                (v3(child(p, "Location")), v3(child(p, "Direction")))
            {
                vp.clipping_planes.push(ClippingPlane {
                    location,
                    direction,
                });
            }
        }
    }
    if let Some(ls) = child(r, "Lines") {
        for l in children(ls, "Line") {
            if let (Some(start), Some(end)) = (v3(child(l, "StartPoint")), v3(child(l, "EndPoint")))
            {
                vp.lines.push(Line { start, end });
            }
        }
    }
    Ok(vp)
}
