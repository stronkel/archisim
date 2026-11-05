//! A small Excel workbook writer (Office Open XML, `.xlsx`): sheets of
//! rows of text and numbers, the first row of each in bold, stored in a
//! zip without compression. Enough for breakdowns and lists that are to be
//! opened, sorted and summed in Excel, LibreOffice or Numbers.
//!
//! [`framing_workbook`] writes the breakdown of the light frames.

use std::fmt::Write as _;

/// A cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
    Empty,
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        Cell::Text(s.to_string())
    }
}
impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::Text(s)
    }
}
impl From<f64> for Cell {
    fn from(v: f64) -> Self {
        Cell::Number(v)
    }
}
impl From<usize> for Cell {
    fn from(v: usize) -> Self {
        Cell::Number(v as f64)
    }
}

/// A sheet: its name and its rows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    /// Width of each column, in characters.
    pub widths: Vec<f64>,
}

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn column(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Sheet names: no more than 31 characters, none of `[]:*?/\`, unique.
fn sheet_name(name: &str, taken: &[String]) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !"[]:*?/\\".contains(*c))
        .take(28)
        .collect();
    let clean = if clean.trim().is_empty() {
        "Hoja".to_string()
    } else {
        clean
    };
    let mut out = clean.clone();
    let mut k = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&out)) {
        out = format!("{clean} {k}");
        k += 1;
    }
    out
}

fn sheet_xml(sheet: &Sheet) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    if !sheet.rows.is_empty() {
        s.push_str(r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>"#);
    }
    if !sheet.widths.is_empty() {
        s.push_str("<cols>");
        for (i, w) in sheet.widths.iter().enumerate() {
            let _ = write!(
                s,
                r#"<col min="{0}" max="{0}" width="{1}" customWidth="1"/>"#,
                i + 1,
                w
            );
        }
        s.push_str("</cols>");
    }
    s.push_str("<sheetData>");
    for (r, row) in sheet.rows.iter().enumerate() {
        let _ = write!(s, r#"<row r="{}">"#, r + 1);
        let style = if r == 0 { r#" s="1""# } else { "" };
        for (c, cell) in row.iter().enumerate() {
            let at = format!("{}{}", column(c), r + 1);
            match cell {
                Cell::Text(t) => {
                    let _ = write!(
                        s,
                        r#"<c r="{at}" t="inlineStr"{style}><is><t xml:space="preserve">{}</t></is></c>"#,
                        esc(t)
                    );
                }
                Cell::Number(v) if v.is_finite() => {
                    let _ = write!(s, r#"<c r="{at}"{style}><v>{v}</v></c>"#);
                }
                _ => {}
            }
        }
        s.push_str("</row>");
    }
    s.push_str("</sheetData></worksheet>");
    s
}

/// The workbook with `sheets`, as the bytes of an `.xlsx` file.
pub fn workbook(sheets: &[Sheet]) -> Vec<u8> {
    let mut names: Vec<String> = Vec::new();
    for sh in sheets {
        let n = sheet_name(&sh.name, &names);
        names.push(n);
    }
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    );
    let mut book = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
    );
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    let count = sheets.len().max(1);
    for i in 0..count {
        let _ = write!(
            types,
            r#"<Override PartName="/xl/worksheets/sheet{0}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
            i + 1
        );
        let name = names.get(i).cloned().unwrap_or_else(|| "Hoja".into());
        let _ = write!(
            book,
            r#"<sheet name="{}" sheetId="{1}" r:id="rId{1}"/>"#,
            esc(&name),
            i + 1
        );
        let _ = write!(
            rels,
            r#"<Relationship Id="rId{0}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{0}.xml"/>"#,
            i + 1
        );
        let xml = sheets
            .get(i)
            .map_or_else(|| sheet_xml(&Sheet::default()), sheet_xml);
        files.push((
            format!("xl/worksheets/sheet{}.xml", i + 1),
            xml.into_bytes(),
        ));
    }
    types.push_str("</Types>");
    book.push_str("</sheets></workbook>");
    let _ = write!(
        rels,
        r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
        count + 1
    );
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    let mut all = vec![
        ("[Content_Types].xml".to_string(), types.into_bytes()),
        ("_rels/.rels".to_string(), root.as_bytes().to_vec()),
        ("xl/workbook.xml".to_string(), book.into_bytes()),
        ("xl/_rels/workbook.xml.rels".to_string(), rels.into_bytes()),
        ("xl/styles.xml".to_string(), styles.as_bytes().to_vec()),
    ];
    all.extend(files);
    zip_stored(&all)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// A zip of `files`, stored (no compression).
fn zip_stored(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        // Local header.
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0x0800);
        le16(&mut out, 0);
        le16(&mut out, 0);
        le16(&mut out, 0x21);
        le32(&mut out, crc);
        le32(&mut out, size);
        le32(&mut out, size);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // Its entry in the central directory.
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0x0800);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0x21);
        le32(&mut central, crc);
        le32(&mut central, size);
        le32(&mut central, size);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let start = out.len() as u32;
    let size = central.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, files.len() as u16);
    le16(&mut out, files.len() as u16);
    le32(&mut out, size);
    le32(&mut out, start);
    le16(&mut out, 0);
    out
}

/// The breakdown of the light frames of `model` as a workbook: the cut
/// list, the totals with the bars and boards to buy, the panels, and every
/// member with its element and panel. `None` if nothing is framed.
pub fn framing_workbook(model: &archisim_core::Model) -> Option<Vec<u8>> {
    use archisim_geom::framing::{all_members, cut_list, wall_panels, BAR_STEEL, BAR_TIMBER};
    let members = all_members(model);
    if members.is_empty() {
        return None;
    }
    let list = cut_list(model, &members);
    let r3 = |v: f64| (v * 1000.0).round() / 1000.0;
    let mut cut = Sheet {
        name: "Lista de corte".into(),
        rows: vec![vec![
            "Material".into(),
            "Perfil".into(),
            "Pieza".into(),
            "Longitud (mm)".into(),
            "Unidades".into(),
            "Total (m)".into(),
            "Peso (kg)".into(),
            "Volumen (m³)".into(),
        ]],
        widths: vec![14.0, 20.0, 30.0, 14.0, 10.0, 11.0, 11.0, 13.0],
    };
    for l in &list.lines {
        let steel = l.material == archisim_core::FrameMaterial::Steel;
        cut.rows.push(vec![
            if steel { "Acero" } else { "Madera" }.into(),
            l.section.clone().into(),
            l.role.name(l.material).into(),
            (l.length_mm as f64).into(),
            l.count.into(),
            r3(l.metres()).into(),
            if steel {
                r3(l.kg()).into()
            } else {
                Cell::Empty
            },
            if steel {
                Cell::Empty
            } else {
                r3(l.m3()).into()
            },
        ]);
    }
    let mut totals = Sheet {
        name: "Totales y compra".into(),
        rows: vec![vec![
            "Concepto".into(),
            "Piezas".into(),
            "Metros".into(),
            "Peso (kg)".into(),
            "Volumen (m³)".into(),
            "Compra".into(),
            "Unidades de compra".into(),
        ]],
        widths: vec![36.0, 9.0, 10.0, 11.0, 13.0, 26.0, 18.0],
    };
    for t in &list.totals {
        let steel = t.material == archisim_core::FrameMaterial::Steel;
        let bar = if steel { BAR_STEEL } else { BAR_TIMBER };
        totals.rows.push(vec![
            t.section.clone().into(),
            t.pieces.into(),
            r3(t.metres).into(),
            if steel { r3(t.kg).into() } else { Cell::Empty },
            if steel { Cell::Empty } else { r3(t.m3).into() },
            format!("Barras de {} m", bar).replace('.', ",").into(),
            t.bars.into(),
        ]);
    }
    for b in &list.boards {
        totals.rows.push(vec![
            format!("{} ({} m²)", b.name, r3(b.area)).into(),
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
            format!("Tableros de {} × {} m", b.board.0, b.board.1)
                .replace('.', ",")
                .into(),
            b.boards.into(),
        ]);
    }
    for (what, n) in [
        ("Tornillos autotaladrantes (estimación)", list.screws),
        ("Clavos (estimación)", list.nails),
        ("Anclajes de los muros", list.anchors),
    ] {
        if n > 0 {
            totals.rows.push(vec![
                what.into(),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                "Unidades".into(),
                n.into(),
            ]);
        }
    }
    let mut panels = Sheet {
        name: "Paneles".into(),
        rows: vec![vec![
            "Panel".into(),
            "Muro".into(),
            "Longitud (m)".into(),
            "Altura (m)".into(),
            "Piezas".into(),
            "Peso (kg)".into(),
        ]],
        widths: vec![10.0, 10.0, 13.0, 11.0, 9.0, 11.0],
    };
    for p in wall_panels(model) {
        panels.rows.push(vec![
            p.mark.clone().into(),
            (p.wall.0 as f64).into(),
            r3(p.length()).into(),
            r3(p.height()).into(),
            p.members.len().into(),
            (p.weight(model).round()).into(),
        ]);
    }
    let mut pieces = Sheet {
        name: "Piezas".into(),
        rows: vec![vec![
            "Elemento".into(),
            "Panel".into(),
            "Pieza".into(),
            "Perfil".into(),
            "Longitud (mm)".into(),
            "X inicio".into(),
            "Y inicio".into(),
            "Z inicio".into(),
            "X fin".into(),
            "Y fin".into(),
            "Z fin".into(),
        ]],
        widths: vec![14.0, 8.0, 30.0, 20.0, 14.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0],
    };
    for m in &members {
        let owner = match m.owner {
            archisim_geom::solid::SolidOwner::Wall(id) => format!("Muro {}", id.0),
            archisim_geom::solid::SolidOwner::Slab(id) => format!("Forjado {}", id.0),
            archisim_geom::solid::SolidOwner::Roof(id) => format!("Cubierta {}", id.0),
            o => format!("{}", o.id()),
        };
        pieces.rows.push(vec![
            owner.into(),
            m.panel.map_or(Cell::Empty, |p| (p + 1).into()),
            m.role.name(m.material).into(),
            m.section.name.clone().into(),
            (m.length() * 1000.0).round().into(),
            r3(m.start[0]).into(),
            r3(m.start[1]).into(),
            r3(m.start[2]).into(),
            r3(m.end[0]).into(),
            r3(m.end[1]).into(),
            r3(m.end[2]).into(),
        ]);
    }
    Some(workbook(&[cut, totals, panels, pieces]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workbook_is_a_zip_with_its_sheets() {
        let book = workbook(&[Sheet {
            name: "Lista: de corte".into(),
            rows: vec![
                vec!["A & B".into(), 3.5.into()],
                vec![Cell::Empty, "x".into()],
            ],
            widths: vec![10.0],
        }]);
        assert_eq!(&book[..4], b"PK\x03\x04");
        let text = String::from_utf8_lossy(&book);
        assert!(text.contains("A &amp; B") && text.contains("<v>3.5</v>"));
        assert!(text.contains(r#"name="Lista de corte""#));
        assert_eq!(column(0), "A");
        assert_eq!(column(27), "AB");
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
