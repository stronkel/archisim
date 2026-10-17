//! Reader for ISO 10303-21 ("STEP physical file") data, as used by IFC.
//!
//! It reads the header's schema and every entity instance of the DATA
//! section into a table by id. Values keep their STEP kind (reference,
//! number, string, enumeration, list, typed value); strings are decoded
//! from their `\X2\`, `\X4\`, `\X\` and `\S\` escapes.

use std::collections::HashMap;
use std::fmt;

/// One attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `#123`
    Ref(u64),
    Int(i64),
    Real(f64),
    Str(String),
    /// `.ENUM.`, also `.T.` / `.F.` / `.U.`
    Enum(String),
    /// `$`
    Null,
    /// `*`
    Derived,
    List(Vec<Value>),
    /// `IFCLABEL('x')`
    Typed(String, Box<Value>),
}

impl Value {
    pub fn as_ref(&self) -> Option<u64> {
        match self {
            Value::Ref(r) => Some(*r),
            _ => None,
        }
    }

    /// A number, also inside a typed value (`IFCLENGTHMEASURE(2.5)`).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Real(v) => Some(*v),
            Value::Int(v) => Some(*v as f64),
            Value::Typed(_, v) => v.as_f64(),
            _ => None,
        }
    }

    /// A string, also inside a typed value (`IFCLABEL('x')`).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Typed(_, v) => v.as_str(),
            _ => None,
        }
    }

    pub fn as_enum(&self) -> Option<&str> {
        match self {
            Value::Enum(s) => Some(s),
            Value::Typed(_, v) => v.as_enum(),
            _ => None,
        }
    }

    /// `.T.` → true, `.F.` → false.
    pub fn as_bool(&self) -> Option<bool> {
        match self.as_enum()? {
            "T" => Some(true),
            "F" => Some(false),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Value] {
        match self {
            Value::List(v) => v,
            _ => &[],
        }
    }

    /// References in a list (ignores anything else).
    pub fn refs(&self) -> Vec<u64> {
        self.as_list().iter().filter_map(Value::as_ref).collect()
    }

    /// Numbers in a list.
    pub fn numbers(&self) -> Vec<f64> {
        self.as_list().iter().filter_map(Value::as_f64).collect()
    }
}

/// One entity instance: its type in upper case and its attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub kind: String,
    pub args: Vec<Value>,
}

impl Entity {
    /// Attribute `i`, or `Null` if the entity has fewer.
    pub fn arg(&self, i: usize) -> &Value {
        self.args.get(i).unwrap_or(&Value::Null)
    }
}

/// A parsed file.
#[derive(Debug, Clone, Default)]
pub struct StepFile {
    /// Schema names from `FILE_SCHEMA`, upper case (`IFC4`, `IFC2X3`…).
    pub schemas: Vec<String>,
    /// Header records (`FILE_DESCRIPTION`, `FILE_NAME`, …) in file order.
    pub header: Vec<Entity>,
    pub entities: HashMap<u64, Entity>,
}

impl StepFile {
    /// A header record by name (`FILE_NAME`).
    pub fn header(&self, name: &str) -> Option<&Entity> {
        self.header.iter().find(|e| e.kind == name)
    }

    pub fn get(&self, id: u64) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// The entity a value refers to.
    pub fn deref(&self, v: &Value) -> Option<&Entity> {
        v.as_ref().and_then(|r| self.get(r))
    }

    /// Ids of every entity of a kind (upper case), in id order.
    pub fn of_kind(&self, kind: &str) -> Vec<u64> {
        let mut v: Vec<u64> = self
            .entities
            .iter()
            .filter(|(_, e)| e.kind == kind)
            .map(|(id, _)| *id)
            .collect();
        v.sort_unstable();
        v
    }
}

/// Why a file could not be read, with the line where it happened.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

