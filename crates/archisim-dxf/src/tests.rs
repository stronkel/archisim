use super::*;

fn fixture(name: &str) -> Drawing {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    read_bytes(&std::fs::read(path).unwrap()).unwrap()
}

/// The hill of the fixtures (scripts/make_dxf_fixtures.py).
fn hill(x: f64, y: f64) -> f64 {
    645.0 - 0.004 * ((x - 431250.0).powi(2) + (y - 4581300.0).powi(2))
}

/// R12: points with their height, 2D polylines at an elevation (contour
/// lines), a 3D polyline, height labels and a building at height 0.
#[test]
fn survey_from_an_r12_drawing() {
    let d = fixture("levantamiento_r12.dxf");
    let layers = survey::layers(&d);
    let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        ["COTAS", "CURVAS", "EDIFICACION", "PUNTOS", "RUPTURA"]
    );
    let get = |n: &str| layers.iter().find(|l| l.name == n).unwrap();
    assert_eq!(get("PUNTOS").points, 49);
    assert_eq!(get("CURVAS").contours, 4);
    assert_eq!(get("COTAS").numbers, 49);
    assert_eq!(get("RUPTURA").points, 11);
    assert!(!get("EDIFICACION").looks_topographic());
    assert_eq!(get("EDIFICACION").flat, 8);
    // All of it: the building's flat corners are left out.
    let s = survey(&d, &SurveyOptions::default());
    assert_eq!(s.points.len(), 49 + 11);
    assert_eq!(s.lines.len(), 4);
    for p in &s.points {
        assert!((p[2] - hill(p[0], p[1])).abs() < 0.002, "{p:?}");
    }
    // Contour lines closed, at their height.
    for l in &s.lines {
        assert_eq!(l.first(), l.last());
        let z = l[0][2];
        assert!([641.0, 642.0, 643.0, 644.0].contains(&z));
        assert!(l.iter().all(|p| p[2] == z));
    }
    assert_eq!(s.contour_interval(), Some(1.0));
    let (lo, hi) = s.z_range().unwrap();
    assert!(lo < 641.0 && hi <= 645.0);
    // Only the points' layer, thinned to 15 m: one in four or so.
    let only = survey(
        &d,
        &SurveyOptions {
            layers: vec!["PUNTOS".into()],
            min_spacing: 15.0,
            ..SurveyOptions::default()
        },
    );
    assert!(only.lines.is_empty());
    assert!(
        only.points.len() < 49 && only.points.len() >= 9,
        "{}",
        only.points.len()
    );
}

/// R2018: contour lines as LWPOLYLINE with elevation, points as blocks
/// whose height is in an attribute, and a TIN of 3D faces.
#[test]
fn survey_from_an_r2018_drawing() {
    let d = fixture("levantamiento_2018.dxf");
    assert_eq!(d.unit, Some(1.0));
    let s = survey(&d, &SurveyOptions::default());
    assert_eq!(s.lines.len(), 3);
    assert_eq!(s.contour_interval(), Some(1.0));
    // 12 blocks with their height from COTA, and the 3 + 3 corners of the
    // two faces.
    assert_eq!(s.points.len(), 12 + 6);
    for p in &s.points {
        assert!((p[2] - hill(p[0], p[1])).abs() < 0.002, "{p:?}");
    }
}

/// Points drawn flat take the height of the label next to them.
#[test]
fn heights_from_the_labels() {
    let text = "  0\nSECTION\n  2\nENTITIES\n  0\nPOINT\n  8\nP\n 10\n100.0\n 20\n200.0\n 30\n0.0\n  0\nTEXT\n  8\nP\n 10\n100.5\n 20\n200.4\n 30\n0.0\n  1\n643,25\n  0\nPOINT\n  8\nP\n 10\n150.0\n 20\n200.0\n 30\n0.0\n  0\nMTEXT\n  8\nP\n 10\n150.3\n 20\n199.8\n 30\n0.0\n  1\n{\\fArial|b0;+644.10 m}\n  0\nENDSEC\n  0\nEOF\n";
    let d = read(text).unwrap();
    assert_eq!(survey(&d, &SurveyOptions::default()).points.len(), 0);
    let s = survey(
        &d,
        &SurveyOptions {
            heights_from_texts: true,
            ..SurveyOptions::default()
        },
    );
    assert_eq!(
        s.points,
        vec![[100.0, 200.0, 643.25], [150.0, 200.0, 644.10]]
    );
    assert_eq!(read("hola"), Err(DxfError::NotDxf));
    assert!(read_bytes(b"AutoCAD Binary DXF\r\n\x1a\0").is_err());
}

/// Lists of points as text: with a point number and a code, decimal
/// commas, a heading; northing first if asked.
#[test]
fn points_from_csv_and_txt() {
    let csv = "N;X;Y;Z;COD\n1;431250,10;4581300,25;643,18;TN\n2;431260;4581301;642,9;TN\n";
    let (p, skipped) = points_from_text(csv, false);
    assert_eq!(skipped, 1);
    assert_eq!(
        p,
        vec![
            [431250.10, 4581300.25, 643.18],
            [431260.0, 4581301.0, 642.9]
        ]
    );
    let txt = "431250.1 4581300.25 643.18\n431260,4581301,642.9\n\nfin\n";
    let (p, skipped) = points_from_text(txt, false);
    assert_eq!((p.len(), skipped), (2, 1));
    assert_eq!(p[1], [431260.0, 4581301.0, 642.9]);
    let (p, _) = points_from_text("4581300 431250 643\n", true);
    assert_eq!(p, vec![[431250.0, 4581300.0, 643.0]]);
}
