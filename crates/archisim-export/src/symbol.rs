//! Plan symbols for object types from DXF drawings (bloque 6 de la
//! revisión del 22/10/2026): the trees, shrubs, people or furniture an
//! office keeps drawn in CAD, brought in as the plan drawing of a type.
//! The whole drawing is the symbol (blocks exploded); texts and
//! dimensions are left out. It is centred on its middle and, stretched,
//! made to fit a square of side 1, so the type draws it as large as
//! itself.

use archisim_core::{PlanSymbol, Point2, Shape, SymbolItem};
use archisim_draft::{transform_shape, Transform};

use crate::dxf_read;

/// The points that bound a shape (enough for its box).
fn extent_points(shape: &Shape) -> Vec<Point2> {
    let ring = |c: Point2, r: f64| {
        vec![
            c + archisim_core::Vec2::new(r, r),
            c - archisim_core::Vec2::new(r, r),
        ]
    };
    match shape {
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline(p) => p.points.clone(),
        Shape::Circle { centre, radius } | Shape::Arc { centre, radius, .. } => {
            ring(*centre, *radius)
        }
        Shape::Ellipse { centre, axis, .. } => ring(*centre, axis.length()),
        Shape::Point { at } => vec![*at],
        Shape::Hatch(h) => h.loops.iter().flat_map(|l| l.points.clone()).collect(),
        Shape::Text(_) | Shape::Dim(_) => Vec::new(),
    }
}

/// A symbol from a DXF file's bytes: its lines, arcs, circles, polylines
/// and fills, each with the colour and weight it has in the drawing.
pub fn symbol_from_dxf(bytes: &[u8], source: &str, fit: bool) -> Result<PlanSymbol, String> {
    let d = dxf_read::read(bytes)?;
    let layer = |name: &str| d.layers.iter().find(|l| l.name == name);
    let mut items: Vec<SymbolItem> = d
        .items
        .iter()
        .filter(|i| !matches!(i.shape, Shape::Text(_) | Shape::Dim(_)))
        .filter(|i| layer(&i.layer).is_none_or(|l| l.visible))
        .map(|i| SymbolItem {
            shape: i.shape.clone(),
            color: i.color.or_else(|| layer(&i.layer).map(|l| l.color)),
            weight: i.weight.or_else(|| layer(&i.layer).map(|l| l.weight)),
        })
        .collect();
    let pts: Vec<Point2> = items.iter().flat_map(|i| extent_points(&i.shape)).collect();
    let Some(first) = pts.first().copied() else {
        return Err("el dibujo no tiene líneas, arcos ni rellenos".into());
    };
    let (lo, hi) = pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    });
    let size = (hi.x - lo.x).max(hi.y - lo.y);
    if size < 1e-9 {
        return Err("el dibujo no tiene tamaño".into());
    }
    let centre = Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let origin = Point2::new(0.0, 0.0);
    let mut t = Transform::translate(origin - centre);
    if fit {
        t = t.then(&Transform::scale(origin, 1.0 / size));
    }
    for i in &mut items {
        i.shape = transform_shape(&i.shape, &t);
    }
    Ok(PlanSymbol {
        source: source.to_string(),
        fit,
        items,
    })
}

/// A drawing for elevations from a DXF (C1, 24/10/2026): the whole
/// drawing (texts and dimensions left out), its middle on `x` = 0, its
/// lowest point on `y` = 0 and 1 high, so the object draws it as tall as
/// itself. `side`: it shows the object seen from its side.
pub fn elevation_from_dxf(
    bytes: &[u8],
    source: &str,
    side: bool,
) -> Result<archisim_core::ElevationFigure, String> {
    let sym = symbol_from_dxf(bytes, source, false)?;
    let pts: Vec<Point2> = sym
        .items
        .iter()
        .flat_map(|i| extent_points(&i.shape))
        .collect();
    let Some(first) = pts.first().copied() else {
        return Err("el dibujo no tiene líneas, arcos ni rellenos".into());
    };
    let (lo, hi) = pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    });
    let height = hi.y - lo.y;
    if height < 1e-9 {
        return Err("el dibujo no tiene altura".into());
    }
    let origin = Point2::new(0.0, 0.0);
    let foot = Point2::new((lo.x + hi.x) / 2.0, lo.y);
    let t = Transform::translate(origin - foot).then(&Transform::scale(origin, 1.0 / height));
    let items = sym
        .items
        .into_iter()
        .map(|i| SymbolItem {
            shape: transform_shape(&i.shape, &t),
            ..i
        })
        .collect();
    Ok(archisim_core::ElevationFigure {
        source: source.to_string(),
        side,
        art: archisim_core::ElevationArt::Lines(items),
    })
}

