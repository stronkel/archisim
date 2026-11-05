//! The light frames as a 3D DXF of axes, for a structural program: every
//! stud, track, header, joist, rafter and tie as a 3D line from end to
//! end through the middle of its section, on a layer per section
//! («C 150×40×15×1,5», «45×145 C24»), with the kind of member in the
//! layer's name after it. Metres; z up. AutoCAD R12 text, the simplest
//! form every program reads.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use archisim_core::Model;
use archisim_geom::framing::all_members;

/// Layer names: letters, digits, spaces and a few signs; the rest become
/// `_` (R12 does not take `×` or `,`).
fn layer(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '×' => 'x',
            ',' => '.',
            c if c.is_ascii_alphanumeric() || " -_.".contains(c) => c,
            _ => '_',
        })
        .collect::<String>()
        .to_uppercase()
}

fn ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            'ñ' => 'n',
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' => 'O',
            'Ú' => 'U',
            'Ñ' => 'N',
            c => c,
        })
        .collect()
}

/// The axes of every framed member as a DXF, `None` if nothing is framed.
pub fn framing_dxf(model: &Model) -> Option<String> {
    let members = all_members(model);
    if members.is_empty() {
        return None;
    }
    let name = |m: &archisim_geom::framing::Member| {
        layer(&ascii(&format!(
            "{} - {}",
            m.section.name,
            m.role.name(m.material)
        )))
    };
    let layers: BTreeSet<String> = members.iter().map(name).collect();
    let mut s = String::new();
    let mut put = |code: i32, v: &str| {
        let _ = writeln!(s, "{code}\n{v}");
    };
    put(0, "SECTION");
    put(2, "HEADER");
    put(9, "$ACADVER");
    put(1, "AC1009");
    put(9, "$INSUNITS");
    put(70, "6");
    put(0, "ENDSEC");
    put(0, "SECTION");
    put(2, "TABLES");
    put(0, "TABLE");
    put(2, "LAYER");
    put(70, &layers.len().to_string());
    for (i, l) in layers.iter().enumerate() {
        put(0, "LAYER");
        put(2, l);
        put(70, "0");
        put(62, &(1 + i % 254).to_string());
        put(6, "CONTINUOUS");
    }
    put(0, "ENDTAB");
    put(0, "ENDSEC");
    put(0, "SECTION");
    put(2, "ENTITIES");
    let f = |v: f64| format!("{:.4}", v);
    for m in &members {
        put(0, "LINE");
        put(8, &name(m));
        put(10, &f(m.start[0]));
        put(20, &f(m.start[1]));
        put(30, &f(m.start[2]));
        put(11, &f(m.end[0]));
        put(21, &f(m.end[1]));
        put(31, &f(m.end[2]));
    }
    put(0, "ENDSEC");
    put(0, "EOF");
    Some(s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn layer_names_are_plain() {
        assert_eq!(
            super::layer("C 150×40×15×1,5 - Montante"),
            "C 150X40X15X1.5 - MONTANTE"
        );
        assert_eq!(
            super::ascii("Montante de apoyo del dintel ñ"),
            "Montante de apoyo del dintel n"
        );
    }
}