struct Cursor<'a> {
    s: &'a [u8],
    i: usize,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line,
            message: message.into(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    /// Skips white space and `/* comments */`.
    fn skip(&mut self) {
        while let Some(c) = self.peek() {
            if c == b'\n' {
                self.line += 1;
                self.i += 1;
            } else if c.is_ascii_whitespace() {
                self.i += 1;
            } else if c == b'/' && self.s.get(self.i + 1) == Some(&b'*') {
                self.i += 2;
                while self.i < self.s.len()
                    && !(self.s[self.i] == b'*' && self.s.get(self.i + 1) == Some(&b'/'))
                {
                    if self.s[self.i] == b'\n' {
                        self.line += 1;
                    }
                    self.i += 1;
                }
                self.i += 2;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), ParseError> {
        self.skip();
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            self.err(format!(
                "expected '{}', found {}",
                c as char,
                self.peek()
                    .map_or("end of file".to_string(), |b| format!("'{}'", b as char))
            ))
        }
    }

    fn keyword(&mut self) -> String {
        self.skip();
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
                self.i += 1;
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_ascii_uppercase()
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        self.skip();
        match self.peek() {
            Some(b'#') => {
                self.i += 1;
                let start = self.i;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                match text.parse() {
                    Ok(n) => Ok(Value::Ref(n)),
                    Err(_) => self.err("bad reference"),
                }
            }
            Some(b'$') => {
                self.i += 1;
                Ok(Value::Null)
            }
            Some(b'*') => {
                self.i += 1;
                Ok(Value::Derived)
            }
            Some(b'\'') => self.string().map(Value::Str),
            Some(b'"') => {
                // Binary: kept as text.
                self.i += 1;
                let start = self.i;
                while self.peek().is_some_and(|c| c != b'"') {
                    self.i += 1;
                }
                let text = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
                self.i += 1;
                Ok(Value::Str(text))
            }
            Some(b'.') => {
                self.i += 1;
                let start = self.i;
                while self.peek().is_some_and(|c| c != b'.') {
                    self.i += 1;
                }
                let text = String::from_utf8_lossy(&self.s[start..self.i]).to_ascii_uppercase();
                self.i += 1;
                Ok(Value::Enum(text))
            }
            Some(b'(') => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip();
                if self.peek() == Some(b')') {
                    self.i += 1;
                    return Ok(Value::List(items));
                }
                loop {
                    items.push(self.value()?);
                    self.skip();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b')') => {
                            self.i += 1;
                            return Ok(Value::List(items));
                        }
                        _ => return self.err("expected ',' or ')' in a list"),
                    }
                }
            }
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => {
                let start = self.i;
                self.i += 1;
                while self.peek().is_some_and(|c| {
                    c.is_ascii_digit() || matches!(c, b'.' | b'E' | b'e' | b'-' | b'+')
                }) {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                if text.contains(['.', 'E', 'e']) {
                    // STEP allows "1." and "1.E-5".
                    let fixed = text.replace(".E", ".0E").replace(".e", ".0e");
                    let fixed = if fixed.ends_with('.') {
                        format!("{fixed}0")
                    } else {
                        fixed
                    };
                    match fixed.parse() {
                        Ok(v) => Ok(Value::Real(v)),
                        Err(_) => self.err(format!("bad number {text}")),
                    }
                } else {
                    match text.parse() {
                        Ok(v) => Ok(Value::Int(v)),
                        Err(_) => self.err(format!("bad number {text}")),
                    }
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let name = self.keyword();
                self.expect(b'(')?;
                let inner = self.value_list_until_close()?;
                // A typed value holds one value.
                let v = inner.into_iter().next().unwrap_or(Value::Null);
                Ok(Value::Typed(name, Box::new(v)))
            }
            _ => self.err("unexpected character in a value"),
        }
    }

    /// Values after an opening parenthesis, up to and including ')'.
    fn value_list_until_close(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut items = Vec::new();
        self.skip();
        if self.peek() == Some(b')') {
            self.i += 1;
            return Ok(items);
        }
        loop {
            items.push(self.value()?);
            self.skip();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b')') => {
                    self.i += 1;
                    return Ok(items);
                }
                _ => return self.err("expected ',' or ')'"),
            }
        }
    }

    /// A quoted string with its escapes decoded.
    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1; // opening quote
        let mut raw = Vec::new();
        loop {
            match self.peek() {
                None => return self.err("unterminated string"),
                Some(b'\'') => {
                    if self.s.get(self.i + 1) == Some(&b'\'') {
                        raw.push(b'\'');
                        self.i += 2;
                    } else {
                        self.i += 1;
                        break;
                    }
                }
                Some(c) => {
                    if c == b'\n' {
                        self.line += 1;
                    }
                    raw.push(c);
                    self.i += 1;
                }
            }
        }
        Ok(decode(&raw))
    }
}

