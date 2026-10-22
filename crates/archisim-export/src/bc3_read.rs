//! Price banks in BC3 (FIEBDC-3), as the regional banks and the budgeting
//! programs publish them (bloque 7 de la revisión del 22/10/2026): the
//! concepts (`~C`: code, unit, summary, price, type), their long texts
//! (`~T`) and how they are made up (`~D`: chapters and the pieces of each
//! item, whose prices give the item's when it has none of its own).

use std::collections::BTreeMap;

/// A concept of the bank.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BankConcept {
    pub code: String,
    pub unit: String,
    pub summary: String,
    /// Euros per unit (worked out from its pieces when the bank gives
    /// none).
    pub price: f64,
    /// Its long description.
    pub text: String,
    /// FIEBDC type: 0 none (items), 1 labour, 2 machinery, 3 material.
    pub kind: u8,
    /// A chapter (its code ends in `#`).
    pub chapter: bool,
}

/// A price bank read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bank {
    /// Its name (the root concept's summary, or the `~V` record's).
    pub name: String,
    /// Code of the root chapter.
    pub root: String,
    pub concepts: BTreeMap<String, BankConcept>,
    /// What each concept is made of: (code, factor × yield), in order.
    pub children: BTreeMap<String, Vec<(String, f64)>>,
}

impl Bank {
    /// The items (concepts that are neither chapters nor basic labour,
    /// machinery or materials) whose code, summary or text hold every word
    /// of `query`, those in `unit` first; at most `limit`.
    pub fn search(&self, query: &str, unit: Option<&str>, limit: usize) -> Vec<&BankConcept> {
        let words: Vec<String> = query
            .split_whitespace()
            .map(fold)
            .filter(|w| !w.is_empty())
            .collect();
        let mut found: Vec<&BankConcept> = self
            .concepts
            .values()
            .filter(|c| !c.chapter && c.kind == 0 && !c.unit.is_empty())
            .filter(|c| {
                if words.is_empty() {
                    return true;
                }
                let hay = fold(&format!("{} {} {}", c.code, c.summary, c.text));
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .collect();
        if let Some(u) = unit {
            let u = unit_key(u);
            found.sort_by_key(|c| unit_key(&c.unit) != u);
        }
        found.truncate(limit);
        found
    }

    /// The chapters above a concept, from the root: their summaries.
    pub fn path_of(&self, code: &str) -> Vec<String> {
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        for (p, kids) in &self.children {
            for (k, _) in kids {
                parent.entry(k.as_str()).or_insert(p.as_str());
            }
        }
        let mut path = Vec::new();
        let mut at = code;
        for _ in 0..16 {
            match parent.get(at) {
                Some(p) if *p != self.root => {
                    if let Some(c) = self.concepts.get(*p) {
                        path.push(c.summary.clone());
                    }
                    at = p;
                }
                _ => break,
            }
        }
        path.reverse();
        path
    }
}

/// Lower case and without accents, for searching.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            c => c,
        })
        .collect()
}

/// A unit as it compares: `m²`, `M2` and `m2` are the same.
pub fn unit_key(u: &str) -> String {
    fold(u.trim())
        .replace('²', "2")
        .replace('³', "3")
        .replace(['.', ' '], "")
        .replace("ud", "u")
}

/// Why a bank could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct BankError(pub String);

impl std::fmt::Display for BankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BankError {}

