//! Small arithmetic grammar; no evaluation, functions or unbounded syntax.
use super::Token;
use crate::{Result, check, input};
struct Parser<'a> {
    source: &'a [u8],
    at: usize,
    depth: usize,
    surface: bool,
}
impl Parser<'_> {
    fn whitespace(&mut self) {
        while self.at < self.source.len() && self.source[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }
    fn take(&mut self, c: u8) -> bool {
        self.whitespace();
        if self.source.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn bound(&self, tokens: Vec<Token>) -> Result<Vec<Token>> {
        check(tokens.len() <= 64, "Expanded formula exceeds 64 tokens")?;
        Ok(tokens)
    }
    fn sum(&mut self) -> Result<Vec<Token>> {
        let mut out = self.product()?;
        loop {
            let op = if self.take(b'+') {
                Token::Add
            } else if self.take(b'-') {
                Token::Subtract
            } else {
                break;
            };
            out.extend(self.product()?);
            out.push(op);
            out = self.bound(out)?;
        }
        Ok(out)
    }
    fn product(&mut self) -> Result<Vec<Token>> {
        let mut out = self.unary()?;
        loop {
            let op = if self.take(b'*') {
                Token::Multiply
            } else if self.take(b'/') {
                Token::Divide
            } else {
                break;
            };
            out.extend(self.unary()?);
            out.push(op);
            out = self.bound(out)?;
        }
        Ok(out)
    }
    fn unary(&mut self) -> Result<Vec<Token>> {
        self.depth += 1;
        check(self.depth <= 24, "Formula nesting exceeds 24")?;
        let result = if self.take(b'-') {
            let mut out = self.unary()?;
            out.push(Token::Negate);
            self.bound(out)
        } else if self.take(b'+') {
            self.unary()
        } else {
            self.power()
        };
        self.depth -= 1;
        result
    }
    fn power(&mut self) -> Result<Vec<Token>> {
        let base = self.primary()?;
        if !self.take(b'^') {
            return Ok(base);
        }
        let negative = self.take(b'-');
        if !negative {
            self.take(b'+');
        }
        let exponent = self.number()?;
        check(
            exponent.fract() == 0. && (1. ..=12.).contains(&exponent),
            "Formula powers require a literal integer of magnitude 1..12",
        )?;
        let mut out = base.clone();
        for _ in 1..exponent as usize {
            out.extend(base.clone());
            out.push(Token::Multiply);
            out = self.bound(out)?;
        }
        if negative {
            let mut reciprocal = vec![Token::Constant(1.)];
            reciprocal.extend(out);
            reciprocal.push(Token::Divide);
            out = self.bound(reciprocal)?;
        }
        Ok(out)
    }
    fn primary(&mut self) -> Result<Vec<Token>> {
        if self.take(b'(') {
            let out = self.sum()?;
            check(self.take(b')'), "Formula requires a closing parenthesis")?;
            return Ok(out);
        }
        self.whitespace();
        if self
            .source
            .get(self.at)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            let start = self.at;
            while self
                .source
                .get(self.at)
                .is_some_and(u8::is_ascii_alphanumeric)
            {
                self.at += 1;
            }
            let variable = std::str::from_utf8(&self.source[start..self.at]).unwrap();
            check(
                if self.surface {
                    ["u", "v"].contains(&variable)
                } else {
                    variable == "t"
                },
                "Choose t for curves or u/v for surfaces; functions are unsupported",
            )?;
            return Ok(vec![match variable {
                "t" => Token::T,
                "u" => Token::U,
                _ => Token::V,
            }]);
        }
        Ok(vec![Token::Constant(self.number()?)])
    }
    fn number(&mut self) -> Result<f64> {
        self.whitespace();
        let start = self.at;
        while self.source.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if self.source.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.source.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self
            .source
            .get(self.at)
            .is_some_and(|c| *c == b'e' || *c == b'E')
        {
            self.at += 1;
            if self
                .source
                .get(self.at)
                .is_some_and(|c| *c == b'+' || *c == b'-')
            {
                self.at += 1;
            }
            while self.source.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        let text = std::str::from_utf8(&self.source[start..self.at]).unwrap();
        let value = text
            .parse::<f64>()
            .map_err(|_| input(format!("Expected a finite formula number at byte {start}")))?;
        check(value.is_finite(), "Formula numbers must be finite")?;
        Ok(value)
    }
}
pub(super) fn parse(source: &str, surface: bool) -> Result<Vec<Token>> {
    check(
        !source.is_empty() && source.len() <= 1024 && source.is_ascii(),
        "Formula text requires 1..1024 ASCII bytes",
    )?;
    let mut parser = Parser {
        source: source.as_bytes(),
        at: 0,
        depth: 0,
        surface,
    };
    let out = parser.sum()?;
    parser.whitespace();
    check(
        parser.at == parser.source.len(),
        "Unexpected formula text; use explicit *, /, +, -, ^ and parentheses",
    )?;
    parser.bound(out)
}