/// Decodes STEP string escapes. Plain bytes are read as UTF-8 (many
/// exporters write it directly), falling back to Latin-1.
pub fn decode(raw: &[u8]) -> String {
    let mut out = String::new();
    let mut plain: Vec<u8> = Vec::new();
    let flush = |plain: &mut Vec<u8>, out: &mut String| {
        if !plain.is_empty() {
            match std::str::from_utf8(plain) {
                Ok(s) => out.push_str(s),
                Err(_) => out.extend(plain.iter().map(|&b| b as char)),
            }
            plain.clear();
        }
    };
    let hex =
        |s: &[u8]| -> Option<u32> { u32::from_str_radix(std::str::from_utf8(s).ok()?, 16).ok() };
    let mut i = 0;
    while i < raw.len() {
        let rest = &raw[i..];
        if rest.starts_with(b"\\X2\\") || rest.starts_with(b"\\X4\\") {
            flush(&mut plain, &mut out);
            let width = if rest[2] == b'2' { 4 } else { 8 };
            i += 4;
            let mut units: Vec<u16> = Vec::new();
            while i + width <= raw.len() && !raw[i..].starts_with(b"\\X0\\") {
                match hex(&raw[i..i + width]) {
                    Some(v) if width == 4 => units.push(v as u16),
                    Some(v) => {
                        if let Some(c) = char::from_u32(v) {
                            out.push_str(&String::from_utf16_lossy(&units));
                            units.clear();
                            out.push(c);
                        }
                    }
                    None => break,
                }
                i += width;
            }
            out.push_str(&String::from_utf16_lossy(&units));
            if raw[i..].starts_with(b"\\X0\\") {
                i += 4;
            }
        } else if rest.starts_with(b"\\X\\") && rest.len() >= 5 {
            flush(&mut plain, &mut out);
            if let Some(v) = hex(&rest[3..5]) {
                out.push(v as u8 as char);
            }
            i += 5;
        } else if rest.starts_with(b"\\S\\") && rest.len() >= 4 {
            flush(&mut plain, &mut out);
            out.push((rest[3] as u32 + 128) as u8 as char);
            i += 4;
        } else if rest.starts_with(b"\\\\") {
            plain.push(b'\\');
            i += 2;
        } else if rest.starts_with(b"\\P") && rest.len() >= 4 && rest[3] == b'\\' {
            // Code page switch: ignored.
            i += 4;
        } else {
            plain.push(raw[i]);
            i += 1;
        }
    }
    flush(&mut plain, &mut out);
    out
}

