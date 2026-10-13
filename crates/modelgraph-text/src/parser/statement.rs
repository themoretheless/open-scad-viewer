use super::*;

#[derive(Clone)]
pub(super) struct Token {
    pub(super) text: String,
    pub(super) start: usize,
    pub(super) end: usize,
}
pub struct Statement {
    pub node: J,
    pub line: usize,
}
pub(super) struct Parser<'a> {
    pub(super) source: &'a str,
    pub(super) tokens: Vec<Token>,
    pub(super) p: usize,
    pub(super) depth: usize,
    pub(super) match_indent: Option<usize>,
    pub(super) line_starts: Vec<usize>,
}
pub(super) fn ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|x| x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}
pub(super) fn unique(xs: impl IntoIterator<Item = String>) -> bool {
    let mut set = std::collections::HashSet::new();
    xs.into_iter().all(|x| set.insert(x))
}
pub(super) fn lex(source: &str) -> R<Vec<Token>> {
    if source.encode_utf16().count() > 262144 {
        return Err(crate::error("ModelGraph Text exceeds 256 KiB."));
    }
    let b = source.as_bytes();
    let mut p = 0;
    let mut out = Vec::new();
    while p < b.len() {
        let start = p;
        if b[p] == b' ' || b[p] == b'\t' || b[p] == b'\r' {
            p += 1;
            continue;
        }
        if source[p..].starts_with("//") {
            while p < b.len() && b[p] != b'\n' {
                p += 1
            }
            continue;
        }
        if source[p..].starts_with("/*")
            && let Some(end) = source[p + 2..].find("*/")
        {
            p += end + 4;
            continue;
        }
        if b[p] == b'"' {
            p += 1;
            let mut closed = false;
            while p < b.len() {
                if b[p] == b'\n' {
                    break;
                }
                if b[p] == b'"' {
                    p += 1;
                    closed = true;
                    break;
                }
                if b[p] == b'\\' {
                    p += 1
                }
                if p < b.len() {
                    p += source[p..].chars().next().unwrap().len_utf8()
                }
            }
            if !closed {
                return Err(crate::error(format!("Unexpected character at {start}: \"")));
            }
        } else if b[p].is_ascii_digit()
            || (b[p] == b'.' && b.get(p + 1).is_some_and(u8::is_ascii_digit))
        {
            if b[p] == b'.' {
                p += 1;
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
            } else {
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
                if b.get(p) == Some(&b'.')
                    && !b
                        .get(p + 1)
                        .is_some_and(|c| *c == b'.' || c.is_ascii_alphabetic() || *c == b'_')
                {
                    p += 1;
                    while p < b.len() && b[p].is_ascii_digit() {
                        p += 1
                    }
                }
            }
            if b.get(p).is_some_and(|c| *c == b'e' || *c == b'E') {
                let old = p;
                p += 1;
                if b.get(p).is_some_and(|c| *c == b'+' || *c == b'-') {
                    p += 1
                }
                let digits = p;
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
                if p == digits {
                    p = old
                }
            }
            for unit in ["mm", "cm", "in", "deg", "rad", "m"] {
                if source[p..].starts_with(unit) {
                    p += unit.len();
                    break;
                }
            }
        } else if b[p].is_ascii_alphabetic() || b[p] == b'_' {
            p += 1;
            while p < b.len() && (b[p].is_ascii_alphanumeric() || b[p] == b'_') {
                p += 1
            }
        } else if let Some(op) = [
            "|>", "->", "=>", "..<", "..", "**", "==", "!=", "<=", ">=", "&&", "||",
        ]
        .iter()
        .find(|op| source[p..].starts_with(**op))
        {
            p += op.len()
        } else if b"\n{}()[],.?:;=+*/%<>-!|@".contains(&b[p]) {
            p += 1
        } else {
            return Err(crate::error(format!(
                "Unexpected character at {}: {}",
                source[..p].encode_utf16().count(),
                source[p..].chars().next().unwrap()
            )));
        }
        out.push(Token {
            text: source[start..p].into(),
            start,
            end: p,
        });
    }
    out.push(Token {
        text: "EOF".into(),
        start: p,
        end: p,
    });
    Ok(out)
}
