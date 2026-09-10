//! Recursive-descent parser: a faithful port of the `Parser` class in
//! `src/services/openscadCompiler.ts` for both language profiles, including
//! node-count/depth limits and exact diagnostic messages and positions.
use crate::ast::*;
use crate::lexer::{Token, TT};
use crate::{LanguageProfile, ParseError, MAX_AST_NODES, MAX_EXPRESSION_DEPTH, MAX_STATEMENT_DEPTH};

type PResult<T> = Result<T, ParseError>;

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    nodes: usize,
    expression_depth: usize,
    statement_depth: usize,
    last_token_end: usize,
    profile: LanguageProfile,
}

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
        let mut result = Vec::new();
        while self.peek().t != TT::Eof {
            result.extend(self.statement()?);
        }
        Ok(result)
    }

    fn peek(&self) -> &'a Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }
    fn peek_at(&self, offset: usize) -> &'a Token {
        self.tokens
            .get(self.pos + offset)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }
    fn advance(&mut self) -> &'a Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
        self.last_token_end = token.end;
        token
    }
    fn matches(&mut self, t: TT) -> bool {
        if self.peek().t == t {
            self.advance();
            return true;
        }
        false
    }
    fn expect(&mut self, t: TT, message: Option<String>) -> PResult<&'a Token> {
        let token = self.advance();
        if token.t != t {
            let message = message
                .unwrap_or_else(|| format!("Expected {}, got {}", t.name(), token.display()));
            return Err(self.fail_at(token, message, None));
        }
        Ok(token)
    }
    fn fail_at(&self, token: &Token, message: String, code: Option<&'static str>) -> ParseError {
        ParseError {
            message,
            code,
            position: token.p,
            end_position: Some(token.end),
        }
    }
    fn fail(&self, message: impl Into<String>) -> ParseError {
        let token = self.peek();
        ParseError {
            message: message.into(),
            code: None,
            position: token.p,
            end_position: Some(token.end),
        }
    }
    fn count_node(&mut self) -> PResult<()> {
        self.nodes += 1;
        if self.nodes > MAX_AST_NODES {
            // Keep the TS `toLocaleString()` grouping in the pinned message.
            return Err(self.fail("Model exceeds the 25,000 syntax node limit"));
        }
        Ok(())
    }
    fn expression_node(&mut self, node: Expr) -> PResult<Expr> {
        self.count_node()?;
        Ok(node)
    }
    fn descend_expression<T>(&mut self, position: usize, parse: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
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

    fn statement(&mut self) -> PResult<Vec<Statement>> {
        if self.statement_depth >= MAX_STATEMENT_DEPTH {
            return Err(self.fail(format!("Model exceeds {MAX_STATEMENT_DEPTH} nested statements")));
        }
        self.statement_depth += 1;
        let result = self.parse_statement();
        self.statement_depth -= 1;
        result
    }

    fn parse_statement(&mut self) -> PResult<Vec<Statement>> {
        if self.matches(TT::Semi) {
            return Ok(Vec::new());
        }
        if self.peek().t == TT::LBrace {
            if self.profile.is_subset() {
                return Err(self.fail("Expected a variable, module, or geometry call"));
            }
            self.advance();
            self.count_node()?;
            let mut statements = Vec::new();
            while self.peek().t != TT::RBrace && self.peek().t != TT::Eof {
                statements.extend(self.statement()?);
            }
            self.expect(TT::RBrace, Some("Expected }".to_string()))?;
            return Ok(statements);
        }
        let mut disabled = false;
        let mut viewport_modifiers: Vec<ViewportModifier> = Vec::new();
        while matches!(self.peek().t, TT::Hash | TT::Percent | TT::Star | TT::Not) {
            let modifier = self.advance();
            if self.profile.is_subset() {
                if modifier.t == TT::Star {
                    disabled = true;
                } else {
                    return Err(self.fail_at(
                        modifier,
                        format!(
                            "Viewport modifier {} is not supported by openscad-viewer-subset@1",
                            modifier.v
                        ),
                        Some("E_FEATURE_VIEWPORT_MODIFIER"),
                    ));
                }
                continue;
            }
            let kind = match modifier.t {
                TT::Star => "disable",
                TT::Hash => "highlight",
                TT::Percent => "background",
                _ => "root",
            };
            viewport_modifiers.push(ViewportModifier {
                kind,
                token: modifier.v.chars().next().unwrap_or('\0'),
                start: modifier.p,
                end: modifier.end,
            });
        }
        if self.peek().t != TT::Ident {
            return Err(self.fail("Expected a variable, module, or geometry call"));
        }

        let mut node: Statement;
        if self.peek().v == "module" {
            node = Statement::Module(self.module_definition()?);
        } else if self.peek().v == "function" {
            if self.profile.is_subset() {
                return Err(self.fail_coded(
                    "function is not supported by openscad-viewer-subset@1",
                    "E_FEATURE_USER_FUNCTION",
                ));
            }
            node = Statement::Function(self.function_definition()?);
        } else if self.peek().v == "include" || self.peek().v == "use" {
            if self.profile.is_subset() {
                let feature = if self.peek().v == "include" { "include" } else { "use" };
                let code = if feature == "include" { "E_FEATURE_INCLUDE" } else { "E_FEATURE_USE" };
                return Err(self.fail_coded(
                    format!("{feature} is not supported by openscad-viewer-subset@1"),
                    code,
                ));
            }
            node = Statement::Directive(self.directive()?);
        } else if self.peek_at(1).t == TT::Eq {
            node = Statement::Assign(self.assignment()?);
        } else {
            node = Statement::Call(self.call()?);
        }
        if !viewport_modifiers.is_empty() {
            let first_span = (viewport_modifiers[0].start, viewport_modifiers[0].end);
            match &mut node {
                Statement::Call(call) => call.viewport_modifiers = viewport_modifiers,
                Statement::Directive(directive) => directive.viewport_modifiers = viewport_modifiers,
                _ => {
                    return Err(ParseError::new(
                        first_span.0,
                        "Viewport modifiers may prefix only a module instantiation or an include/use directive",
                    )
                    .with_end(first_span.1));
                }
            }
        }
        self.count_node()?;
        if self.profile.is_subset() && disabled {
            Ok(Vec::new())
        } else {
            Ok(vec![node])
        }
    }

    fn fail_coded(&self, message: impl Into<String>, code: &'static str) -> ParseError {
        let token = self.peek();
        ParseError {
            message: message.into(),
            code: Some(code),
            position: token.p,
            end_position: Some(token.end),
        }
    }

    fn assignment(&mut self) -> PResult<AssignNode> {
        let name = self.expect(TT::Ident, None)?;
        let (name_v, name_p) = (name.v.clone(), name.p);
        self.expect(TT::Eq, None)?;
        let value = self.expression()?;
        let terminator = self.expect(TT::Semi, Some("Expected ; after assignment".to_string()))?;
        Ok(AssignNode {
            name: name_v,
            value,
            p: name_p,
            end: terminator.end,
        })
    }

    fn directive(&mut self) -> PResult<DirectiveNode> {
        let keyword = self.advance();
        let (keyword_v, keyword_p) = (keyword.v.clone(), keyword.p);
        let path = self.expect(
            TT::DirectivePath,
            Some(format!("Expected <path> after {keyword_v}")),
        )?;
        if !path.closed {
            return Err(self.fail_at(path, format!("Unterminated {keyword_v} path"), None));
        }
        if path.v.is_empty() {
            return Err(self.fail_at(path, format!("{keyword_v} path cannot be empty"), None));
        }
        if path.v.contains(['\r', '\n']) {
            return Err(self.fail_at(
                path,
                format!("{keyword_v} path cannot contain a line break"),
                None,
            ));
        }
        let path_span = (path.p + 1, path.end - 1);
        self.matches(TT::Semi);
        Ok(DirectiveNode {
            directive: if keyword_v == "include" { "include" } else { "use" },
            path: path.v.clone(),
            path_span,
            p: keyword_p,
            end: self.last_token_end,
            viewport_modifiers: Vec::new(),
        })
    }

    fn module_definition(&mut self) -> PResult<ModuleNode> {
        let keyword = self.advance();
        let keyword_p = keyword.p;
        let name = self.expect(TT::Ident, Some("Expected module name".to_string()))?;
        let name_v = name.v.clone();
        self.expect(TT::LParen, Some("Expected ( after module name".to_string()))?;
        let mut params = Vec::new();
        while self.peek().t != TT::RParen {
            let param = self.expect(TT::Ident, Some("Expected parameter name".to_string()))?;
            let param_v = param.v.clone();
            let default_value = if self.matches(TT::Eq) {
                Some(self.expression()?)
            } else {
                None
            };
            params.push(ModuleParam {
                name: param_v,
                default_value,
            });
            if !self.matches(TT::Comma) && self.peek().t != TT::RParen {
                return Err(self.fail("Expected , or )"));
            }
        }
        self.advance();
        let children = self.body(true)?;
        Ok(ModuleNode {
            name: name_v,
            params,
            children,
            p: keyword_p,
            end: self.last_token_end,
        })
    }

    fn function_definition(&mut self) -> PResult<FunctionNode> {
        let keyword = self.advance();
        let keyword_p = keyword.p;
        let name = self.expect(TT::Ident, Some("Expected function name".to_string()))?;
        let name_v = name.v.clone();
        let params = self.function_parameters()?;
        self.expect(TT::Eq, Some("Expected = before function body".to_string()))?;
        let body = self.expression()?;
        let terminator = self.expect(
            TT::Semi,
            Some("Expected ; after function definition".to_string()),
        )?;
        Ok(FunctionNode {
            name: name_v,
            params,
            body,
            p: keyword_p,
            end: terminator.end,
        })
    }

    fn function_parameters(&mut self) -> PResult<Vec<ModuleParam>> {
        self.expect(
            TT::LParen,
            Some("Expected ( before function parameters".to_string()),
        )?;
        let mut params = Vec::new();
        let mut names = std::collections::HashSet::new();
        while self.peek().t != TT::RParen {
            let param = self.expect(TT::Ident, Some("Expected parameter name".to_string()))?;
            if names.contains(&param.v) {
                return Err(self.fail_at(param, format!("Duplicate parameter {}", param.v), None));
            }
            names.insert(param.v.clone());
            let default_value = if self.matches(TT::Eq) {
                Some(self.expression()?)
            } else {
                None
            };
            params.push(ModuleParam {
                name: param.v.clone(),
                default_value,
            });
            if !self.matches(TT::Comma) {
                break;
            }
            if self.peek().t == TT::RParen {
                break;
            }
        }
        self.expect(
            TT::RParen,
            Some("Expected ) after function parameters".to_string()),
        )?;
        Ok(params)
    }

    fn call(&mut self) -> PResult<CallNode> {
        let name = self.expect(TT::Ident, None)?;
        let (name_v, name_p) = (name.v.clone(), name.p);
        let mut call_arguments: Vec<ExpressionArgument> = Vec::new();
        let mut args: Vec<(String, Expr)> = Vec::new();
        let mut arg_kinds: Vec<(String, &'static str)> = Vec::new();
        let mut arg_spans: Vec<(String, (usize, usize))> = Vec::new();
        if self.matches(TT::LParen) {
            let mut positional = 0usize;
            while self.peek().t != TT::RParen {
                let key: String;
                let kind: &'static str;
                if self.peek().t == TT::Ident && self.peek_at(1).t == TT::Eq {
                    key = self.advance().v.clone();
                    self.advance();
                    kind = "named";
                } else {
                    key = format!("_{positional}");
                    positional += 1;
                    kind = "positional";
                }
                if args.iter().any(|(k, _)| *k == key) && !self.profile.is_stable() {
                    return Err(self.fail(format!("Duplicate argument {key}")));
                }
                let start = self.peek().p;
                let value = self.expression()?;
                record_insert(&mut args, key.clone(), value.clone());
                record_insert(&mut arg_kinds, key.clone(), kind);
                record_insert(&mut arg_spans, key.clone(), (start, self.last_token_end));
                call_arguments.push(ExpressionArgument {
                    name: (kind == "named").then(|| key.clone()),
                    value,
                    p: start,
                    end: self.last_token_end,
                });
                if !self.matches(TT::Comma) && self.peek().t != TT::RParen {
                    return Err(self.fail("Expected , or )"));
                }
            }
            self.advance();
        } else if self.profile.is_stable() {
            return Err(self.fail(format!("Expected ( after module name {name_v}")));
        }

        let children = self.body(false)?;
        let mut alternative = Vec::new();
        if name_v == "if" && self.peek().t == TT::Ident && self.peek().v == "else" {
            self.advance();
            alternative = self.body(true)?;
        }
        Ok(CallNode {
            name: name_v,
            call_arguments,
            args,
            arg_kinds,
            arg_spans,
            children,
            alternative,
            p: name_p,
            end: self.last_token_end,
            viewport_modifiers: Vec::new(),
            operation_id: None,
        })
    }

    fn body(&mut self, required: bool) -> PResult<Vec<Statement>> {
        if self.matches(TT::LBrace) {
            let mut children = Vec::new();
            while self.peek().t != TT::RBrace && self.peek().t != TT::Eof {
                children.extend(self.statement()?);
            }
            self.expect(TT::RBrace, Some("Expected }".to_string()))?;
            return Ok(children);
        }
        if self.matches(TT::Semi) {
            return Ok(Vec::new());
        }
        if !required && matches!(self.peek().t, TT::RBrace | TT::Eof) {
            return Ok(Vec::new());
        }
        self.statement()
    }

    pub fn expression(&mut self) -> PResult<Expr> {
        let position = self.peek().p;
        self.descend_expression(position, |this| {
            if this.peek().t == TT::Ident && this.peek_at(1).t == TT::LParen {
                let keyword = this.peek().v.as_str();
                if keyword == "function" {
                    if this.profile.is_subset() {
                        return Err(this.fail_coded(
                            "function is not supported by openscad-viewer-subset@1",
                            "E_FEATURE_USER_FUNCTION",
                        ));
                    }
                    return this.anonymous_function_expression();
                }
                if this.profile.is_stable()
                    && (keyword == "let" || keyword == "assert" || keyword == "echo")
                {
                    return this.wrapper_expression(keyword);
                }
            }
            this.ternary()
        })
    }

    fn anonymous_function_expression(&mut self) -> PResult<Expr> {
        let keyword = self.expect(TT::Ident, None)?;
        let keyword_p = keyword.p;
        let params = self.function_parameters()?;
        let body = self.expression()?;
        self.expression_node(Expr::Function {
            params,
            body: Box::new(body),
            p: keyword_p,
        })
    }

    fn wrapper_expression(&mut self, keyword: &str) -> PResult<Expr> {
        let start = self.expect(TT::Ident, None)?.p;
        self.expect(TT::LParen, Some(format!("Expected ( after {keyword}")))?;
        let args = self.expression_arguments(format!("Expected ) after {keyword} arguments"), TT::RParen)?;
        if keyword == "let" {
            let body = self.expression()?;
            return self.expression_node(Expr::Let {
                args,
                body: Box::new(body),
                p: start,
            });
        }
        let body = if self.can_start_expression() {
            Some(Box::new(self.expression()?))
        } else {
            None
        };
        if keyword == "assert" {
            self.expression_node(Expr::Assert { args, body, p: start })
        } else {
            self.expression_node(Expr::Echo { args, body, p: start })
        }
    }

    fn can_start_expression(&self) -> bool {
        let token = self.peek();
        if matches!(
            token.t,
            TT::Num | TT::Str | TT::LParen | TT::LBracket | TT::Plus | TT::Minus | TT::Not
        ) {
            return true;
        }
        if token.t != TT::Ident {
            return false;
        }
        if self.profile.is_stable()
            && matches!(token.v.as_str(), "else" | "for" | "if" | "each")
        {
            return false;
        }
        true
    }

    fn ternary(&mut self) -> PResult<Expr> {
        let test = self.binary_or()?;
        if !self.matches(TT::Question) {
            return Ok(test);
        }
        let yes = self.expression()?;
        self.expect(TT::Colon, Some("Expected : in conditional expression".to_string()))?;
        let no = self.expression()?;
        let p = expr_position(&test);
        self.expression_node(Expr::Ternary {
            test: Box::new(test),
            yes: Box::new(yes),
            no: Box::new(no),
            p,
        })
    }

    fn binary_or(&mut self) -> PResult<Expr> {
        self.binary(Self::binary_and, &[TT::Or])
    }
    fn binary_and(&mut self) -> PResult<Expr> {
        if self.profile.is_stable() {
            self.binary(Self::equality, &[TT::And])
        } else {
            self.binary(Self::legacy_comparison, &[TT::And])
        }
    }
    /// Frozen subset@1 level: equality and ordering operators remain combined.
    fn legacy_comparison(&mut self) -> PResult<Expr> {
        self.binary(
            Self::additive,
            &[TT::Lt, TT::Gt, TT::LtEq, TT::GtEq, TT::EqEq, TT::NotEq],
        )
    }
    fn equality(&mut self) -> PResult<Expr> {
        self.binary(Self::comparison, &[TT::EqEq, TT::NotEq])
    }
    fn comparison(&mut self) -> PResult<Expr> {
        self.binary(Self::additive, &[TT::Lt, TT::Gt, TT::LtEq, TT::GtEq])
    }
    fn additive(&mut self) -> PResult<Expr> {
        self.binary(Self::multiplicative, &[TT::Plus, TT::Minus])
    }
    fn multiplicative(&mut self) -> PResult<Expr> {
        if self.profile.is_stable() {
            self.binary(Self::full_unary, &[TT::Star, TT::Slash, TT::Percent])
        } else {
            self.binary(Self::legacy_power, &[TT::Star, TT::Slash, TT::Percent])
        }
    }

    /// Frozen subset@1 precedence: retained so the versioned legacy route cannot drift.
    fn legacy_power(&mut self) -> PResult<Expr> {
        let left = self.legacy_unary()?;
        if !self.matches(TT::Caret) {
            return Ok(left);
        }
        let p = expr_position(&left);
        let position = self.peek().p;
        let right = self.descend_expression(position, Self::legacy_power)?;
        self.expression_node(Expr::Binary {
            op: TT::Caret,
            left: Box::new(left),
            right: Box::new(right),
            p,
        })
    }

    fn binary(&mut self, next: fn(&mut Self) -> PResult<Expr>, operators: &[TT]) -> PResult<Expr> {
        let mut left = next(self)?;
        while operators.contains(&self.peek().t) {
            let op = self.advance();
            let op_t = op.t;
            let op_p = op.p;
            let right = next(self)?;
            left = self.expression_node(Expr::Binary {
                op: op_t,
                left: Box::new(left),
                right: Box::new(right),
                p: op_p,
            })?;
        }
        Ok(left)
    }

    fn legacy_unary(&mut self) -> PResult<Expr> {
        if matches!(self.peek().t, TT::Plus | TT::Minus | TT::Not) {
            let op = self.advance();
            let (op_t, op_p) = (op.t, op.p);
            let value = self.descend_expression(op_p, Self::legacy_unary)?;
            return self.expression_node(Expr::Unary {
                op: op_t,
                value: Box::new(value),
                p: op_p,
            });
        }
        self.postfix()
    }

    /// OpenSCAD 2021.01 precedence: exponentiation binds tighter than unary.
    fn full_unary(&mut self) -> PResult<Expr> {
        if matches!(self.peek().t, TT::Plus | TT::Minus | TT::Not) {
            let op = self.advance();
            let (op_t, op_p) = (op.t, op.p);
            let value = self.descend_expression(op_p, Self::full_unary)?;
            return self.expression_node(Expr::Unary {
                op: op_t,
                value: Box::new(value),
                p: op_p,
            });
        }
        self.full_power()
    }

    fn full_power(&mut self) -> PResult<Expr> {
        let left = self.postfix()?;
        if !self.matches(TT::Caret) {
            return Ok(left);
        }
        let p = expr_position(&left);
        let position = self.peek().p;
        // Going through unary permits `2 ^ -2` and keeps `2 ^ 3 ^ 2`
        // right-associative, matching the pinned OpenSCAD grammar.
        let right = self.descend_expression(position, Self::full_unary)?;
        self.expression_node(Expr::Binary {
            op: TT::Caret,
            left: Box::new(left),
            right: Box::new(right),
            p,
        })
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut value = self.primary()?;
        loop {
            if self.matches(TT::LBracket) {
                let p = expr_position(&value);
                let index = self.expression()?;
                self.expect(TT::RBracket, Some("Expected ] after index".to_string()))?;
                value = self.expression_node(Expr::Index {
                    value: Box::new(value),
                    index: Box::new(index),
                    p,
                })?;
                continue;
            }
            if self.matches(TT::Dot) {
                let member = self.expect(TT::Ident, Some("Expected member name after .".to_string()))?;
                let member_v = member.v.clone();
                let p = expr_position(&value);
                value = self.expression_node(Expr::Member {
                    value: Box::new(value),
                    name: member_v,
                    p,
                })?;
                continue;
            }
            if self.matches(TT::LParen) {
                let args = self.expression_arguments(
                    "Expected ) after function arguments".to_string(),
                    TT::RParen,
                )?;
                let p = expr_position(&value);
                let name = match &value {
                    Expr::Identifier { name, .. } => Some(name.clone()),
                    _ => None,
                };
                value = self.expression_node(Expr::Call {
                    name,
                    callee: Box::new(value),
                    args,
                    p,
                })?;
                continue;
            }
            return Ok(value);
        }
    }

    /// Parse OpenSCAD's ordered arguments_call production after `(`.
    fn expression_arguments(&mut self, closing_message: String, terminator: TT) -> PResult<Vec<ExpressionArgument>> {
        let mut args = Vec::new();
        let mut named = std::collections::HashSet::new();
        let mut saw_named = false;
        while self.peek().t != terminator {
            let start = self.peek().p;
            let mut name: Option<String> = None;
            if self.peek().t == TT::Ident && self.peek_at(1).t == TT::Eq {
                let name_token = self.advance();
                name = Some(name_token.v.clone());
                self.advance();
                if named.contains(name.as_ref().unwrap()) && self.profile.is_subset() {
                    // Preserve the frozen subset@1 diagnostic position at the value.
                    return Err(self.fail(format!("Duplicate argument {}", name.unwrap())));
                }
                named.insert(name.clone().unwrap());
                saw_named = true;
            } else if saw_named && self.profile.is_subset() {
                return Err(self.fail("Positional arguments must precede named arguments"));
            }
            let argument = self.expression()?;
            args.push(ExpressionArgument {
                name,
                value: argument,
                p: start,
                end: self.last_token_end,
            });
            if !self.matches(TT::Comma) {
                break;
            }
            if self.peek().t == terminator {
                break;
            }
        }
        self.expect(terminator, Some(closing_message))?;
        Ok(args)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let token = self.peek();
        match token.t {
            TT::Num => {
                let token = self.advance();
                // The lexer only emits JS-`Number`-parseable numerals; Rust's
                // f64 parser agrees on all of them (including overflow to inf).
                let value: f64 = token.v.parse().unwrap_or(f64::NAN);
                self.expression_node(Expr::Literal {
                    value: LiteralValue::Number(value),
                    p: token.p,
                })
            }
            TT::Str => {
                let token = self.advance();
                self.expression_node(Expr::Literal {
                    value: LiteralValue::String(token.v.clone()),
                    p: token.p,
                })
            }
            TT::LParen => {
                self.advance();
                let value = self.expression()?;
                self.expect(TT::RParen, Some("Expected )".to_string()))?;
                Ok(value)
            }
            TT::LBracket => {
                let p = token.p;
                self.advance();
                self.vector_or_range(p)
            }
            TT::Ident => {
                let token = self.advance();
                match token.v.as_str() {
                    "true" | "false" => self.expression_node(Expr::Literal {
                        value: LiteralValue::Bool(token.v == "true"),
                        p: token.p,
                    }),
                    "undef" => self.expression_node(Expr::Literal {
                        value: LiteralValue::Undef,
                        p: token.p,
                    }),
                    "function" if self.peek().t == TT::LParen => {
                        if self.profile.is_subset() {
                            return Err(self.fail_at(
                                token,
                                "function is not supported by openscad-viewer-subset@1".to_string(),
                                Some("E_FEATURE_USER_FUNCTION"),
                            ));
                        }
                        Err(self.fail_at(
                            token,
                            "Anonymous function must start an expression".to_string(),
                            None,
                        ))
                    }
                    "let" | "assert" | "echo"
                        if self.profile.is_stable() && self.peek().t == TT::LParen =>
                    {
                        Err(self.fail_at(
                            token,
                            format!("{} expression must start an expression", token.v),
                            None,
                        ))
                    }
                    "for" | "if" | "else" | "each" if self.profile.is_stable() => Err(self.fail_at(
                        token,
                        format!("Expected expression, got {}", token.v),
                        None,
                    )),
                    _ => self.expression_node(Expr::Identifier {
                        name: token.v.clone(),
                        p: token.p,
                    }),
                }
            }
            _ => Err(self.fail_at(
                token,
                format!("Expected expression, got {}", token.display()),
                None,
            )),
        }
    }

    fn vector_or_range(&mut self, p: usize) -> PResult<Expr> {
        if self.matches(TT::RBracket) {
            return self.expression_node(Expr::Vector { items: Vec::new(), p });
        }
        let first = if self.profile.is_stable() && self.is_list_comprehension_start(self.pos, 0) {
            self.list_comprehension()?
        } else {
            self.expression()?
        };
        if self.matches(TT::Colon) {
            if first.is_list_comprehension() {
                return Err(self.fail("A list comprehension cannot start a range"));
            }
            let second = self.expression()?;
            let mut step: Option<Box<Expr>> = None;
            let mut end = second;
            if self.matches(TT::Colon) {
                step = Some(Box::new(end));
                end = self.expression()?;
            }
            self.expect(TT::RBracket, Some("Expected ] after range".to_string()))?;
            return self.expression_node(Expr::Range {
                start: Box::new(first),
                step,
                end: Box::new(end),
                p,
            });
        }
        let mut items = vec![first];
        while self.matches(TT::Comma) {
            if self.peek().t == TT::RBracket {
                break;
            }
            items.push(
                if self.profile.is_stable() && self.is_list_comprehension_start(self.pos, 0) {
                    self.list_comprehension()?
                } else {
                    self.expression()?
                },
            );
        }
        self.expect(TT::RBracket, Some("Expected ]".to_string()))?;
        self.expression_node(Expr::Vector { items, p })
    }

    fn is_list_comprehension_start(&self, index: usize, recursion: usize) -> bool {
        if recursion > MAX_EXPRESSION_DEPTH {
            return false;
        }
        let Some(token) = self.tokens.get(index) else { return false };
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
        let Some(close) = self.matching_right_paren(index + 1) else { return false };
        self.is_list_comprehension_parenthesized_start(close + 1, recursion + 1)
    }

    fn is_list_comprehension_parenthesized_start(&self, index: usize, recursion: usize) -> bool {
        if self.is_list_comprehension_start(index, recursion) {
            return true;
        }
        self.tokens.get(index).map(|t| t.t) == Some(TT::LParen)
            && self.is_list_comprehension_parenthesized_start(index + 1, recursion + 1)
    }

    fn matching_right_paren(&self, open_index: usize) -> Option<usize> {
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

    fn list_comprehension(&mut self) -> PResult<Expr> {
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
                self.expect(TT::Semi, Some("Expected ; after C-style for condition".to_string()))?;
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
            let args = self.expression_arguments("Expected ) after for bindings".to_string(), TT::RParen)?;
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

    fn list_comprehension_or_expression(&mut self) -> PResult<Expr> {
        if self.is_list_comprehension_start(self.pos, 0) {
            return self.list_comprehension();
        }
        if self.peek().t == TT::LParen && self.is_list_comprehension_start(self.pos + 1, 0) {
            return self.parenthesized_list_comprehension();
        }
        self.expression()
    }

    fn parenthesized_list_comprehension(&mut self) -> PResult<Expr> {
        if !self.matches(TT::LParen) {
            return self.list_comprehension();
        }
        let value = self.list_comprehension()?;
        self.expect(TT::RParen, Some("Expected ) after list comprehension".to_string()))?;
        Ok(value)
    }

    fn has_top_level_semicolon_before_right_paren(&self) -> bool {
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

/// `Record<string, V>` semantics: last write wins, first insertion order kept.
fn record_insert<V>(vec: &mut Vec<(String, V)>, key: String, value: V) {
    if let Some(entry) = vec.iter_mut().find(|(k, _)| *k == key) {
        entry.1 = value;
    } else {
        vec.push((key, value));
    }
}

fn expr_position(expr: &Expr) -> usize {
    match expr {
        Expr::Literal { p, .. }
        | Expr::Identifier { p, .. }
        | Expr::Vector { p, .. }
        | Expr::Range { p, .. }
        | Expr::Unary { p, .. }
        | Expr::Binary { p, .. }
        | Expr::Ternary { p, .. }
        | Expr::Function { p, .. }
        | Expr::Call { p, .. }
        | Expr::Index { p, .. }
        | Expr::Member { p, .. }
        | Expr::Let { p, .. }
        | Expr::Assert { p, .. }
        | Expr::Echo { p, .. }
        | Expr::LcFor { p, .. }
        | Expr::LcForC { p, .. }
        | Expr::LcIf { p, .. }
        | Expr::LcLet { p, .. }
        | Expr::LcEach { p, .. } => *p,
    }
}