/// A picture for elevations from a PNG (with transparency): kept as it
/// is, with its width over its height.
pub fn elevation_from_png(
    bytes: &[u8],
    source: &str,
    side: bool,
) -> Result<archisim_core::ElevationFigure, String> {
    // The size is in the first chunk (IHDR), after the signature.
    const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err("no es una imagen PNG".into());
    }
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let (w, h) = (be(&bytes[16..20]), be(&bytes[20..24]));
    if w == 0 || h == 0 {
        return Err("la imagen no tiene tamaño".into());
    }
    Ok(archisim_core::ElevationFigure {
        source: source.to_string(),
        side,
        art: archisim_core::ElevationArt::Image {
            png: std::sync::Arc::new(bytes.to_vec()),
            aspect: w as f64 / h as f64,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_stands_for_an_elevation() {
        // A trunk and a crown, 6 high, away from the origin.
        let dxf = "0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\nT\n10\n10\n20\n5\n30\n0\n11\n10\n21\n8\n31\n0\n\
0\nCIRCLE\n8\nC\n10\n10\n20\n9\n30\n0\n40\n2\n\
0\nENDSEC\n0\nEOF\n";
        let f = elevation_from_dxf(dxf.as_bytes(), "arbol.dxf", false).unwrap();
        let archisim_core::ElevationArt::Lines(items) = &f.art else {
            panic!("{f:?}");
        };
        match &items[0].shape {
            Shape::Line { a, b } => {
                assert!(a.distance(Point2::new(0.0, 0.0)) < 1e-9, "{a:?}");
                assert!((b.y - 0.5).abs() < 1e-9, "{b:?}");
            }
            other => panic!("{other:?}"),
        }
        assert!((f.art.aspect() - 4.0 / 6.0).abs() < 1e-9);
        // A PNG of 40 × 80: half as wide as high.
        let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&40u32.to_be_bytes());
        png.extend_from_slice(&80u32.to_be_bytes());
        let f = elevation_from_png(&png, "p.png", true).unwrap();
        assert!((f.art.aspect() - 0.5).abs() < 1e-12);
        assert!(f.side);
        assert!(elevation_from_png(b"GIF89a", "x", false).is_err());
    }

    #[test]
    fn a_drawing_becomes_a_symbol() {
        // A circle of 2 m and a cross, drawn away from the origin, in metres.
        let dxf =
            "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
0\nCIRCLE\n8\nCOPA\n62\n3\n10\n10\n20\n20\n30\n0\n40\n1\n\
0\nLINE\n8\nCOPA\n10\n9\n20\n20\n30\n0\n11\n11\n21\n20\n31\n0\n\
0\nTEXT\n8\nTXT\n10\n10\n20\n20\n30\n0\n40\n0.2\n1\nPino\n\
0\nENDSEC\n0\nEOF\n";
        let s = symbol_from_dxf(dxf.as_bytes(), "pino.dxf", true).unwrap();
        assert_eq!(s.items.len(), 2, "the text is left out");
        match &s.items[0].shape {
            Shape::Circle { centre, radius } => {
                assert!(centre.distance(Point2::new(0.0, 0.0)) < 1e-9);
                assert!((radius - 0.5).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        assert!(s.items[0].color.is_some());
        let real = symbol_from_dxf(dxf.as_bytes(), "pino.dxf", false).unwrap();
        match &real.items[0].shape {
            Shape::Circle { radius, .. } => assert!((radius - 1.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
        assert!(
            symbol_from_dxf(b"0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n", "x", true).is_err()
        );
    }
}
