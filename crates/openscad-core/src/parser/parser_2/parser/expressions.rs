use super::super::*;

impl<'a> Parser<'a> {
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

    pub(super) fn anonymous_function_expression(&mut self) -> PResult<Expr> {
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

    pub(super) fn wrapper_expression(&mut self, keyword: &str) -> PResult<Expr> {
        let start = self.expect(TT::Ident, None)?.p;
        self.expect(TT::LParen, Some(format!("Expected ( after {keyword}")))?;
        let args =
            self.expression_arguments(format!("Expected ) after {keyword} arguments"), TT::RParen)?;
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
            self.expression_node(Expr::Assert {
                args,
                body,
                p: start,
            })
        } else {
            self.expression_node(Expr::Echo {
                args,
                body,
                p: start,
            })
        }
    }

    pub(super) fn can_start_expression(&self) -> bool {
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
        if self.profile.is_stable() && matches!(token.v.as_str(), "else" | "for" | "if" | "each") {
            return false;
        }
        true
    }

    pub(super) fn ternary(&mut self) -> PResult<Expr> {
        let test = self.binary_or()?;
        if !self.matches(TT::Question) {
            return Ok(test);
        }
        let yes = self.expression()?;
        self.expect(
            TT::Colon,
            Some("Expected : in conditional expression".to_string()),
        )?;
        let no = self.expression()?;
        let p = expr_position(&test);
        self.expression_node(Expr::Ternary {
            test: Box::new(test),
            yes: Box::new(yes),
            no: Box::new(no),
            p,
        })
    }

    pub(super) fn binary_or(&mut self) -> PResult<Expr> {
        self.binary(Self::binary_and, &[TT::Or])
    }
    pub(super) fn binary_and(&mut self) -> PResult<Expr> {
        if self.profile.is_stable() {
            self.binary(Self::equality, &[TT::And])
        } else {
            self.binary(Self::legacy_comparison, &[TT::And])
        }
    }
    /// Frozen subset@1 level: equality and ordering operators remain combined.
    pub(super) fn legacy_comparison(&mut self) -> PResult<Expr> {
        self.binary(
            Self::additive,
            &[TT::Lt, TT::Gt, TT::LtEq, TT::GtEq, TT::EqEq, TT::NotEq],
        )
    }
    pub(super) fn equality(&mut self) -> PResult<Expr> {
        self.binary(Self::comparison, &[TT::EqEq, TT::NotEq])
    }
    pub(super) fn comparison(&mut self) -> PResult<Expr> {
        self.binary(Self::additive, &[TT::Lt, TT::Gt, TT::LtEq, TT::GtEq])
    }
    pub(super) fn additive(&mut self) -> PResult<Expr> {
        self.binary(Self::multiplicative, &[TT::Plus, TT::Minus])
    }
    pub(super) fn multiplicative(&mut self) -> PResult<Expr> {
        if self.profile.is_stable() {
            self.binary(Self::full_unary, &[TT::Star, TT::Slash, TT::Percent])
        } else {
            self.binary(Self::legacy_power, &[TT::Star, TT::Slash, TT::Percent])
        }
    }

    /// Frozen subset@1 precedence: retained so the versioned legacy route cannot drift.
    pub(super) fn legacy_power(&mut self) -> PResult<Expr> {
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

    pub(super) fn binary(&mut self, next: fn(&mut Self) -> PResult<Expr>, operators: &[TT]) -> PResult<Expr> {
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

    pub(super) fn legacy_unary(&mut self) -> PResult<Expr> {
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
    pub(super) fn full_unary(&mut self) -> PResult<Expr> {
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

    pub(super) fn full_power(&mut self) -> PResult<Expr> {
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

    pub(super) fn postfix(&mut self) -> PResult<Expr> {
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
                let member =
                    self.expect(TT::Ident, Some("Expected member name after .".to_string()))?;
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
    pub(super) fn expression_arguments(
        &mut self,
        closing_message: String,
        terminator: TT,
    ) -> PResult<Vec<ExpressionArgument>> {
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

    pub(super) fn primary(&mut self) -> PResult<Expr> {
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
                    "for" | "if" | "else" | "each" if self.profile.is_stable() => Err(
                        self.fail_at(token, format!("Expected expression, got {}", token.v), None)
                    ),
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

    pub(super) fn vector_or_range(&mut self, p: usize) -> PResult<Expr> {
        if self.matches(TT::RBracket) {
            return self.expression_node(Expr::Vector {
                items: Vec::new(),
                p,
            });
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
}
