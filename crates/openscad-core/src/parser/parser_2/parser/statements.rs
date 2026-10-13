use super::super::*;

impl<'a> Parser<'a> {
    pub(super) fn statement(&mut self) -> PResult<Vec<Statement>> {
        if self.statement_depth >= MAX_STATEMENT_DEPTH {
            return Err(self.fail(format!(
                "Model exceeds {MAX_STATEMENT_DEPTH} nested statements"
            )));
        }
        self.statement_depth += 1;
        let result = self.parse_statement();
        self.statement_depth -= 1;
        result
    }

    pub(super) fn parse_statement(&mut self) -> PResult<Vec<Statement>> {
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
                let feature = if self.peek().v == "include" {
                    "include"
                } else {
                    "use"
                };
                let code = if feature == "include" {
                    "E_FEATURE_INCLUDE"
                } else {
                    "E_FEATURE_USE"
                };
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
                Statement::Directive(directive) => {
                    directive.viewport_modifiers = viewport_modifiers
                }
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

    pub(super) fn fail_coded(&self, message: impl Into<String>, code: &'static str) -> ParseError {
        let token = self.peek();
        ParseError {
            message: message.into(),
            code: Some(code),
            position: token.p,
            end_position: Some(token.end),
        }
    }

    pub(super) fn assignment(&mut self) -> PResult<AssignNode> {
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

    pub(super) fn directive(&mut self) -> PResult<DirectiveNode> {
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
            directive: if keyword_v == "include" {
                "include"
            } else {
                "use"
            },
            path: path.v.clone(),
            path_span,
            p: keyword_p,
            end: self.last_token_end,
            viewport_modifiers: Vec::new(),
        })
    }

    pub(super) fn module_definition(&mut self) -> PResult<ModuleNode> {
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

    pub(super) fn function_definition(&mut self) -> PResult<FunctionNode> {
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

    pub(super) fn function_parameters(&mut self) -> PResult<Vec<ModuleParam>> {
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

    pub(super) fn call(&mut self) -> PResult<CallNode> {
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

                let kind: &'static str =
                    if self.peek().t == TT::Ident && self.peek_at(1).t == TT::Eq {
                        key = self.advance().v.clone();
                        self.advance();
                        "named"
                    } else {
                        key = format!("_{positional}");
                        positional += 1;
                        "positional"
                    };
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

    pub(super) fn body(&mut self, required: bool) -> PResult<Vec<Statement>> {
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
}
