//! Unit tests for the OpenSCAD front-end, modeled on the language conformance
//! corpus (`tests/fixtures/language-conformance-v1.json`) plus grammar, limit
//! and diagnostic-position cases cross-checked against the TypeScript parser.
use crate::ast::Statement;
use crate::{compile, Diagnostic, LanguageProfile};

const SUBSET: LanguageProfile = LanguageProfile::ViewerSubset;
const STABLE: LanguageProfile = LanguageProfile::Stable2021;

fn ok(source: &str, profile: LanguageProfile) -> Vec<Statement> {
    match compile(source, profile) {
        Ok(statements) => statements,
        Err(d) => panic!("expected {source:?} to compile, got {d:?}"),
    }
}

fn err(source: &str, profile: LanguageProfile) -> Diagnostic {
    match compile(source, profile) {
        Ok(_) => panic!("expected {source:?} to be rejected"),
        Err(d) => d,
    }
}

fn top_level(statements: &[Statement]) -> Vec<String> {
    statements
        .iter()
        .map(|s| match s {
            Statement::Call(call) => call.name.clone(),
            other => other.type_name().to_string(),
        })
        .collect()
}

#[test]
fn corpus_compile_cases() {
    assert_eq!(top_level(&ok("cube([2, 3, 4]);", SUBSET)), ["cube"]);
    assert_eq!(
        top_level(&ok(
            "translate([1, 2, 3]) rotate([0, 0, 45]) sphere(r = 2, $fn = 16);",
            SUBSET
        )),
        ["translate"]
    );
    assert_eq!(
        top_level(&ok(
            "module peg(x) { translate([x, 0, 0]) children(); } for (i = [0:2]) peg(i) cube(1);",
            SUBSET
        )),
        ["module", "for"]
    );
    // The subset `*` disable modifier drops the statement entirely.
    assert_eq!(top_level(&ok("*cube(1); sphere(1);", SUBSET)), ["sphere"]);
}

#[test]
fn corpus_reject_codes() {
    assert_eq!(err("include <part.scad>", SUBSET).code, Some("E_FEATURE_INCLUDE"));
    assert_eq!(err("use <part.scad>", SUBSET).code, Some("E_FEATURE_USE"));
    assert_eq!(
        err("function f(x) = x; cube(f(1));", SUBSET).code,
        Some("E_FEATURE_USER_FUNCTION")
    );
    assert_eq!(
        err("#cube(1);", SUBSET).code,
        Some("E_FEATURE_VIEWPORT_MODIFIER")
    );
}

#[test]
fn corpus_reject_positions() {
    // Cross-checked against the TS parser (UTF-16 offsets, 1-based line/column).
    let d = err("include <part.scad>", SUBSET);
    assert_eq!((d.start, d.end, d.line, d.column), (0, 7, 1, 1));
    assert_eq!(d.message, "include is not supported by openscad-viewer-subset@1");
    let d = err("#cube(1);", SUBSET);
    assert_eq!((d.start, d.end, d.line, d.column), (0, 1, 1, 1));
    assert_eq!(
        d.message,
        "Viewport modifier # is not supported by openscad-viewer-subset@1"
    );
}

#[test]
fn full_profile_directives_require_project_compilation() {
    let d = err("include <part.scad>", STABLE);
    assert_eq!(d.code, None);
    assert_eq!(d.message, "include requires project compilation");
    assert_eq!((d.start, d.line, d.column), (0, 1, 1));
    let d = err("use <lib.scad>\ncube(1);", STABLE);
    assert_eq!(d.message, "use requires project compilation");
}

#[test]
fn viewport_modifier_on_assignment_is_rejected() {
    let d = err("#x = 1;", STABLE);
    assert_eq!(
        d.message,
        "Viewport modifiers may prefix only a module instantiation or an include/use directive"
    );
    assert_eq!((d.start, d.end), (0, 1));
}

#[test]
fn lexer_diagnostics() {
    let d = err("cube(1); /* dangling", SUBSET);
    assert_eq!(d.message, "Unterminated block comment");
    assert_eq!((d.start, d.line, d.column), (9, 1, 10));
    let d = err("cube(\"abc);", SUBSET);
    assert_eq!(d.message, "Unterminated string");
    assert_eq!((d.start, d.end), (5, 6));
    let d = err("cube(1e+);", SUBSET);
    assert_eq!(d.message, "Invalid exponent");
    assert_eq!((d.start, d.end), (5, 6));
    let d = err("cube(@);", SUBSET);
    assert_eq!(d.message, "Unexpected character \"@\"");
    let d = err("x = 1;\ny = @;", SUBSET);
    assert_eq!((d.line, d.column), (2, 5));
}

#[test]
fn parser_diagnostics_match_ts_wording() {
    assert_eq!(err("x = 1", SUBSET).message, "Expected ; after assignment");
    assert_eq!(err("cube(1 2);", SUBSET).message, "Expected , or )");
    assert_eq!(err("if (true) { cube(1);", SUBSET).message, "Expected }");
    assert_eq!(err("cube(a=1, a=2);", SUBSET).message, "Duplicate argument a");
    assert_eq!(
        err("x = f(a=1, 2);", SUBSET).message,
        "Positional arguments must precede named arguments"
    );
    assert_eq!(err("x = ;", SUBSET).message, "Expected expression, got ;");
    assert_eq!(err("x =", SUBSET).message, "Expected expression, got Eof");
    assert_eq!(err("}", SUBSET).message, "Expected a variable, module, or geometry call");
    assert_eq!(err("{ cube(1); }", SUBSET).message, "Expected a variable, module, or geometry call");
    // 2021.01-only forms.
    assert_eq!(
        err("function f(x, x) = x;", STABLE).message,
        "Duplicate parameter x"
    );
    assert_eq!(err("cube;", STABLE).message, "Expected ( after module name cube");
    assert_eq!(
        err("x = 1 + let(a = 1) a;", STABLE).message,
        "let expression must start an expression"
    );
    assert_eq!(err("x = 1 + for;", STABLE).message, "Expected expression, got for");
    assert_eq!(
        err("x = [for (i = [0:2]) i : 3];", STABLE).message,
        "A list comprehension cannot start a range"
    );
}