/// Parses a whole STEP file.
pub fn parse(text: &str) -> Result<StepFile, ParseError> {
    let mut c = Cursor {
        s: text.as_bytes(),
        i: 0,
        line: 1,
    };
    let mut file = StepFile::default();
    c.skip();
    if c.keyword() != "ISO-10303-21" {
        return c.err("not a STEP file (it must start with ISO-10303-21;)");
    }
    c.expect(b';')?;
    // HEADER: read records until ENDSEC, keeping FILE_SCHEMA.
    let mut in_data = false;
    loop {
        c.skip();
        if c.peek().is_none() {
            break;
        }
        if c.peek() == Some(b'#') {
            if !in_data {
                return c.err("entity outside the DATA section");
            }
            c.i += 1;
            let start = c.i;
            while c.peek().is_some_and(|b| b.is_ascii_digit()) {
                c.i += 1;
            }
            let id: u64 = std::str::from_utf8(&c.s[start..c.i])
                .ok()
                .and_then(|t| t.parse().ok())
                .map_or_else(|| c.err("bad entity id"), Ok)?;
            c.expect(b'=')?;
            c.skip();
            let kind = if c.peek() == Some(b'(') {
                // Complex entity instance: keep the first part's name.
                c.i += 1;
                let k = c.keyword();
                let mut depth = 1;
                while depth > 0 {
                    match c.peek() {
                        None => return c.err("unterminated complex entity"),
                        Some(b'(') => depth += 1,
                        Some(b')') => depth -= 1,
                        Some(b'\'') => {
                            c.string()?;
                            continue;
                        }
                        Some(b'\n') => c.line += 1,
                        _ => {}
                    }
                    c.i += 1;
                }
                c.expect(b';')?;
                file.entities.insert(
                    id,
                    Entity {
                        kind: k,
                        args: Vec::new(),
                    },
                );
                continue;
            } else {
                c.keyword()
            };
            c.expect(b'(')?;
            let args = c.value_list_until_close()?;
            c.expect(b';')?;
            file.entities.insert(id, Entity { kind, args });
            continue;
        }
        let word = c.keyword();
        match word.as_str() {
            "HEADER" | "ENDSEC" => {
                c.expect(b';')?;
                if word == "ENDSEC" {
                    in_data = false;
                }
            }
            "DATA" => {
                c.skip();
                if c.peek() == Some(b'(') {
                    c.i += 1;
                    c.value_list_until_close()?;
                }
                c.expect(b';')?;
                in_data = true;
            }
            "END-ISO-10303-21" => break,
            "" => return c.err("unexpected character"),
            record => {
                c.expect(b'(')?;
                let args = c.value_list_until_close()?;
                c.expect(b';')?;
                if record == "FILE_SCHEMA" {
                    file.schemas = args
                        .first()
                        .map(|v| {
                            v.as_list()
                                .iter()
                                .filter_map(Value::as_str)
                                .map(|s| s.to_ascii_uppercase())
                                .collect()
                        })
                        .unwrap_or_default();
                }
                file.header.push(Entity {
                    kind: record.to_string(),
                    args,
                });
            }
        }
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('a.ifc','2026-10-17T00:00:00',(''),(''),'x','y','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
/* a comment */
#1=IFCCARTESIANPOINT((0.,1.5,-2.E-3));
#2=IFCWALL('2N1qQ$abcdefghijklmnop',#3,'Fa\\X2\\00E7\\X0\\ada ''norte''',$,*,.T.,(#1,#2),IFCLABEL('x'),12);
#3=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$);
#4=(IFCLENGTHMEASURE(1.)IFCMEASUREWITHUNIT(#1));
ENDSEC;
END-ISO-10303-21;
";

    #[test]
    fn parses_entities_values_and_strings() {
        let f = parse(SAMPLE).unwrap();
        assert_eq!(f.schemas, vec!["IFC4"]);
        assert_eq!(f.header("FILE_NAME").unwrap().arg(4).as_str(), Some("x"));
        assert_eq!(f.entities.len(), 4);
        let p = f.get(1).unwrap();
        assert_eq!(p.kind, "IFCCARTESIANPOINT");
        assert_eq!(p.arg(0).numbers(), vec![0.0, 1.5, -0.002]);
        let w = f.get(2).unwrap();
        assert_eq!(w.arg(2).as_str(), Some("Façada 'norte'"));
        assert_eq!(w.arg(3), &Value::Null);
        assert_eq!(w.arg(4), &Value::Derived);
        assert_eq!(w.arg(5).as_bool(), Some(true));
        assert_eq!(w.arg(6).refs(), vec![1, 2]);
        assert_eq!(w.arg(7).as_str(), Some("x"));
        assert_eq!(w.arg(8).as_f64(), Some(12.0));
        assert_eq!(f.deref(w.arg(1)).unwrap().kind, "IFCPROPERTYSINGLEVALUE");
        assert_eq!(f.get(3).unwrap().arg(2).as_bool(), Some(false));
        assert_eq!(f.of_kind("IFCWALL"), vec![2]);
        assert_eq!(f.get(4).unwrap().kind, "IFCLENGTHMEASURE");
    }

    #[test]
    fn decodes_escapes() {
        assert_eq!(decode(b"Cubierta \\X2\\00F100E1\\X0\\"), "Cubierta ñá");
        assert_eq!(decode(b"\\X4\\0001F600\\X0\\"), "😀");
        assert_eq!(decode(b"Ca\\X\\F1o"), "Caño");
        assert_eq!(decode(b"a\\S\\qb"), "añb");
        assert_eq!(decode("Muro tabiquería".as_bytes()), "Muro tabiquería");
        assert_eq!(decode(b"C:\\\\dir"), "C:\\dir");
    }

    #[test]
    fn reports_where_it_fails() {
        let bad = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=IFCWALL('a',;\nENDSEC;\n";
        let err = parse(bad).unwrap_err();
        assert_eq!(err.line, 5);
        assert!(parse("hello").is_err());
    }

    #[test]
    fn reads_what_the_exporter_writes() {
        let mut m = archisim_core::Model::new("Casa «Ñ»");
        m.add_level("Planta baja", 0.0);
        let text = crate::export_ifc(&m, &crate::ExportOptions::default());
        let f = parse(&text).unwrap();
        assert_eq!(f.schemas, vec!["IFC4"]);
        let project = f.get(f.of_kind("IFCPROJECT")[0]).unwrap();
        assert_eq!(project.arg(2).as_str(), Some("Casa «Ñ»"));
        assert_eq!(f.of_kind("IFCBUILDINGSTOREY").len(), 1);
    }
}
