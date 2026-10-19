use crate::*;

/// A box element: class, GlobalId, name, lower and upper corner.
pub(crate) struct B<'a>(&'a str, &'a str, &'a str, [f64; 3], [f64; 3]);

/// A small IFC4 file in metres: every box an extrusion on its own
/// placement, in one storey; `openings` (host GlobalId, box) void their
/// host.
pub(crate) fn ifc(boxes: &[B], openings: &[(&str, [f64; 3], [f64; 3])]) -> String {
    let mut s = String::from(
        "ISO-10303-21;HEADER;FILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\
         FILE_NAME('x.ifc','2026-10-19T00:00:00',(''),(''),'','Prueba','');\
         FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\n\
         #1=IFCPROJECT('0p000000000000000000p1',$,'Proyecto',$,$,$,$,$,#2);\n\
         #2=IFCUNITASSIGNMENT((#3));#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n\
         #4=IFCDIRECTION((0.,0.,1.));#5=IFCBUILDINGSTOREY('0s000000000000000000s1',$,'Planta baja',$,$,$,$,$,.ELEMENT.,0.);\n",
    );
    let mut n = 100;
    let mut contained = Vec::new();
    let mut ids = std::collections::HashMap::new();
    let solid = |s: &mut String, n: &mut u64, lo: [f64; 3], hi: [f64; 3]| -> (u64, u64) {
        let b = *n;
        *n += 10;
        *s += &format!(
            "#{b}=IFCCARTESIANPOINT(({},{},{}));#{}=IFCAXIS2PLACEMENT3D(#{b},$,$);#{}=IFCLOCALPLACEMENT($,#{});\n\
             #{}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,{},{});#{}=IFCCARTESIANPOINT(({},{}));#{}=IFCAXIS2PLACEMENT2D(#{},$);\n\
             #{}=IFCEXTRUDEDAREASOLID(#{},$,#4,{});#{}=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#{}));#{}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{}));\n",
            f(lo[0]), f(lo[1]), f(lo[2]),
            b + 1, b + 2, b + 1,
            b + 3, f(hi[0] - lo[0]), f(hi[1] - lo[1]),
            b + 4, f((hi[0] - lo[0]) / 2.0), f((hi[1] - lo[1]) / 2.0), b + 5, b + 4,
            b + 6, b + 3, f(hi[2] - lo[2]), b + 7, b + 6, b + 8, b + 7,
        );
        // The profile placement goes in its own field.
        let fixed = s.replace(
            &format!("#{}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,", b + 3),
            &format!("#{}=IFCRECTANGLEPROFILEDEF(.AREA.,$,#{},", b + 3, b + 5),
        );
        *s = fixed;
        (b + 2, b + 8)
    };
    for B(class, guid, name, lo, hi) in boxes {
        let (pl, shape) = solid(&mut s, &mut n, *lo, *hi);
        let id = n;
        n += 1;
        let extra = match *class {
            "IFCBEAM" | "IFCCOLUMN" | "IFCWALL" | "IFCSLAB" | "IFCPIPESEGMENT"
            | "IFCDUCTSEGMENT" => ",$",
            _ => "",
        };
        s += &format!("#{id}={class}('{guid}',$,'{name}',$,$,#{pl},#{shape},${extra});\n");
        contained.push(format!("#{id}"));
        ids.insert(guid.to_string(), id);
    }
    for (k, (host, lo, hi)) in openings.iter().enumerate() {
        let (pl, shape) = solid(&mut s, &mut n, *lo, *hi);
        let id = n;
        n += 2;
        s += &format!(
            "#{id}=IFCOPENINGELEMENT('0o00000000000000000{k:03}',$,'Hueco',$,$,#{pl},#{shape},$,.OPENING.);\n\
             #{}=IFCRELVOIDSELEMENT('0v00000000000000000{k:03}',$,$,$,#{},#{id});\n",
            id + 1,
            ids[*host]
        );
    }
    s += &format!(
        "#{n}=IFCRELCONTAINEDINSPATIALSTRUCTURE('0c000000000000000000c1',$,$,$,({}),#5);\nENDSEC;END-ISO-10303-21;\n",
        contained.join(",")
    );
    s
}

fn f(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.")
    }
}

fn architecture() -> String {
    ifc(
        &[
            B(
                "IFCWALL",
                "0w000000000000000000w1",
                "Fachada",
                [0.0, 0.0, 0.0],
                [6.0, 0.25, 3.0],
            ),
            B(
                "IFCSLAB",
                "0f000000000000000000f1",
                "Solera",
                [0.0, 0.0, -0.3],
                [6.0, 5.0, 0.0],
            ),
        ],
        &[("0w000000000000000000w1", [2.0, -0.1, 1.0], [3.0, 0.35, 2.0])],
    )
}

