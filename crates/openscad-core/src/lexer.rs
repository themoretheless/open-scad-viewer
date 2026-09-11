//! Tokenizer: a faithful port of `tokenize` in
//! `src/services/openscadCompiler.ts`, operating on UTF-16 code units so
//! positions match the TypeScript parser byte-for-byte.
use crate::ParseError;

/// Token types; names match the TS `TT` enum exactly (diagnostic messages
/// reference them via `TT[type]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TT {
    Num,
    Str,
    Ident,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Hash,
    Dollar,
    Dot,
    Colon,
    Question,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    NotEq,
    Not,
    And,
    Or,
    Eof,
    DirectivePath,
}

impl TT {
    pub fn name(self) -> &'static str {
        match self {
            Self::Num => "Num",
            Self::Str => "Str",
            Self::Ident => "Ident",
            Self::LParen => "LParen",
            Self::RParen => "RParen",
            Self::LBrace => "LBrace",
            Self::RBrace => "RBrace",
            Self::LBracket => "LBracket",
            Self::RBracket => "RBracket",
            Self::Comma => "Comma",
            Self::Semi => "Semi",
            Self::Eq => "Eq",
            Self::Plus => "Plus",
            Self::Minus => "Minus",
            Self::Star => "Star",
            Self::Slash => "Slash",
            Self::Percent => "Percent",
            Self::Caret => "Caret",
            Self::Hash => "Hash",
            Self::Dollar => "Dollar",
            Self::Dot => "Dot",
            Self::Colon => "Colon",
            Self::Question => "Question",
            Self::Lt => "Lt",
            Self::Gt => "Gt",
            Self::LtEq => "LtEq",
            Self::GtEq => "GtEq",
            Self::EqEq => "EqEq",
            Self::NotEq => "NotEq",
            Self::Not => "Not",
            Self::And => "And",
            Self::Or => "Or",
            Self::Eof => "Eof",
            Self::DirectivePath => "DirectivePath",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub t: TT,
    pub v: String,
    pub p: usize,
    pub end: usize,
    pub closed: bool,
}

impl Token {
    fn new(t: TT, v: String, p: usize, end: usize) -> Self {
        Self {
            t,
            v,
            p,
            end,
            closed: false,
        }
    }
    /// Display form used by TS error messages: `token.v || TT[token.t]`.
    pub fn display(&self) -> &str {
        if self.v.is_empty() {
            self.t.name()
        } else {
            &self.v
        }
    }
}

fn is_digit(u: Option<u16>) -> bool {
    matches!(u, Some(u) if (0x30..=0x39).contains(&u))
}
fn is_ident_start(u: Option<u16>) -> bool {
    match u {
        Some(u) => {
            (0x61..=0x7a).contains(&u) // a-z
            || (0x41..=0x5a).contains(&u) // A-Z
            || u == 0x5f // _
            || u == 0x24
        } // $
        None => false,
    }
}
fn is_ident_part(u: Option<u16>) -> bool {
    is_ident_start(u) || is_digit(u)
}

/// `JSON.stringify(ch)` for a single character, used in the
/// `Unexpected character` diagnostic.
fn json_stringify_char(u: u16) -> String {
    let mut out = String::from("\"");
    match u {
        0x22 => out.push_str("\\\""),
        0x5c => out.push_str("\\\\"),
        0x08 => out.push_str("\\b"),
        0x09 => out.push_str("\\t"),
        0x0a => out.push_str("\\n"),
        0x0c => out.push_str("\\f"),
        0x0d => out.push_str("\\r"),
        u if u < 0x20 => out.push_str(&format!("\\u{u:04x}")),
        u => out.push(char::from_u32(u as u32).unwrap_or('\u{fffd}')),
    }
    out.push('"');
    out
}

fn decode_units(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

pub fn tokenize(source: &[u16]) -> Result<Vec<Token>, ParseError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut pending_directive = false;
    let at = |index: usize| source.get(index).copied();
    while i < source.len() {
        let ch = source[i];
        if ch <= 0x20 {
            i += 1;
            continue;
        }
        if ch == b'/' as u16 && at(i + 1) == Some(b'/' as u16) {
            while i < source.len() && source[i] != b'\n' as u16 {
                i += 1;
            }
            continue;
        }
        if ch == b'/' as u16 && at(i + 1) == Some(b'*' as u16) {
            let start = i;
            i += 2;
            while i + 1 < source.len()
                && !(source[i] == b'*' as u16 && source[i + 1] == b'/' as u16)
            {
                i += 1;
            }
            if i + 1 >= source.len() {
                return Err(ParseError::new(start, "Unterminated block comment"));
            }
            i += 2;
            continue;
        }

        let p = i;
        if pending_directive && ch == b'<' as u16 {
            i += 1;
            let path_start = i;
            while i < source.len() && source[i] != b'>' as u16 {
                i += 1;
            }
            let closed = i < source.len();
            let path = decode_units(&source[path_start..i]);
            if closed {
                i += 1;
            }
            let mut token = Token::new(TT::DirectivePath, path, p, i);
            token.closed = closed;
            out.push(token);
            pending_directive = false;
            continue;
        }
        pending_directive = false;
        if ch == b'"' as u16 {
            i += 1;
            let mut value: Vec<u16> = Vec::new();
            let mut closed = false;
            while i < source.len() {
                if source[i] == b'"' as u16 {
                    i += 1;
                    closed = true;
                    break;
                }
                if source[i] == b'\\' as u16 {
                    i += 1;
                    let Some(escaped) = at(i) else { break };
                    value.push(match escaped {
                        0x6e => b'\n' as u16, // n
                        0x74 => b'\t' as u16, // t
                        other => other,
                    });
                    i += 1;
                } else {
                    value.push(source[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err(ParseError::new(p, "Unterminated string"));
            }
            out.push(Token::new(TT::Str, decode_units(&value), p, i));
            continue;
        }

        if is_digit(Some(ch)) || (ch == b'.' as u16 && is_digit(at(i + 1))) {
            let mut value = String::new();
            while let Some(u) = at(i).filter(|&u| is_digit(Some(u))) {
                value.push(u as u8 as char);
                i += 1;
            }
            if at(i) == Some(b'.' as u16) {
                value.push('.');
                i += 1;
                while let Some(u) = at(i).filter(|&u| is_digit(Some(u))) {
                    value.push(u as u8 as char);
                    i += 1;
                }
            }
            if matches!(at(i), Some(u) if u == b'e' as u16 || u == b'E' as u16) {
                value.push(at(i).unwrap() as u8 as char);
                i += 1;
                if matches!(at(i), Some(u) if u == b'+' as u16 || u == b'-' as u16) {
                    value.push(at(i).unwrap() as u8 as char);
                    i += 1;
                }
                if !is_digit(at(i)) {
                    return Err(ParseError::new(p, "Invalid exponent"));
                }
                while let Some(u) = at(i).filter(|&u| is_digit(Some(u))) {
                    value.push(u as u8 as char);
                    i += 1;
                }
            }
            out.push(Token::new(TT::Num, value, p, i));
            continue;
        }

        if is_ident_start(Some(ch)) {
            let start = i;
            while is_ident_part(at(i)) {
                i += 1;
            }
            let value = decode_units(&source[start..i]);
            pending_directive = value == "include" || value == "use";
            out.push(Token::new(TT::Ident, value, p, i));
            continue;
        }

        let two: Option<[u16; 2]> = match (at(i), at(i + 1)) {
            (Some(a), Some(b)) => Some([a, b]),
            _ => None,
        };
        let doubles: &[([u16; 2], TT)] = &[
            ([b'<' as u16, b'=' as u16], TT::LtEq),
            ([b'>' as u16, b'=' as u16], TT::GtEq),
            ([b'=' as u16, b'=' as u16], TT::EqEq),
            ([b'!' as u16, b'=' as u16], TT::NotEq),
            ([b'&' as u16, b'&' as u16], TT::And),
            ([b'|' as u16, b'|' as u16], TT::Or),
        ];
        if let Some(pair) = two
            && let Some((_, t)) = doubles.iter().find(|(k, _)| *k == pair)
        {
            out.push(Token::new(*t, decode_units(&pair), p, i + 2));
            i += 2;
            continue;
        }

        let single = match ch {
            u if u == b'(' as u16 => Some(TT::LParen),
            u if u == b')' as u16 => Some(TT::RParen),
            u if u == b'{' as u16 => Some(TT::LBrace),
            u if u == b'}' as u16 => Some(TT::RBrace),
            u if u == b'[' as u16 => Some(TT::LBracket),
            u if u == b']' as u16 => Some(TT::RBracket),
            u if u == b',' as u16 => Some(TT::Comma),
            u if u == b';' as u16 => Some(TT::Semi),
            u if u == b'=' as u16 => Some(TT::Eq),
            u if u == b'+' as u16 => Some(TT::Plus),
            u if u == b'-' as u16 => Some(TT::Minus),
            u if u == b'*' as u16 => Some(TT::Star),
            u if u == b'/' as u16 => Some(TT::Slash),
            u if u == b'%' as u16 => Some(TT::Percent),
            u if u == b'^' as u16 => Some(TT::Caret),
            u if u == b'#' as u16 => Some(TT::Hash),
            u if u == b'$' as u16 => Some(TT::Dollar),
            u if u == b'.' as u16 => Some(TT::Dot),
            u if u == b':' as u16 => Some(TT::Colon),
            u if u == b'?' as u16 => Some(TT::Question),
            u if u == b'<' as u16 => Some(TT::Lt),
            u if u == b'>' as u16 => Some(TT::Gt),
            u if u == b'!' as u16 => Some(TT::Not),
            _ => None,
        };
        let Some(token_type) = single else {
            return Err(ParseError::new(
                p,
                format!("Unexpected character {}", json_stringify_char(ch)),
            ));
        };
        out.push(Token::new(
            token_type,
            decode_units(&source[i..i + 1]),
            p,
            i + 1,
        ));
        i += 1;
    }
    out.push(Token::new(
        TT::Eof,
        String::new(),
        source.len(),
        source.len(),
    ));
    Ok(out)
}
