use super::*;

mod comprehension;
mod expressions;
mod statements;

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], profile: LanguageProfile) -> Self {
        Self {
            tokens,
            pos: 0,
            nodes: 0,
            expression_depth: 0,
            statement_depth: 0,
            last_token_end: 0,
            profile,
        }
    }

    pub fn parse_all(&mut self) -> PResult<Vec<Statement>> {
        let mut result = Vec::with_capacity((self.tokens.len() / 8).clamp(4, 256));
        while self.peek().t != TT::Eof {
            result.extend(self.statement()?);
        }
        Ok(result)
    }

    pub(super) fn peek(&self) -> &'a Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }
    pub(super) fn peek_at(&self, offset: usize) -> &'a Token {
        self.tokens
            .get(self.pos + offset)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }
    pub(super) fn advance(&mut self) -> &'a Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
        self.last_token_end = token.end;
        token
    }
    pub(super) fn matches(&mut self, t: TT) -> bool {
        if self.peek().t == t {
            self.advance();
            return true;
        }
        false
    }
    pub(super) fn expect(&mut self, t: TT, message: Option<String>) -> PResult<&'a Token> {
        let token = self.advance();
        if token.t != t {
            let message = message
                .unwrap_or_else(|| format!("Expected {}, got {}", t.name(), token.display()));
            return Err(self.fail_at(token, message, None));
        }
        Ok(token)
    }
    pub(super) fn fail_at(&self, token: &Token, message: String, code: Option<&'static str>) -> ParseError {
        ParseError {
            message,
            code,
            position: token.p,
            end_position: Some(token.end),
        }
    }
    pub(super) fn fail(&self, message: impl Into<String>) -> ParseError {
        let token = self.peek();
        ParseError {
            message: message.into(),
            code: None,
            position: token.p,
            end_position: Some(token.end),
        }
    }
    pub(super) fn count_node(&mut self) -> PResult<()> {
        self.nodes += 1;
        if self.nodes > MAX_AST_NODES {
            // Keep the TS `toLocaleString()` grouping in the pinned message.
            return Err(self.fail("Model exceeds the 25,000 syntax node limit"));
        }
        Ok(())
    }
    pub(super) fn expression_node(&mut self, node: Expr) -> PResult<Expr> {
        self.count_node()?;
        Ok(node)
    }
    pub(super) fn descend_expression<T>(
        &mut self,
        position: usize,
        parse: impl FnOnce(&mut Self) -> PResult<T>,
    ) -> PResult<T> {
        if self.expression_depth >= MAX_EXPRESSION_DEPTH {
            return Err(ParseError::new(
                position,
                format!("Expression exceeds {MAX_EXPRESSION_DEPTH} nested levels"),
            ));
        }
        self.expression_depth += 1;
        let result = parse(self);
        self.expression_depth -= 1;
        result
    }
}