#[test]
fn grammar_accepts_both_profiles() {
    for profile in [SUBSET, STABLE] {
        ok("x = 1 + 2 * 3 ^ 2 - -4;", profile);
        ok("y = a ? b : c;", profile);
        ok("z = [0:2:10]; w = [1, 2, 3][0]; v = a.x;", profile);
        ok("for (i = [0:2]) if (i > 0) cube(i); else sphere(1);", profile);
        ok("module m(a, b = 2) { children(); } m(1);", profile);
        ok("r = !true == false || 1 < 2 && 3 >= 4;", profile);
    }
    // 2021.01-only grammar.
    ok("x = let(a = 1, b = 2) a + b;", STABLE);
    ok("y = assert(x > 0) echo(\"v\") x;", STABLE);
    ok("f = function(v) v * 2;", STABLE);
    ok("xs = [for (i = [0:4]) if (i % 2 == 0) i];", STABLE);
    ok("ys = [for (i = 0; i < 4; i = i + 1) i];", STABLE);
    ok("zs = [each [1, 2], let(q = 3) (for (k = [0:q]) k)];", STABLE);
    ok("%cube(1); #sphere(1); !cylinder(1); *square(1);", STABLE);
    ok("{ cube(1); { sphere(1); } }", STABLE);
}

#[test]
fn subset_power_binds_tighter_than_unary() {
    // Frozen subset precedence: `-2^2` parses as `-(2^2)` is NOT the subset
    // rule; subset parses unary first: (-2)^2. Both parse; only the AST
    // shape differs. Verify the subset keeps legacy shape (unary outermost).
    let statements = ok("x = -2^2;", SUBSET);
    let Statement::Assign(assign) = &statements[0] else { panic!("assign") };
    assert_eq!(assign.value.kind_name(), "binary");
    let stable = ok("x = -2^2;", STABLE);
    let Statement::Assign(assign) = &stable[0] else { panic!("assign") };
    assert_eq!(assign.value.kind_name(), "unary");
}

#[test]
fn number_literals_follow_f64_semantics() {
    // Same numeral forms as JS Number(): fractions, exponents, overflow.
    ok("a = .5; b = 1.; c = 1e3; d = 2.5E-2; e = 1e999;", SUBSET);
}

/// Deep-recursion cases need more stack than the default 2 MiB test thread.
fn with_big_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
fn ast_node_limit() {
    with_big_stack(|| {
        // `cube;` costs exactly one node per statement (no argument expressions).
        let source = "cube;".repeat(crate::MAX_AST_NODES + 1);
        let d = err(&source, SUBSET);
        assert_eq!(d.message, "Model exceeds the 25,000 syntax node limit");
        let source = "cube;".repeat(crate::MAX_AST_NODES);
        ok(&source, SUBSET);
    });
}

#[test]
fn expression_depth_limit() {
    with_big_stack(|| {
        // 255 nested parentheses after `x = ` reach depth 256; 256 exceed it.
        let nested = format!("x = {}1{};", "(".repeat(255), ")".repeat(255));
        ok(&nested, SUBSET);
        let too_deep = format!("x = {}1{};", "(".repeat(256), ")".repeat(256));
        let d = err(&too_deep, SUBSET);
        assert_eq!(d.message, "Expression exceeds 256 nested levels");
    });
}

#[test]
fn statement_depth_limit() {
    with_big_stack(|| {
        // 127 nested blocks leave the inner statement at depth 127; 128 fail.
        let nested = format!("{}cube(1);{}", "{".repeat(127), "}".repeat(127));
        ok(&nested, STABLE);
        let too_deep = format!("{}cube(1);{}", "{".repeat(128), "}".repeat(128));
        let d = err(&too_deep, STABLE);
        assert_eq!(d.message, "Model exceeds 128 nested statements");
    });
}

#[test]
fn operation_ids_are_stable() {
    let statements = ok("cube(1); translate([1,0,0]) cube(2);", SUBSET);
    let Statement::Call(first) = &statements[0] else { panic!("call") };
    assert_eq!(
        first.operation_id.as_deref(),
        Some("op:root/call%3Acube%230")
    );
    let Statement::Call(second) = &statements[1] else { panic!("call") };
    assert_eq!(
        second.operation_id.as_deref(),
        Some("op:root/call%3Atranslate%230")
    );
    let Statement::Call(child) = &second.children[0] else { panic!("call") };
    assert_eq!(
        child.operation_id.as_deref(),
        Some("op:root/call%3Atranslate%230/children/call%3Acube%230")
    );
}

#[test]
fn utf16_positions_match_ts() {
    // TS positions count UTF-16 code units: the emoji is 2 units.
    let d = err("// \u{1f600}\n@", SUBSET);
    assert_eq!((d.start, d.line, d.column), (6, 2, 1));
}
