use super::super::*;

impl<'a> Parser<'a> {
    pub(super) fn is_list_comprehension_start(&self, index: usize, recursion: usize) -> bool {
        if recursion > MAX_EXPRESSION_DEPTH {
            return false;
        }
        let Some(token) = self.tokens.get(index) else {
            return false;
        };
        if token.t != TT::Ident {
            return false;
        }
        if token.v == "each" {
            return true;
        }
        if (token.v == "for" || token.v == "if")
            && self.tokens.get(index + 1).map(|t| t.t) == Some(TT::LParen)
        {
            return true;
        }
        if token.v != "let" || self.tokens.get(index + 1).map(|t| t.t) != Some(TT::LParen) {
            return false;
        }
        let Some(close) = self.matching_right_paren(index + 1) else {
            return false;
        };
        self.is_list_comprehension_parenthesized_start(close + 1, recursion + 1)
    }

    pub(super) fn is_list_comprehension_parenthesized_start(&self, index: usize, recursion: usize) -> bool {
        if self.is_list_comprehension_start(index, recursion) {
            return true;
        }
        self.tokens.get(index).map(|t| t.t) == Some(TT::LParen)
            && self.is_list_comprehension_parenthesized_start(index + 1, recursion + 1)
    }

    pub(super) fn matching_right_paren(&self, open_index: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (index, token) in self.tokens.iter().enumerate().skip(open_index) {
            match token.t {
                TT::LParen => depth += 1,
                TT::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn list_comprehension(&mut self) -> PResult<Expr> {
        let keyword = self.expect(TT::Ident, None)?;
        let (keyword_v, keyword_p) = (keyword.v.clone(), keyword.p);
        if keyword_v == "each" {
            let value = self.list_comprehension_or_expression()?;
            return self.expression_node(Expr::LcEach {
                value: Box::new(value),
                p: keyword_p,
            });
        }

        self.expect(TT::LParen, Some(format!("Expected ( after {keyword_v}")))?;
        if keyword_v == "for" {
            if self.has_top_level_semicolon_before_right_paren() {
                let init = self.expression_arguments(
                    "Expected ; after C-style for initializer".to_string(),
                    TT::Semi,
                )?;
                let condition = self.expression()?;
                self.expect(
                    TT::Semi,
                    Some("Expected ; after C-style for condition".to_string()),
                )?;
                let update = self.expression_arguments(
                    "Expected ) after C-style for update".to_string(),
                    TT::RParen,
                )?;
                let body = self.list_comprehension_or_expression()?;
                return self.expression_node(Expr::LcForC {
                    init,
                    condition: Box::new(condition),
                    update,
                    body: Box::new(body),
                    p: keyword_p,
                });
            }
            let args =
                self.expression_arguments("Expected ) after for bindings".to_string(), TT::RParen)?;
            let body = self.list_comprehension_or_expression()?;
            return self.expression_node(Expr::LcFor {
                args,
                body: Box::new(body),
                p: keyword_p,
            });
        }

        if keyword_v == "if" {
            let condition = self.expression()?;
            self.expect(
                TT::RParen,
                Some("Expected ) after list-comprehension condition".to_string()),
            )?;
            let yes = self.list_comprehension_or_expression()?;
            let mut no: Option<Box<Expr>> = None;
            if self.peek().t == TT::Ident && self.peek().v == "else" {
                self.advance();
                no = Some(Box::new(self.list_comprehension_or_expression()?));
            }
            return self.expression_node(Expr::LcIf {
                condition: Box::new(condition),
                yes: Box::new(yes),
                no,
                p: keyword_p,
            });
        }

        if keyword_v == "let" {
            let args = self.expression_arguments(
                "Expected ) after list-comprehension let bindings".to_string(),
                TT::RParen,
            )?;
            let body = self.parenthesized_list_comprehension()?;
            return self.expression_node(Expr::LcLet {
                args,
                body: Box::new(body),
                p: keyword_p,
            });
        }

        Err(self.fail_at(
            keyword,
            format!("Expected list comprehension, got {keyword_v}"),
            None,
        ))
    }

    pub(super) fn list_comprehension_or_expression(&mut self) -> PResult<Expr> {
        if self.is_list_comprehension_start(self.pos, 0) {
            return self.list_comprehension();
        }
        if self.peek().t == TT::LParen && self.is_list_comprehension_start(self.pos + 1, 0) {
            return self.parenthesized_list_comprehension();
        }
        self.expression()
    }

    pub(super) fn parenthesized_list_comprehension(&mut self) -> PResult<Expr> {
        if !self.matches(TT::LParen) {
            return self.list_comprehension();
        }
        let value = self.list_comprehension()?;
        self.expect(
            TT::RParen,
            Some("Expected ) after list comprehension".to_string()),
        )?;
        Ok(value)
    }

    pub(super) fn has_top_level_semicolon_before_right_paren(&self) -> bool {
        let mut parens = 0i64;
        let mut brackets = 0i64;
        let mut braces = 0i64;
        for token in self.tokens.iter().skip(self.pos) {
            match token.t {
                TT::LParen => parens += 1,
                TT::RParen => {
                    if parens == 0 && brackets == 0 && braces == 0 {
                        return false;
                    }
                    parens -= 1;
                }
                TT::LBracket => brackets += 1,
                TT::RBracket => brackets -= 1,
                TT::LBrace => braces += 1,
                TT::RBrace => braces -= 1,
                TT::Semi if parens == 0 && brackets == 0 && braces == 0 => return true,
                _ => {}
            }
        }
        false
    }
}
