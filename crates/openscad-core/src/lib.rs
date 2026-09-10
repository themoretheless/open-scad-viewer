//! OpenSCAD language front-end (lexer, parser, AST) for the two repository
//! language profiles: the frozen `openscad-viewer-subset@1` and
//! `openscad/stable-2021.01`. This is a faithful port of the TypeScript
//! compiler front-end in `src/services/openscadCompiler.ts`; diagnostics keep
//! the exact messages, codes and UTF-16 source positions of the TS parser so
//! the language conformance corpus gates parity.
#![forbid(unsafe_code)]

pub mod ast;
pub mod builtins;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod serialize;
pub mod value;

pub use ast::{ExpressionArgument, ModuleParam, Statement, ViewportModifier};
pub use lexer::{tokenize, TT};
pub use parser::Parser;

/// Mirrors `MAX_SOURCE_LENGTH` in `src/services/openscadParser.ts`.
pub const MAX_SOURCE_LENGTH: usize = 250_000;
/// Mirrors `MAX_AST_NODES` in `src/services/openscadCompiler.ts`.
pub const MAX_AST_NODES: usize = 25_000;
/// Mirrors `MAX_EXPRESSION_DEPTH` in `src/services/openscadCompiler.ts`.
pub const MAX_EXPRESSION_DEPTH: usize = 256;
/// Mirrors `MAX_STATEMENT_DEPTH` in `src/services/openscadCompiler.ts`.
pub const MAX_STATEMENT_DEPTH: usize = 128;

pub const PROFILE_VIEWER_SUBSET: &str = "openscad-viewer-subset@1";
pub const PROFILE_STABLE_2021: &str = "openscad/stable-2021.01";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageProfile {
    ViewerSubset,
    Stable2021,
}

impl LanguageProfile {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            PROFILE_VIEWER_SUBSET => Some(Self::ViewerSubset),
            PROFILE_STABLE_2021 => Some(Self::Stable2021),
            _ => None,
        }
    }
    pub fn is_subset(self) -> bool {
        matches!(self, Self::ViewerSubset)
    }
    pub fn is_stable(self) -> bool {
        matches!(self, Self::Stable2021)
    }
}

/// A positioned parse diagnostic, structurally identical to the TS
/// `OpenSCADParseError` (positions are UTF-16 code-unit offsets).
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub code: Option<&'static str>,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Internal raised error: raw span before line/column resolution.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub code: Option<&'static str>,
    pub position: usize,
    pub end_position: Option<usize>,
}

impl ParseError {
    pub fn new(position: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            position,
            end_position: None,
        }
    }
    pub fn coded(position: usize, message: impl Into<String>, code: &'static str) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            position,
            end_position: None,
        }
    }
    pub fn with_end(mut self, end: usize) -> Self {
        self.end_position = Some(end);
        self
    }
    /// Resolve against the UTF-16 source units exactly like the TS error class.
    pub fn resolve(self, units: &[u16]) -> Diagnostic {
        let safe = self.position.min(units.len());
        let before = &units[..safe];
        let line = before.iter().filter(|&&u| u == b'\n' as u16).count() + 1;
        let line_start = before.iter().rposition(|&u| u == b'\n' as u16).map_or(0, |i| i + 1);
        let column = safe - line_start + 1;
        let end = self
            .end_position
            .unwrap_or(safe + 1)
            .min(units.len())
            .max(safe);
        Diagnostic {
            message: self.message,
            code: self.code,
            start: safe,
            end,
            line,
            column,
        }
    }
}

/// Pure compiler front-end: source text to a statement AST, mirroring the TS
/// `compileOpenSCAD` (including the single-source directive rejection).
pub fn compile(source: &str, profile: LanguageProfile) -> Result<Vec<Statement>, Diagnostic> {
    let units: Vec<u16> = source.encode_utf16().collect();
    compile_units(&units, profile)
}

/// Same as [`compile`] but over pre-decoded UTF-16 units (positions stay in
/// UTF-16 code units to match the TypeScript parser).
pub fn compile_units(units: &[u16], profile: LanguageProfile) -> Result<Vec<Statement>, Diagnostic> {
    let tokens = tokenize(units).map_err(|e| e.resolve(units))?;
    let mut statements = Parser::new(&tokens, profile)
        .parse_all()
        .map_err(|e| e.resolve(units))?;
    if let Some(directive) = find_directive(&statements) {
        return Err(ParseError::new(
            directive.p,
            format!("{} requires project compilation", directive.directive),
        )
        .with_end(directive.end)
        .resolve(units));
    }
    assign_operation_ids(&mut statements, &["root".to_string()]);
    Ok(statements)
}

fn find_directive(nodes: &[Statement]) -> Option<&ast::DirectiveNode> {
    for node in nodes {
        match node {
            Statement::Directive(directive) => return Some(directive),
            Statement::Call(call) => {
                if let Some(found) = find_directive(&call.children) {
                    return Some(found);
                }
                if let Some(found) = find_directive(&call.alternative) {
                    return Some(found);
                }
            }
            Statement::Module(module) => {
                if let Some(found) = find_directive(&module.children) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Port of `encodeURIComponent` for the identifier-shaped path segments used
/// by operation ids (names may contain `$` and `_`).
fn encode_uri_component(segment: &str) -> String {
    const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
    let mut out = String::new();
    for byte in segment.as_bytes() {
        if UNRESERVED.contains(byte) {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Port of `assignOperationIds`: per-sibling occurrence counting so
/// whitespace, comments and unrelated siblings do not move an identity.
pub fn assign_operation_ids(nodes: &mut [Statement], parent: &[String]) {
    let mut occurrences = std::collections::HashMap::<String, usize>::new();
    for node in nodes.iter_mut() {
        if matches!(node, Statement::Directive(_)) {
            continue;
        }
        let key = format!("{}:{}", node.type_name(), node.name());
        let occurrence = occurrences.entry(key.clone()).or_insert(0);
        let index = *occurrence;
        *occurrence += 1;
        let mut path = parent.to_vec();
        path.push(format!("{key}#{index}"));
        let encoded: Vec<String> = path.iter().map(|s| encode_uri_component(s)).collect();
        match node {
            Statement::Call(call) => {
                call.operation_id = Some(format!("op:{}", encoded.join("/")));
                let mut child_parent = path.clone();
                child_parent.push("children".to_string());
                assign_operation_ids(&mut call.children, &child_parent);
                let mut alt_parent = path;
                alt_parent.push("alternative".to_string());
                assign_operation_ids(&mut call.alternative, &alt_parent);
            }
            Statement::Module(module) => {
                let mut body_parent = path;
                body_parent.push("body".to_string());
                assign_operation_ids(&mut module.children, &body_parent);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod eval_tests;