fn structure_and_services() -> String {
    ifc(
        &[
            // Crosses the façade above the window: clash.
            B(
                "IFCBEAM",
                "0b000000000000000000b1",
                "Viga V-1",
                [3.5, -1.0, 2.5],
                [3.8, 2.0, 2.8],
            ),
            // Rests on the ground slab: contact, no clash.
            B(
                "IFCBEAM",
                "0b000000000000000000b2",
                "Viga de atado",
                [0.5, 1.0, 0.0],
                [5.5, 1.3, 0.4],
            ),
            // Through the window opening: no clash.
            B(
                "IFCPIPESEGMENT",
                "0t000000000000000000t1",
                "Bajante",
                [2.4, -1.0, 1.4],
                [2.6, 2.0, 1.6],
            ),
            // Through the slab: clash.
            B(
                "IFCPIPESEGMENT",
                "0t000000000000000000t2",
                "Acometida",
                [5.0, 3.0, -1.0],
                [5.1, 3.1, 1.0],
            ),
        ],
        &[],
    )
}

#[test]
fn reads_elements_with_openings_and_storeys() {
    let arch = read_elements(&architecture()).unwrap();
    assert_eq!(arch.elements.len(), 2);
    let wall = &arch.elements[0];
    assert_eq!(wall.class, "IFCWALL");
    assert_eq!(wall.storey.as_deref(), Some("Planta baja"));
    assert_eq!(wall.voids.len(), 1);
    assert!((wall.volume - 6.0 * 0.25 * 3.0).abs() < 1e-9);
    assert_eq!(arch.main_discipline(), Discipline::Architecture);
    let mep = read_elements(&structure_and_services()).unwrap();
    assert_eq!(mep.elements[0].discipline, Discipline::Structure);
    assert_eq!(mep.elements[2].discipline, Discipline::Plumbing);
}

#[test]
fn clashes_ignore_contact_and_openings() {
    let arch = read_elements(&architecture()).unwrap();
    let other = read_elements(&structure_and_services()).unwrap();
    let clashes = find_clashes(&arch, &other, &ClashOptions::default());
    let pairs: Vec<(&str, &str)> = clashes
        .iter()
        .map(|c| {
            (
                arch.elements[c.a].name.as_str(),
                other.elements[c.b].name.as_str(),
            )
        })
        .collect();
    assert_eq!(pairs.len(), 2, "{pairs:?}");
    assert!(pairs.contains(&("Fachada", "Viga V-1")));
    assert!(pairs.contains(&("Solera", "Acometida")));
    let slab = clashes
        .iter()
        .find(|c| other.elements[c.b].name == "Acometida")
        .unwrap();
    // Half the slab's thickness, or the pipe's half width seen from the
    // slab: the deepest point found is well inside.
    assert!(
        slab.depth > 0.04 && slab.depth <= 0.15 + 1e-9,
        "{}",
        slab.depth
    );
    // A coarse tolerance hides the shallow one.
    // Clearance of 50 cm: the pipe that went through the window opening
    // now counts (its sides are 15 cm from the jambs and the sill), the
    // beam resting on the slab does not (it touches).
    let roomy = ClashOptions {
        clearance: 0.5,
        ..ClashOptions::default()
    };
    let with_gaps = find_clashes(&arch, &other, &roomy);
    let hard = with_gaps.iter().filter(|c| c.gap.is_none()).count();
    assert_eq!(hard, 2);
    let soft: Vec<_> = with_gaps.iter().filter(|c| c.gap.is_some()).collect();
    assert!(soft
        .iter()
        .any(|c| other.elements[c.b].name == "Bajante" && arch.elements[c.a].name == "Fachada"));
    assert!(!soft.iter().any(
        |c| other.elements[c.b].name == "Viga de atado" && arch.elements[c.a].name == "Solera"
    ));
    // The pipe (20 × 20 cm at 2,40–2,60 m, 1,40–1,60 m high) is 40 cm
    // from the jambs, the head and the sill of the 1 × 1 m opening.
    let pipe = soft
        .iter()
        .find(|c| other.elements[c.b].name == "Bajante")
        .unwrap();
    assert!((pipe.gap.unwrap() - 0.4).abs() < 0.03, "{:?}", pipe.gap);
    // A copy of the same model linked to itself: nothing.
    assert!(find_clashes(&arch, &arch, &ClashOptions::default()).is_empty());
    let loose = ClashOptions {
        tolerance: 0.2,
        ..ClashOptions::default()
    };
    assert!(find_clashes(&arch, &other, &loose).is_empty());
}

