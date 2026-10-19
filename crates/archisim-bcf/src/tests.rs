use crate::*;

fn sample(version: BcfVersion) -> Bcf {
    let mut vp = Viewpoint::new("a0000000-0000-4000-8000-000000000001".into());
    vp.camera = Some(Camera::Perspective {
        position: [10.0, -8.5, 6.25],
        direction: [-0.6, 0.6, -0.5291503],
        up: [0.0, 0.0, 1.0],
        field_of_view: 60.0,
        aspect_ratio: 1.6,
    });
    vp.selection = vec![
        "2N1qQ0Lv5D0eD9eS7fGgk1".into(),
        "1XbyTwsOb1zBOoFhO$v_2I".into(),
    ];
    vp.default_visibility = false;
    vp.exceptions = vec!["2N1qQ0Lv5D0eD9eS7fGgk1".into()];
    vp.coloring = vec![("FF0000".into(), vec!["2N1qQ0Lv5D0eD9eS7fGgk1".into()])];
    vp.clipping_planes = vec![ClippingPlane {
        location: [0.0, 0.0, 1.1],
        direction: [0.0, 0.0, 1.0],
    }];
    vp.lines = vec![Line {
        start: [0.0, 0.0, 0.0],
        end: [1.0, 2.0, 3.0],
    }];
    vp.snapshot = Some(vec![0x89, b'P', b'N', b'G', 1, 2, 3]);
    let mut vp2 = Viewpoint::new("a0000000-0000-4000-8000-000000000002".into());
    vp2.camera = Some(Camera::Orthogonal {
        position: [5.0, 5.0, 20.0],
        direction: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        view_to_world_scale: 12.5,
        aspect_ratio: 1.6,
    });
    let topic = Topic {
        guid: "b0000000-0000-4000-8000-000000000001".into(),
        topic_type: Some("Interferencia".into()),
        status: Some("Abierta".into()),
        title: "Bajante atraviesa la viga «V-3» & el forjado".into(),
        priority: Some("Alta".into()),
        index: Some(1),
        labels: vec!["Estructura".into(), "Instalaciones".into()],
        creation_date: "2026-10-19T15:30:00Z".into(),
        creation_author: "antonio@ejemplo.es".into(),
        modified_date: Some("2026-10-19T16:00:00Z".into()),
        modified_author: Some("ingeniero@ejemplo.es".into()),
        due_date: Some("2026-10-30T00:00:00Z".into()),
        assigned_to: Some("ingeniero@ejemplo.es".into()),
        stage: Some("Proyecto de ejecución".into()),
        description: Some("Desplazar la bajante 20 cm o abrir un pasatubos.".into()),
        reference_links: vec!["https://example.com/plano-E03".into()],
        related_topics: vec!["b0000000-0000-4000-8000-000000000002".into()],
        files: vec![FileRef {
            ifc_project: Some("0YvctVUKr0kugbFTf53O9L".into()),
            ifc_spatial_element: None,
            is_external: true,
            filename: Some("vivienda.ifc".into()),
            date: Some("2026-10-19T15:00:00Z".into()),
            reference: None,
        }],
        comments: vec![Comment {
            guid: "c0000000-0000-4000-8000-000000000001".into(),
            date: "2026-10-19T15:31:00Z".into(),
            author: "antonio@ejemplo.es".into(),
            text: "Línea 1\nLínea 2 <con> signos".into(),
            viewpoint: Some(vp.guid.clone()),
            modified_date: None,
            modified_author: None,
        }],
        viewpoints: vec![vp, vp2],
    };
    let mut other = Topic {
        guid: "b0000000-0000-4000-8000-000000000002".into(),
        title: "Segunda".into(),
        index: Some(2),
        creation_date: "2026-10-19T15:40:00Z".into(),
        creation_author: "antonio@ejemplo.es".into(),
        ..Topic::default()
    };
    other.status = Some("Cerrada".into());
    other.topic_type = Some("Consulta".into());
    Bcf {
        version,
        project_id: Some("d0000000-0000-4000-8000-000000000001".into()),
        project_name: Some("Vivienda".into()),
        extensions: if version == BcfVersion::V3_0 {
            Extensions::archisim()
        } else {
            Extensions::default()
        },
        topics: vec![topic, other],
    }
}

#[test]
fn round_trip_3_0() {
    let bcf = sample(BcfVersion::V3_0);
    let bytes = write_bcf(&bcf);
    // For scripts/validate_bcf.py.
    if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("ejemplo.bcf"), &bytes).unwrap();
    }
    let back = read_bcf(&bytes).unwrap();
    assert_eq!(back, bcf);
    // Same content, same bytes.
    assert_eq!(write_bcf(&back), bytes);
}

#[test]
fn round_trip_2_1() {
    let mut bcf = sample(BcfVersion::V2_1);
    let bytes = write_bcf(&bcf);
    let back = read_bcf(&bytes).unwrap();
    // 2.1 has no aspect ratio: the reader gives the default.
    for vp in bcf.topics.iter_mut().flat_map(|t| t.viewpoints.iter_mut()) {
        match &mut vp.camera {
            Some(Camera::Perspective { aspect_ratio, .. })
            | Some(Camera::Orthogonal { aspect_ratio, .. }) => *aspect_ratio = 1.5,
            None => {}
        }
    }
    assert_eq!(back, bcf);
}

/// buildingSMART's BCF test cases (`BCF_TESTCASES=<BCF-XML>/Test Cases`):
/// every 2.1 and 3.0 file reads, and writes back to the same content.
#[test]
fn buildingsmart_files() {
    let Ok(dir) = std::env::var("BCF_TESTCASES") else {
        return;
    };
    let mut n = 0;
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                if !p.ends_with("unzipped") && !p.ends_with("v2.0") {
                    stack.push(p);
                }
                continue;
            }
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
            if ext != "bcf" && ext != "bcfzip" {
                continue;
            }
            let bytes = std::fs::read(&p).unwrap();
            let bcf = read_bcf(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(!bcf.topics.is_empty(), "{}", p.display());
            for t in &bcf.topics {
                assert!(!t.title.is_empty(), "{}", p.display());
            }
            let again = read_bcf(&write_bcf(&bcf)).unwrap();
            assert_eq!(again, bcf, "{}", p.display());
            n += 1;
        }
    }
    eprintln!("BCF de buildingSMART leídos y reescritos: {n}");
    assert!(n > 20);
}
