//! A small XML reader, enough for GML and OSM: start and end tags with
//! their attributes, and the text between them. No DTDs, no namespaces
//! resolved (a tag keeps its prefix), entities `&amp;`, `&lt;`, `&gt;`,
//! `&quot;` and `&apos;` decoded.

/// What the reader finds.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// A start tag (and whether it closes itself, `<a/>`).
    Start {
        name: &'a str,
        attrs: Vec<(&'a str, String)>,
        empty: bool,
    },
    End(&'a str),
    Text(String),
}

/// Decodes the five entities and numeric references.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(j) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let ent = &rest[1..j];
        match ent {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let code = ent
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()));
                match code.and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => out.push_str(&rest[..=j]),
                }
            }
        }
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    out
}

/// The events of a document, in order.
pub fn events(text: &str) -> Vec<Event<'_>> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'<' {
            // Comments, declarations and CDATA.
            if text[i..].starts_with("<!--") {
                i = text[i..].find("-->").map_or(b.len(), |j| i + j + 3);
                continue;
            }
            if text[i..].starts_with("<![CDATA[") {
                let end = text[i..].find("]]>").map_or(b.len(), |j| i + j);
                out.push(Event::Text(text[i + 9..end].to_string()));
                i = (end + 3).min(b.len());
                continue;
            }
            if text[i..].starts_with("<?") || text[i..].starts_with("<!") {
                i = text[i..].find('>').map_or(b.len(), |j| i + j + 1);
                continue;
            }
            let end = match text[i..].find('>') {
                Some(j) => i + j,
                None => break,
            };
            let inner = &text[i + 1..end];
            i = end + 1;
            if let Some(name) = inner.strip_prefix('/') {
                out.push(Event::End(name.trim()));
                continue;
            }
            let (inner, empty) = match inner.strip_suffix('/') {
                Some(x) => (x, true),
                None => (inner, false),
            };
            let name_end = inner
                .find(|c: char| c.is_whitespace())
                .unwrap_or(inner.len());
            let name = &inner[..name_end];
            let mut attrs = Vec::new();
            let mut rest = &inner[name_end..];
            loop {
                rest = rest.trim_start();
                let Some(eq) = rest.find('=') else {
                    break;
                };
                let key = rest[..eq].trim();
                let after = rest[eq + 1..].trim_start();
                let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                    break;
                };
                let Some(close) = after[1..].find(q) else {
                    break;
                };
                attrs.push((key, unescape(&after[1..1 + close])));
                rest = &after[close + 2..];
            }
            out.push(Event::Start { name, attrs, empty });
            if empty {
                out.push(Event::End(name));
            }
        } else {
            let end = text[i..].find('<').map_or(b.len(), |j| i + j);
            let t = &text[i..end];
            if !t.trim().is_empty() {
                out.push(Event::Text(unescape(t)));
            }
            i = end;
        }
    }
    out
}

/// The local part of a tag's name: `gml:posList` → `posList`.
pub fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tags_attributes_and_text() {
        let e = events(
            r#"<?xml version="1.0"?><a x="1" y='b&amp;c'><b/>texto &lt;1&gt;<!-- no --></a>"#,
        );
        assert_eq!(
            e[0],
            Event::Start {
                name: "a",
                attrs: vec![("x", "1".into()), ("y", "b&c".into())],
                empty: false
            }
        );
        assert_eq!(e[2], Event::End("b"));
        assert_eq!(e[3], Event::Text("texto <1>".into()));
        assert_eq!(e[4], Event::End("a"));
        assert_eq!(local("gml:posList"), "posList");
    }
}
