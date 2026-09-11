//! AST node types mirroring `src/services/openscadCompiler.ts`.
use crate::lexer::TT;

/// Literal scalar carried by `Expr::Literal`; `Undef` is the OpenSCAD
/// `undef` literal (TS `undefined`).
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Undef,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionArgument {
    pub name: Option<String>,
    pub value: Expr,
    pub p: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleParam {
    pub name: String,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportModifier {
    pub kind: &'static str,
    pub token: char,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: LiteralValue,
        p: usize,
    },
    Identifier {
        name: String,
        p: usize,
    },
    Vector {
        items: Vec<Expr>,
        p: usize,
    },
    Range {
        start: Box<Expr>,
        step: Option<Box<Expr>>,
        end: Box<Expr>,
        p: usize,
    },
    Unary {
        op: TT,
        value: Box<Expr>,
        p: usize,
    },
    Binary {
        op: TT,
        left: Box<Expr>,
        right: Box<Expr>,
        p: usize,
    },
    Ternary {
        test: Box<Expr>,
        yes: Box<Expr>,
        no: Box<Expr>,
        p: usize,
    },
    Function {
        params: Vec<ModuleParam>,
        body: Box<Expr>,
        p: usize,
    },
    Call {
        /// Fast-path name for an identifier callee; None for a computed callee.
        name: Option<String>,
        callee: Box<Expr>,
        args: Vec<ExpressionArgument>,
        p: usize,
    },
    Index {
        value: Box<Expr>,
        index: Box<Expr>,
        p: usize,
    },
    Member {
        value: Box<Expr>,
        name: String,
        p: usize,
    },
    Let {
        args: Vec<ExpressionArgument>,
        body: Box<Expr>,
        p: usize,
    },
    Assert {
        args: Vec<ExpressionArgument>,
        /// Absence is different from an explicit `undef` body.
        body: Option<Box<Expr>>,
        p: usize,
    },
    Echo {
        args: Vec<ExpressionArgument>,
        body: Option<Box<Expr>>,
        p: usize,
    },
    LcFor {
        args: Vec<ExpressionArgument>,
        body: Box<Expr>,
        p: usize,
    },
    LcForC {
        init: Vec<ExpressionArgument>,
        condition: Box<Expr>,
        update: Vec<ExpressionArgument>,
        body: Box<Expr>,
        p: usize,
    },
    LcIf {
        condition: Box<Expr>,
        yes: Box<Expr>,
        no: Option<Box<Expr>>,
        p: usize,
    },
    LcLet {
        args: Vec<ExpressionArgument>,
        body: Box<Expr>,
        p: usize,
    },
    LcEach {
        value: Box<Expr>,
        p: usize,
    },
}

impl Expr {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Literal { .. } => "literal",
            Self::Identifier { .. } => "identifier",
            Self::Vector { .. } => "vector",
            Self::Range { .. } => "range",
            Self::Unary { .. } => "unary",
            Self::Binary { .. } => "binary",
            Self::Ternary { .. } => "ternary",
            Self::Function { .. } => "function",
            Self::Call { .. } => "call",
            Self::Index { .. } => "index",
            Self::Member { .. } => "member",
            Self::Let { .. } => "let",
            Self::Assert { .. } => "assert",
            Self::Echo { .. } => "echo",
            Self::LcFor { .. } => "lc-for",
            Self::LcForC { .. } => "lc-for-c",
            Self::LcIf { .. } => "lc-if",
            Self::LcLet { .. } => "lc-let",
            Self::LcEach { .. } => "lc-each",
        }
    }
    pub fn is_list_comprehension(&self) -> bool {
        matches!(
            self,
            Self::LcFor { .. }
                | Self::LcForC { .. }
                | Self::LcIf { .. }
                | Self::LcLet { .. }
                | Self::LcEach { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub name: String,
    /// Complete authored order, including duplicate named arguments.
    pub call_arguments: Vec<ExpressionArgument>,
    /// Last-value compatibility index used by legacy call sites; insertion
    /// order of first occurrence is preserved like the TS Record.
    pub args: Vec<(String, Expr)>,
    pub arg_kinds: Vec<(String, &'static str)>,
    pub arg_spans: Vec<(String, (usize, usize))>,
    pub children: Vec<Statement>,
    pub alternative: Vec<Statement>,
    pub p: usize,
    pub end: usize,
    pub viewport_modifiers: Vec<ViewportModifier>,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignNode {
    pub name: String,
    pub value: Expr,
    pub p: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleNode {
    pub name: String,
    pub params: Vec<ModuleParam>,
    pub children: Vec<Statement>,
    pub p: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: String,
    pub params: Vec<ModuleParam>,
    pub body: Expr,
    pub p: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirectiveNode {
    pub directive: &'static str,
    pub path: String,
    pub path_span: (usize, usize),
    pub p: usize,
    pub end: usize,
    pub viewport_modifiers: Vec<ViewportModifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Call(CallNode),
    Assign(AssignNode),
    Module(ModuleNode),
    Function(FunctionNode),
    Directive(DirectiveNode),
}

impl Statement {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Call(_) => "call",
            Self::Assign(_) => "assign",
            Self::Module(_) => "module",
            Self::Function(_) => "function",
            Self::Directive(_) => "directive",
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Call(node) => &node.name,
            Self::Assign(node) => &node.name,
            Self::Module(node) => &node.name,
            Self::Function(node) => &node.name,
            Self::Directive(node) => node.directive,
        }
    }
}
