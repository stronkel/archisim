//! Splits a recipe into indented lines of tokens.

use crate::error::{syntax, FamilyError};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    Sym(&'static str),
}

impl Tok {
    pub(crate) fn describe(&self) -> String {
        match self {
            Tok::Ident(s) => format!("«{s}»"),
            Tok::Num(n) => format!("«{n}»"),
            Tok::Str(s) => format!("\"{s}\""),
            Tok::Sym(s) => format!("«{s}»"),
        }
    }
}

/// One non-empty line: its number (from 1), indentation in spaces, tokens,
/// and the lines indented under it.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    pub no: usize,
    pub indent: usize,
    pub toks: Vec<Tok>,
    pub children: Vec<Line>,
}

/// Symbols, longest first so that `==` is not read as `=` `=`.
const SYMBOLS: [&str; 18] = [
    "..", "==", "!=", "<=", ">=", "(", ")", ",", "<", ">", "=", "+", "-", "*", "/", "[", "]", ":",
];

/// Tokenises one line (without its indentation). `#` starts a comment.
pub(crate) fn tokenize(text: &str, no: usize) -> Result<Vec<Tok>, FamilyError> {
    let chars: Vec<char> = text.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '#' {
            break;
        } else if c == '"' {
            let mut s = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err(syntax(no, "text without closing quotes")),
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some('\\') => {
                        match chars.get(i + 1) {
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            Some('n') => s.push('\n'),
                            _ => return Err(syntax(no, "unknown escape in text")),
                        }
                        i += 2;
                    }
                    Some(&ch) => {
                        s.push(ch);
                        i += 1;
                    }
                }
            }
            toks.push(Tok::Str(s));
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // A decimal point needs a digit after it: `1..3` is a range.
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text: String = chars[start..i].iter().collect();
            let n: f64 = text
                .parse()
                .map_err(|_| syntax(no, format!("«{text}» is not a number")))?;
            toks.push(Tok::Num(n));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
        } else {
            let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let Some(sym) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) else {
                return Err(syntax(no, format!("unexpected character «{c}»")));
            };
            toks.push(Tok::Sym(sym));
            i += sym.chars().count();
        }
    }
    Ok(toks)
}

/// Reads the whole text into a tree of lines by indentation. Tabs are not
/// allowed in the indentation (their width is ambiguous).
pub(crate) fn lines(source: &str) -> Result<Vec<Line>, FamilyError> {
    let mut flat = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let no = i + 1;
        let body = raw.trim_start_matches(' ');
        if body.starts_with('\t') {
            return Err(syntax(no, "indent with spaces, not tabs"));
        }
        let toks = tokenize(body, no)?;
        if toks.is_empty() {
            continue;
        }
        flat.push(Line {
            no,
            indent: raw.len() - body.len(),
            toks,
            children: Vec::new(),
        });
    }
    block(&mut flat.into_iter().peekable(), 0)
}

/// Collects the lines at exactly `indent`, each with its deeper lines as
/// children.
fn block(
    it: &mut std::iter::Peekable<std::vec::IntoIter<Line>>,
    indent: usize,
) -> Result<Vec<Line>, FamilyError> {
    let mut out: Vec<Line> = Vec::new();
    while let Some(next) = it.peek() {
        if next.indent < indent {
            break;
        }
        if next.indent > indent {
            let Some(parent) = out.last_mut() else {
                return Err(syntax(next.no, "unexpected indentation"));
            };
            let child_indent = next.indent;
            parent.children = block(it, child_indent)?;
            // After the children, the next line must return to this level
            // or to an outer one, not to somewhere in between.
            if let Some(after) = it.peek() {
                if after.indent > indent && after.indent < child_indent {
                    return Err(syntax(after.no, "indentation does not match any block"));
                }
            }
            continue;
        }
        out.push(it.next().expect("peeked"));
    }
    Ok(out)
}