#[test]
fn clash_topics_follow_the_model() {
    let arch = read_elements(&architecture()).unwrap();
    let other = read_elements(&structure_and_services()).unwrap();
    let opts = ClashOptions::default();
    let me = Author("arquitecto@ejemplo.es".into());
    let mut bcf = archisim_bcf::Bcf::default();
    let clashes = find_clashes(&arch, &other, &opts);
    assert_eq!(
        clashes_to_bcf(&mut bcf, &clashes, &arch, &other, &[], &me),
        (2, 0, 0)
    );
    assert_eq!(bcf.topics.len(), 2);
    let t = &bcf.topics[0];
    assert_eq!(t.topic_type.as_deref(), Some("Interferencia"));
    assert_eq!(t.viewpoints[0].selection.len(), 2);
    assert!(t.viewpoints[0].camera.is_some());
    // Survives a BCF file.
    let mut bcf = archisim_bcf::read_bcf(&archisim_bcf::write_bcf(&bcf)).unwrap();
    // The beam is moved out of the wall: its topic is resolved, the other
    // one stays.
    let moved = structure_and_services().replace(
        "#100=IFCCARTESIANPOINT((3.5,-1.,2.5))",
        "#100=IFCCARTESIANPOINT((3.5,1.,2.5))",
    );
    assert_ne!(moved, structure_and_services());
    let other2 = read_elements(&moved).unwrap();
    let clashes = find_clashes(&arch, &other2, &opts);
    assert_eq!(clashes.len(), 1);
    assert_eq!(
        clashes_to_bcf(&mut bcf, &clashes, &arch, &other2, &[], &me),
        (0, 1, 1)
    );
    let beam = bcf
        .topics
        .iter()
        .find(|t| t.title.contains("Viga V-1"))
        .unwrap();
    assert_eq!(beam.status.as_deref(), Some("Resuelta"));
    assert_eq!(beam.comments.len(), 1);
    // A second linked model checked after the first leaves the first one's
    // topics as they are.
    let third = read_elements(&ifc(
        &[B(
            "IFCCOLUMN",
            "0k000000000000000000k9",
            "Pilar",
            [1.0, -0.2, 0.0],
            [1.3, 0.1, 3.0],
        )],
        &[],
    ))
    .unwrap();
    let clashes = find_clashes(&arch, &third, &opts);
    assert_eq!(clashes.len(), 1);
    assert_eq!(
        clashes_to_bcf(&mut bcf, &clashes, &arch, &third, &[], &me),
        (1, 0, 0)
    );
    let open = bcf
        .topics
        .iter()
        .filter(|t| t.status.as_deref() == Some("Abierta"))
        .count();
    assert_eq!(open, 2);
}

#[test]
fn versions_by_global_id() {
    let old = read_elements(&structure_and_services()).unwrap();
    let new_text = structure_and_services()
        .replace(
            "#100=IFCCARTESIANPOINT((3.5,-1.,2.5))",
            "#100=IFCCARTESIANPOINT((3.5,-0.75,2.5))",
        )
        .replace("'Bajante'", "'Bajante pluviales'");
    let mut new = read_elements(&new_text).unwrap();
    new.elements.retain(|e| e.name != "Viga de atado");
    let extra = read_elements(&ifc(
        &[B(
            "IFCCOLUMN",
            "0k000000000000000000k1",
            "Pilar P-1",
            [0.0, 2.0, 0.0],
            [0.3, 2.3, 3.0],
        )],
        &[],
    ))
    .unwrap();
    new.elements.extend(extra.elements);
    let r = compare(&old, &new);
    assert_eq!(r.unchanged, 1, "{}", r.summary());
    let find = |n: &str| r.changes.iter().find(|c| c.name.starts_with(n)).unwrap();
    assert_eq!(find("Pilar").change, Change::Added);
    assert_eq!(find("Viga de atado").change, Change::Removed);
    match &find("Viga V-1").change {
        Change::Modified {
            moved: Some(d),
            reshaped: false,
            data: false,
        } => assert!((d - 0.25).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        find("Bajante").change,
        Change::Modified {
            moved: None,
            reshaped: false,
            data: true
        }
    );
    assert!(r.summary().contains("movido 0,25 m"));
}

/// `MESH_VOLUMES=<file.ifc>`: prints GlobalId, class and volume of every
/// element, to compare with IfcOpenShell (`scripts/compare_volumes.py`).
#[test]
fn print_volumes() {
    let Ok(path) = std::env::var("MESH_VOLUMES") else {
        return;
    };
    let m = read_elements(&std::fs::read_to_string(path).unwrap()).unwrap();
    for e in &m.elements {
        println!(
            "VOL {} {} {:.9} {}",
            e.global_id,
            e.class,
            e.volume,
            u8::from(e.mesh.approximate)
        );
    }
}
