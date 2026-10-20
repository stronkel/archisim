//! Low-level writer for ISO 10303-21 ("STEP physical file") data.

use std::fmt::Write as _;

/// A reference to an entity instance (`#123`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ref(pub usize);

impl std::fmt::Display for Ref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Collects entity instances and numbers them in order.
#[derive(Debug, Default)]
pub struct StepWriter {
    lines: Vec<String>,
}

impl StepWriter {
    /// Adds an entity such as `IFCDIRECTION((0.,0.,1.))` and returns its ref.
    pub fn add(&mut self, entity: impl AsRef<str>) -> Ref {
        let r = Ref(self.lines.len() + 1);
        self.lines.push(format!("{r}={};", entity.as_ref()));
        r
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Every entity whose first attribute is a `GlobalId` (22 characters
    /// in quotes), by it: where an element of the model was written.
    pub fn by_guid(&self) -> std::collections::HashMap<String, Ref> {
        let mut out = std::collections::HashMap::new();
        for (i, l) in self.lines.iter().enumerate() {
            let Some(open) = l.find("('") else { continue };
            let rest = &l[open + 2..];
            if rest.len() > 23 && rest.as_bytes()[22] == b'\'' {
                out.entry(rest[..22].to_string()).or_insert(Ref(i + 1));
            }
        }
        out
    }

    /// The DATA section body.
    pub fn data(&self) -> String {
        let mut s = String::new();
        for l in &self.lines {
            s.push_str(l);
            s.push('\n');
        }
        s
    }
}

/// A STEP string literal: quotes doubled, backslashes escaped and any
/// non-ASCII character encoded with `\X2\` (or `\X4\` beyond the BMP).
pub fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            c if (c as u32) <= 0xFFFF => {
                let _ = write!(out, "\\X2\\{:04X}\\X0\\", c as u32);
            }
            c => {
                let _ = write!(out, "\\X4\\{:08X}\\X0\\", c as u32);
            }
        }
    }
    out.push('\'');
    out
}

/// A STEP real number. Always contains a decimal point, as the format requires.
pub fn real(value: f64) -> String {
    let v = if value.abs() < 1e-12 { 0.0 } else { value };
    // Round away floating noise below a micrometre.
    let v = (v * 1e9).round() / 1e9;
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.")
    }
}

/// `(#1,#2,#3)`
pub fn list(refs: &[Ref]) -> String {
    let items: Vec<String> = refs.iter().map(|r| r.to_string()).collect();
    format!("({})", items.join(","))
}

pub fn boolean(value: bool) -> &'static str {
    if value {
        ".T."
    } else {
        ".F."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(string("Muro"), "'Muro'");
        assert_eq!(string("O'Neil"), "'O''Neil'");
        assert_eq!(string("a\\b"), "'a\\\\b'");
        assert_eq!(string("Cerámico"), "'Cer\\X2\\00E1\\X0\\mico'");
        assert_eq!(string("ñ"), "'\\X2\\00F1\\X0\\'");
    }

    #[test]
    fn reals_have_a_point() {
        assert_eq!(real(3.0), "3.");
        assert_eq!(real(0.0), "0.");
        assert_eq!(real(-0.0), "0.");
        assert_eq!(real(0.115), "0.115");
        assert_eq!(real(0.1 + 0.2), "0.3");
    }

    #[test]
    fn refs_are_sequential() {
        let mut w = StepWriter::default();
        assert_eq!(w.add("A()"), Ref(1));
        assert_eq!(w.add("B()"), Ref(2));
        assert_eq!(w.data(), "#1=A();\n#2=B();\n");
    }
}
