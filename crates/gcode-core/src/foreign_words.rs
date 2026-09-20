//! Borrowed words for the tolerant preview reader, not a printer protocol validator.
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    G(u32),
    M(u32),
    Other,
}

pub(super) fn command(word: &str) -> Command {
    let bytes = word.as_bytes();
    if bytes.len() < 2 || !bytes[1..].iter().all(u8::is_ascii_digit) {
        return Command::Other;
    }
    let Ok(number) = word[1..].parse() else {
        return Command::Other;
    };
    match bytes[0].to_ascii_uppercase() {
        b'G' => Command::G(number),
        b'M' => Command::M(number),
        _ => Command::Other,
    }
}

/// Checksum suffixes are discarded, not verified. Allocate only to remove
/// parenthesized comments embedded in code; semicolons inside them are comments too.
pub(super) fn code(line: &str) -> Cow<'_, str> {
    let Some(first) = line.find(['(', ')', ';', '*']) else {
        return Cow::Borrowed(line);
    };
    if matches!(line.as_bytes()[first], b';' | b'*') {
        return Cow::Borrowed(&line[..first]);
    }
    let mut clean = String::with_capacity(line.len());
    clean.push_str(&line[..first]);
    let mut depth = 0usize;
    for ch in line[first..].chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ';' | '*' if depth == 0 => break,
            _ if depth == 0 => clean.push(ch),
            _ => {}
        }
    }
    Cow::Owned(clean)
}

struct Words<'a>(&'a str);

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0 = self.0.trim_start();
        if self.0.is_empty() {
            return None;
        }
        let mut previous = ' ';
        let mut end = self.0.len();
        for (index, ch) in self.0.char_indices() {
            // E/e always starts an extruder word after a decimal, never an exponent.
            if ch.is_whitespace()
                || (ch.is_ascii_alphabetic() && (previous.is_ascii_digit() || previous == '.'))
            {
                end = index;
                break;
            }
            previous = ch;
        }
        let word = &self.0[..end];
        self.0 = &self.0[end..];
        Some(word)
    }
}

pub(super) fn words(code: &str) -> impl Iterator<Item = &str> {
    Words(code).filter(|word| {
        let bytes = word.as_bytes();
        !(bytes.len() > 1
            && bytes[0].eq_ignore_ascii_case(&b'N')
            && bytes[1..].iter().all(u8::is_ascii_digit))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_lines_and_suffixes_borrow_the_input() {
        for text in ["G1X10E2", "G1 X10 ; comment", "N1G1X10E2*12"] {
            assert!(matches!(code(text), Cow::Borrowed(_)));
        }
        assert_eq!(code("G1 (outer (nested)*12;ignored) X2; end"), "G1  X2");
        assert_eq!(code("G1 X2 (unfinished ; *0"), "G1 X2 ");
    }

    #[test]
    fn compact_words_keep_extrusion_distinct_and_normalize_command_only() {
        let parsed: Vec<_> = words("n123g01x10y1e2F600").collect();
        assert_eq!(parsed, ["g01", "x10", "y1", "e2", "F600"]);
        assert_eq!(command(parsed[0]), Command::G(1));
        assert_eq!(command("m083"), Command::M(83));
        for unknown in ["G", "G1.2", "G4294967296", "SET_PRESSURE_ADVANCE", "T1"] {
            assert_eq!(command(unknown), Command::Other);
        }
        assert_eq!(words("G1 Xabc").collect::<Vec<_>>(), ["G1", "Xabc"]);
    }
}