/// Reads a BC3 file.
pub fn read_bank(bytes: &[u8]) -> Result<Bank, BankError> {
    let text = decode(bytes);
    if !text.contains("~C|") {
        return Err(BankError(
            "no es un fichero BC3 (FIEBDC-3): no tiene conceptos".into(),
        ));
    }
    let mut bank = Bank::default();
    let mut given_price: BTreeMap<String, bool> = BTreeMap::new();
    let mut version_name = String::new();
    for record in text.split('~').skip(1) {
        let mut fields = record.split('|');
        let tag = fields.next().unwrap_or("").trim();
        let fields: Vec<&str> = fields.collect();
        let field = |i: usize| fields.get(i).copied().unwrap_or("").trim();
        match tag {
            "V" => version_name = field(3).to_string(),
            "C" => {
                // Several codes may name the same concept: the first.
                let code = clean_code(field(0).split('\\').next().unwrap_or(""));
                if code.is_empty() {
                    continue;
                }
                let price_text = field(3).split('\\').next().unwrap_or("").trim();
                let price = price_text.replace(',', ".").parse::<f64>().ok();
                given_price.insert(
                    code.clone(),
                    price.is_some_and(|p| p > 0.0) && !price_text.is_empty(),
                );
                let chapter = field(0).contains('#');
                if field(0).contains("##") && bank.root.is_empty() {
                    bank.root = code.clone();
                }
                bank.concepts.insert(
                    code.clone(),
                    BankConcept {
                        code,
                        unit: field(1).to_string(),
                        summary: field(2).to_string(),
                        price: price.filter(|p| p.is_finite()).unwrap_or(0.0),
                        text: String::new(),
                        kind: field(5).parse().unwrap_or(0),
                        chapter,
                    },
                );
            }
            "T" => {
                let code = clean_code(field(0));
                let text = fields.get(1).copied().unwrap_or("").trim().to_string();
                bank.concepts.entry(code).or_default().text = text;
            }
            "D" => {
                let parent = clean_code(field(0));
                let parts: Vec<&str> = field(1).split('\\').collect();
                let mut kids = Vec::new();
                for chunk in parts.chunks(3) {
                    let code = clean_code(chunk[0]);
                    if code.is_empty() {
                        continue;
                    }
                    let num = |i: usize| {
                        chunk
                            .get(i)
                            .and_then(|s| s.trim().replace(',', ".").parse::<f64>().ok())
                            .unwrap_or(1.0)
                    };
                    kids.push((code, num(1) * num(2)));
                }
                bank.children.insert(parent, kids);
            }
            _ => {}
        }
    }
    if bank.concepts.is_empty() {
        return Err(BankError("el fichero no tiene conceptos".into()));
    }
    // The items without a price of their own: the sum of their pieces.
    let codes: Vec<String> = bank.concepts.keys().cloned().collect();
    let mut memo: BTreeMap<String, f64> = BTreeMap::new();
    for code in &codes {
        if !given_price.get(code).copied().unwrap_or(false) {
            let p = price_of(&bank, &given_price, code, &mut memo, 0);
            if let Some(c) = bank.concepts.get_mut(code) {
                c.price = p;
            }
        }
    }
    bank.name = bank
        .concepts
        .get(&bank.root)
        .map(|c| c.summary.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or(version_name);
    Ok(bank)
}

fn price_of(
    bank: &Bank,
    given: &BTreeMap<String, bool>,
    code: &str,
    memo: &mut BTreeMap<String, f64>,
    depth: usize,
) -> f64 {
    if let Some(p) = memo.get(code) {
        return *p;
    }
    let own = bank.concepts.get(code).map_or(0.0, |c| c.price);
    if depth > 12 || given.get(code).copied().unwrap_or(false) {
        return own;
    }
    let p = match bank.children.get(code) {
        Some(kids) if !kids.is_empty() => kids
            .iter()
            .map(|(k, f)| f * price_of(bank, given, k, memo, depth + 1))
            .sum(),
        _ => own,
    };
    memo.insert(code.to_string(), p);
    p
}

fn clean_code(s: &str) -> String {
    s.trim().trim_end_matches('#').trim().to_string()
}

/// The text of the file: UTF-8 if it is, otherwise the charset its `~V`
/// record names (ANSI, the default, or the DOS code page 850 or 437).
fn decode(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let head: String = bytes.iter().take(400).map(|&b| b as char).collect();
    let dos = head
        .split('~')
        .find(|r| r.starts_with("V|"))
        .is_some_and(|v| v.contains("|850|") || v.contains("|437|"));
    bytes
        .iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else if dos {
                CP850[(b - 0x80) as usize]
            } else {
                CP1252
                    .get((b as usize).wrapping_sub(0x80))
                    .copied()
                    .filter(|_| b < 0xA0)
                    .unwrap_or(b as char)
            }
        })
        .collect()
}

