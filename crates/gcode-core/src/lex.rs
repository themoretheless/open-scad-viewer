//! RS-274 / Marlin-style line lexer. Does not interpret modal state.

use super::{invalid, Result};
use crate::block::{Block, Program, Word};
use crate::MAX_LINE_BYTES;

pub fn lex_program(text: &str, max_bytes: usize, max_blocks: usize) -> Result<Program> {
    if text.len() > max_bytes {
        return Err(invalid(
            "GCODE_INPUT_LIMIT",
            "G-code input exceeds the configured byte budget",
        ));
    }
    let mut blocks = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        if raw.len() > MAX_LINE_BYTES {
            return Err(invalid(
                "GCODE_LINE_LIMIT",
                "G-code line exceeds 4096 bytes",
            ));
        }
        if let Some(block) = lex_line(raw, index + 1)? {
            blocks.push(block);
            if blocks.len() > max_blocks {
                return Err(invalid(
                    "GCODE_BLOCK_LIMIT",
                    "G-code exceeded the block budget",
                ));
            }
        }
    }
    Ok(Program { blocks })
}

pub fn lex_line(raw: &str, line: usize) -> Result<Option<Block>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let (body, checksum) = split_checksum(trimmed)?;
    if let Some(expected) = checksum {
        let got = body.bytes().fold(0_u8, |acc, byte| acc ^ byte);
        if got != expected {
            return Err(invalid(
                "GCODE_CHECKSUM",
                "G-code checksum does not match the line body",
            ));
        }
    }
    let (code, comment) = split_comments(body);
    let code = code.trim();
    if code.is_empty() {
        return Ok(Some(Block {
            line,
            n: None,
            words: Vec::new(),
            comment,
        }));
    }
    let words = lex_words(code)?;
    let n = words
        .iter()
        .find(|word| word.letter == 'N')
        .and_then(|word| {
            if word.value.is_finite() && word.value >= 0.0 && word.value == word.value.trunc() {
                Some(word.value as u32)
            } else {
                None
            }
        });
    Ok(Some(Block {
        line,
        n,
        words,
        comment,
    }))
}

fn split_checksum(line: &str) -> Result<(&str, Option<u8>)> {
    let Some(star) = line.rfind('*') else {
        return Ok((line, None));
    };
    let body = &line[..star];
    let after = line[star + 1..].trim_start();
    let digits_end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    if digits_end == 0 {
        return Err(invalid(
            "GCODE_CHECKSUM",
            "G-code checksum marker is missing a value",
        ));
    }
    let value = after[..digits_end].parse::<u8>().map_err(|_| {
        invalid(
            "GCODE_CHECKSUM",
            "G-code checksum is not an 8-bit integer",
        )
    })?;
    Ok((body, Some(value)))
}

fn split_comments(line: &str) -> (String, Option<String>) {
    let mut code = String::new();
    let mut comments = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ';' {
            comments.push(chars.collect::<String>());
            break;
        }
        if c == '(' {
            let mut inner = String::new();
            for next in chars.by_ref() {
                if next == ')' {
                    break;
                }
                inner.push(next);
            }
            comments.push(inner);
            continue;
        }
        code.push(c);
    }
    let comment = if comments.is_empty() {
        None
    } else {
        Some(
            comments
                .iter()
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        )
        .filter(|text| !text.is_empty())
    };
    (code, comment)
}

fn lex_words(code: &str) -> Result<Vec<Word>> {
    let bytes = code.as_bytes();
    let mut i = 0;
    let mut words = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            return Err(invalid(
                "GCODE_SYNTAX",
                "G-code word must start with a letter",
            ));
        }
        let letter = (c as char).to_ascii_uppercase();
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut saw_digit = false;
        let mut saw_dot = false;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_digit() {
                saw_digit = true;
                i += 1;
                continue;
            }
            if b == b'.' && !saw_dot {
                saw_dot = true;
                i += 1;
                continue;
            }
            if (b == b'e' || b == b'E') && saw_digit {
                let mut j = i + 1;
                if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                    j += 1;
                }
                let exp_start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > exp_start {
                    i = j;
                    continue;
                }
            }
            break;
        }
        if !saw_digit {
            return Err(invalid(
                "GCODE_SYNTAX",
                "G-code word is missing a numeric value",
            ));
        }
        let number = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
        let value = number.parse::<f64>().map_err(|_| {
            invalid("GCODE_SYNTAX", "G-code word is not a finite number")
        })?;
        if !value.is_finite() {
            return Err(invalid(
                "GCODE_SYNTAX",
                "G-code word is not a finite number",
            ));
        }
        words.push(Word { letter, value });
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_motion_and_comment() {
        let block = lex_line("G1 X10 Y2.5 E1.0 ; draw", 1).unwrap().unwrap();
        assert_eq!(block.words[0], Word { letter: 'G', value: 1.0 });
        assert_eq!(block.comment.as_deref(), Some("draw"));
    }

    #[test]
    fn verifies_checksum() {
        let body = "N1 G1 X1.0";
        let cs = body.bytes().fold(0_u8, |a, b| a ^ b);
        let line = format!("{body}*{cs}");
        let block = lex_line(&line, 1).unwrap().unwrap();
        assert_eq!(block.n, Some(1));
        lex_line(&format!("{body}*0"), 1).unwrap_err();
    }
}