/// Windows-1252 from 0x80 to 0x9F (above, it is Latin-1).
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

/// DOS code page 850 from 0x80.
const CP850: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©',
    '╣', '║', '╗', '╝', '¢', '¥', '┐', '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '¤', 'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì',
    '▀', 'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´', '\u{AD}',
    '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{A0}',
];

#[cfg(test)]
mod tests {
    use super::*;

    /// A small bank written by hand, as the regional banks write theirs.
    const SAMPLE: &str = "~V|Prueba|FIEBDC-3/2016|Banco de prueba|1.0|ANSI|\r\n\
~K|\\2\\2\\3\\2\\2\\2\\2\\EUR\\|\r\n\
~C|BANCO##||Banco de prueba 2026|0|011026|0|\r\n\
~D|BANCO##|F#\\1\\1\\R#\\1\\1\\|\r\n\
~C|F#||Fachadas y particiones|0|011026|0|\r\n\
~D|F#|FFQ010\\1\\1\\FFZ020\\1\\1\\|\r\n\
~C|FFQ010|m2|Hoja de partición interior de ladrillo hueco doble||011026|0|\r\n\
~D|FFQ010|mo001\\1\\0,35\\mt001\\1\\33\\|\r\n\
~T|FFQ010|Hoja de partición interior de 7 cm de espesor de fábrica de ladrillo cerámico hueco doble, para revestir.|\r\n\
~C|FFZ020|m²|Hoja exterior de fachada de ladrillo cara vista|45,80|011026|0|\r\n\
~C|R#||Revestimientos|0|011026|0|\r\n\
~C|mo001|h|Oficial 1ª construcción|21,50|011026|1|\r\n\
~C|mt001|ud|Ladrillo cerámico hueco doble|0,12|011026|3|\r\n";

    #[test]
    fn concepts_texts_and_prices_from_their_pieces() {
        let bank = read_bank(SAMPLE.as_bytes()).unwrap();
        assert_eq!(bank.name, "Banco de prueba 2026");
        assert_eq!(bank.root, "BANCO");
        let f = &bank.concepts["FFQ010"];
        assert_eq!(f.unit, "m2");
        assert!(f.text.starts_with("Hoja de partición interior de 7 cm"));
        // 0.35 h × 21.50 + 33 ud × 0.12.
        assert!(
            (f.price - (0.35 * 21.5 + 33.0 * 0.12)).abs() < 1e-9,
            "{}",
            f.price
        );
        assert!((bank.concepts["FFZ020"].price - 45.8).abs() < 1e-9);
        assert!(bank.concepts["F"].chapter);
        assert_eq!(bank.path_of("FFQ010"), vec!["Fachadas y particiones"]);
        // Searching: without accents, and only items.
        let found = bank.search("particion ladrillo", None, 10);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "FFQ010");
        assert!(bank.search("oficial", None, 10).is_empty());
        // The unit asked for first: m² and m2 are the same.
        let all = bank.search("", Some("m2"), 10);
        assert_eq!(all.len(), 2);
        assert_eq!(unit_key("m²"), unit_key("M2"));
    }

    #[test]
    fn windows_1252_and_dos_files() {
        let ansi: Vec<u8> = "~V|x|FIEBDC-3/2016|x|1|ANSI|\r\n~C|A1|m2|Partici"
            .bytes()
            .chain([0xF3, b'n'])
            .chain("|10|010126|0|\r\n".bytes())
            .collect();
        assert_eq!(
            read_bank(&ansi).unwrap().concepts["A1"].summary,
            "Partición"
        );
        let dos: Vec<u8> = "~V|x|FIEBDC-3/2016|x|1|850|\r\n~C|A1|m2|Partici"
            .bytes()
            .chain([0xA2, b'n'])
            .chain("|10|010126|0|\r\n".bytes())
            .collect();
        assert_eq!(read_bank(&dos).unwrap().concepts["A1"].summary, "Partición");
        assert!(read_bank(b"hola").is_err());
    }
}
