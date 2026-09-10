//! Evaluator unit tests (migration stage 2): values, scopes, statements,
//! argument binding, echo/assert, list comprehensions, built-ins and budgets,
//! in both language profiles. Expectations are transcribed from the TypeScript
//! evaluator semantics (`openscadParser.ts`, `openScadValueSemantics.ts`,
//! `openScadBuiltinFunctions.ts`).
use crate::eval::{evaluate_source, Evaluation, EvaluatorOptions, MAX_EVAL_OPS};
use crate::value::{
    compare_values, deep_equal, format_number, format_value, js_number_to_string, js_to_precision,
    truthy, Value,
};
use crate::LanguageProfile;

const SUBSET: LanguageProfile = LanguageProfile::ViewerSubset;
const STABLE: LanguageProfile = LanguageProfile::Stable2021;

/// Deep-recursion and budget tests run on a large stack: 128 nested user
/// function calls (the OpenSCAD limit) need more than the 2 MiB test-thread
/// default in debug builds.
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

fn eval_ok(source: &str, profile: LanguageProfile) -> Evaluation {
    match evaluate_source(source, profile, EvaluatorOptions::default()) {
        Ok(evaluation) => evaluation,
        Err(diagnostic) => panic!("expected success for {source:?}, got {} @{}", diagnostic.message, diagnostic.start),
    }
}

fn eval_err(source: &str, profile: LanguageProfile) -> String {
    match evaluate_source(source, profile, EvaluatorOptions::default()) {
        Ok(_) => panic!("expected failure for {source:?}"),
        Err(diagnostic) => diagnostic.message,
    }
}

fn shape_count(source: &str, profile: LanguageProfile) -> usize {
    eval_ok(source, profile).shapes.len()
}

// ---------------------------------------------------------------- values ---

#[test]
fn truthiness_follows_openscad() {
    assert!(!truthy(&Value::<'static>::Undef));
    assert!(!truthy(&Value::Bool(false)));
    assert!(truthy(&Value::Bool(true)));
    assert!(!truthy(&Value::Number(0.0)));
    assert!(truthy(&Value::Number(f64::NAN)));
    assert!(!truthy(&Value::string("")));
    assert!(truthy(&Value::string("0")));
    assert!(!truthy(&Value::<'static>::vector(vec![])));
    assert!(truthy(&Value::vector(vec![Value::Number(0.0)])));
    assert!(truthy(&Value::<'static>::Range { start: 1.0, step: 1.0, end: 0.0 }));
}

#[test]
fn deep_equal_covers_ranges_and_vectors() {
    assert!(deep_equal(
        &Value::<'static>::vector(vec![Value::Number(1.0), Value::Bool(true)]),
        &Value::vector(vec![Value::Number(1.0), Value::Bool(true)]),
    ));
    assert!(!deep_equal(
        &Value::<'static>::vector(vec![Value::Number(1.0)]),
        &Value::vector(vec![Value::Bool(true)]),
    ));
    assert!(deep_equal(
        &Value::<'static>::Range { start: 0.0, step: 1.0, end: 3.0 },
        &Value::Range { start: 0.0, step: 1.0, end: 3.0 },
    ));
    assert!(!deep_equal(
        &Value::<'static>::Range { start: 0.0, step: 1.0, end: 3.0 },
        &Value::Range { start: 0.0, step: 1.0, end: 4.0 },
    ));
    assert!(deep_equal(
        &Value::<'static>::Range { start: 5.0, step: 1.0, end: 0.0 },
        &Value::Range { start: 1.0, step: 2.0, end: 0.0 },
    ));
}

#[test]
fn comparison_orders_vectors_lexicographically() {
    let a = Value::<'static>::vector(vec![Value::Number(1.0), Value::Number(2.0)]);
    let b = Value::vector(vec![Value::Number(1.0), Value::Number(3.0)]);
    assert!(compare_values(&a, &b).unwrap() < 0.0);
    assert!(compare_values(&Value::Str("a".into()), &Value::Str("b".into())).unwrap() < 0.0);
    assert!(compare_values(&Value::Bool(false), &Value::Bool(true)).unwrap() < 0.0);
    assert!(compare_values(&Value::Undef, &Value::Undef).is_none());
}

#[test]
fn js_number_formatting_matches_ecmascript() {
    assert_eq!(js_number_to_string(24.0), "24");
    assert_eq!(js_number_to_string(-0.0), "0");
    assert_eq!(js_number_to_string(1e21), "1e+21");
    assert_eq!(js_number_to_string(1e20), "100000000000000000000");
    assert_eq!(js_number_to_string(1e-7), "1e-7");
    assert_eq!(js_number_to_string(1.5e-6), "0.0000015");
    assert_eq!(js_number_to_string(0.1 + 0.2), "0.30000000000000004");
    assert_eq!(js_number_to_string(std::f64::consts::PI), "3.141592653589793");
    assert_eq!(js_to_precision(123.456, 6), "123.456");
    assert_eq!(js_to_precision(0.000123456, 6), "0.000123456");
    assert_eq!(js_to_precision(1e-7, 6), "1.00000e-7");
    assert_eq!(js_to_precision(1234567.0, 6), "1.23457e+6");
}

#[test]
fn openscad_value_formatting_uses_six_significant_digits() {
    assert_eq!(format_value(&Value::Undef), "undef");
    assert_eq!(format_value(&Value::Bool(true)), "true");
    assert_eq!(format_value(&Value::Number(1.0 / 3.0)), "0.333333");
    assert_eq!(format_value(&Value::Number(-0.0)), "0");
    assert_eq!(format_value(&Value::Number(f64::INFINITY)), "inf");
    assert_eq!(format_value(&Value::Number(f64::NAN)), "nan");
    assert_eq!(format_value(&Value::string("a\"b\n")), "\"a\\\"b\\n\"");
    assert_eq!(
        format_value(&Value::vector(vec![Value::Number(1.0), Value::string("x")])),
        "[1, \"x\"]"
    );
    assert_eq!(
        format_value(&Value::Range { start: 0.0, step: 2.0, end: 10.0 }),
        "[0 : 2 : 10]"
    );
    assert_eq!(format_number(1.23456789), "1.23457");
    assert_eq!(format_number(1e-7), "1e-7");
    assert_eq!(format_number(1e-6), "1e-6");
    assert_eq!(format_number(100000.0), "100000");
    assert_eq!(format_number(1000000.0), "1e+6");
}

// ------------------------------------------------------- subset evaluator ---

#[test]
fn subset_variables_and_expressions() {
    assert_eq!(shape_count("x = 2; cube([x, 1 + 2, 4]);", SUBSET), 1);
    assert_eq!(shape_count("cube(2);", SUBSET), 1);
    assert_eq!(eval_err("cube(y);", SUBSET), "Unknown variable y");
    assert_eq!(eval_err("cube(1/0);", SUBSET), "Expression produced a non-finite number");
    assert_eq!(eval_err("cube([0:0:1]);", SUBSET), "Range step cannot be zero");
    assert_eq!(eval_err("cube(1 < \"a\");", SUBSET), "comparison operand must be a finite number");
}

#[test]
fn subset_control_flow_and_loops() {
    assert_eq!(shape_count("for (i = [0:2]) translate([i * 2, 0, 0]) cube(1);", SUBSET), 3);
    assert_eq!(shape_count("assert(true) if (true) let(x = 2) cube(x);", SUBSET), 1);
    assert_eq!(shape_count("if (false) cube(1); else cube(2);", SUBSET), 1);
    assert_eq!(shape_count("if (0) cube(1);", SUBSET), 0);
    assert_eq!(shape_count("for (i = []) cube(i);", SUBSET), 0);
    assert_eq!(
        eval_err("for (i = [0:1], j = [0:1]) cube(1);", SUBSET),
        "for() currently requires one named iterator"
    );
    assert_eq!(eval_err("for (i = 5) cube(1);", SUBSET), "for() iterator must be a vector or range");
}

#[test]
fn subset_statements_and_modules() {
    assert_eq!(shape_count("module peg(x) { translate([x, 0, 0]) children(); } for (i = [0:2]) peg(i) cube(1);", SUBSET), 3);
    assert_eq!(shape_count("module pair() { cube(1); translate([2,0,0]) cube(1); } pair();", SUBSET), 2);
    assert_eq!(shape_count("module pick(i) { children(i); } pick(1) { cube(1); cube(2); }", SUBSET), 1);
    assert_eq!(shape_count("module pick(i) { children(i); } pick(9) { cube(1); cube(2); }", SUBSET), 0);
    // Defaults see parameters bound earlier in the same call.
    assert_eq!(shape_count("module m(a, b = a + 1) { cube([a, b, 1]); } m(1);", SUBSET), 1);
}

#[test]
fn subset_geometry_argument_validation_is_kept() {
    assert_eq!(eval_err("cube(0);", SUBSET), "Cube dimensions must be positive");
    assert_eq!(eval_err("sphere(0);", SUBSET), "Sphere radius must be positive");
    assert_eq!(eval_err("cylinder(0, 1);", SUBSET), "Cylinder height must be positive");
    assert_eq!(eval_err("cylinder(1, 0, 0);", SUBSET), "Cylinder radii must be non-negative and not both zero");
    assert_eq!(eval_err("square(-1);", SUBSET), "Square dimensions must be positive");
    assert_eq!(eval_err("circle(d = 0);", SUBSET), "Circle radius must be positive");
    assert_eq!(eval_err("linear_extrude(-1) square(1);", SUBSET), "linear_extrude() height must be positive");
    assert_eq!(eval_err("linear_extrude(1) cube(1);", SUBSET), "linear_extrude() requires 2D children");
    assert_eq!(eval_err("offset(1) cube(1);", SUBSET), "offset() requires 2D children");
    assert_eq!(eval_err("projection() square(1);", SUBSET), "projection() requires 3D children");
    assert_eq!(eval_err("union() { cube(1); square(1); }", SUBSET), "union() cannot mix 2D and 3D children");
    assert_eq!(eval_err("scale(0) cube(1);", SUBSET), "Scale values cannot be zero");
    assert_eq!(eval_err("rotate(45, [0, 0, 0]) cube(1);", SUBSET), "Rotation axis cannot be zero");
    assert_eq!(eval_err("color(\"no-such\") cube(1);", SUBSET), "Unknown color no-such");
    assert_eq!(eval_err("foo();", SUBSET), "Unsupported geometry operation foo()");
    assert_eq!(eval_err("minkowski() { cube(1); }", SUBSET), "Unsupported geometry operation minkowski()");
    assert_eq!(eval_err("echo(\"hi\");", SUBSET), "Unsupported geometry operation echo()");
}

#[test]
fn subset_boolean_and_transform_shape_algebra() {
    assert_eq!(shape_count("union() { cube(1); cube(1); }", SUBSET), 1);
    assert_eq!(shape_count("difference() { cube(1); cube(1); }", SUBSET), 1);
    assert_eq!(shape_count("difference() { cube(1); }", SUBSET), 1);
    assert_eq!(shape_count("difference() {}", SUBSET), 0);
    assert_eq!(shape_count("hull() { cube(1); translate([2,0,0]) cube(1); }", SUBSET), 1);
    assert_eq!(shape_count("linear_extrude(height = 4) difference() { square([2, 3]); translate([0.5, 0.5]) square([1, 1]); }", SUBSET), 1);
    assert_eq!(shape_count("rotate_extrude($fn = 16) translate([2, 0]) square([1, 1]);", SUBSET), 1);
    assert_eq!(shape_count("linear_extrude(height = 1) offset(r = 0.1) projection() cube(1);", SUBSET), 1);
    assert_eq!(shape_count("translate([1,2,3]) { cube(1); cube(1); }", SUBSET), 2);
    assert_eq!(shape_count("group() { cube(1); cube(1); }", SUBSET), 2);
    assert_eq!(shape_count("render() { cube(1); cube(1); }", SUBSET), 2);
    let two_d = eval_ok("square(1);", SUBSET);
    assert!(two_d.shapes.is_empty());
    assert!(two_d.warnings.iter().any(|w| w.contains("top-level 2D object")));
}

#[test]
fn subset_assert_statement_semantics() {
    assert_eq!(shape_count("assert(true) cube(1);", SUBSET), 1);
    assert_eq!(eval_err("assert(false) cube(1);", SUBSET), "Assertion 'false' failed");
    assert_eq!(
        eval_err("assert(1 > 2, \"boom\") cube(1);", SUBSET),
        "Assertion '1 > 2' failed: boom"
    );
    assert_eq!(eval_err("assert() cube(1);", SUBSET), "assert() requires a condition");
    assert_eq!(
        eval_err("assert(true, extra = 1) cube(1);", SUBSET),
        "assert() does not accept argument extra"
    );
    assert_eq!(
        eval_err("assert(true, condition = true) cube(1);", SUBSET),
        "assert() condition was provided more than once"
    );
    // Expression-form constructs are outside the frozen subset.
    assert_eq!(
        eval_err("x = assert(true); cube(1);", SUBSET),
        "Expression-form assert() is not supported; use statement assert()"
    );
    // The subset parser itself rejects the stable expression forms.
    assert_eq!(eval_err("x = let(a = 1) a; cube(1);", SUBSET), "Expected ; after assignment");
    assert_eq!(eval_err("x = echo(1) 2; cube(1);", SUBSET), "Expected ; after assignment");
    assert_eq!(eval_err("x = [for (i = [0:2]) i]; cube(1);", SUBSET), "Expected ]");
}

#[test]
fn subset_builtins_and_errors() {
    assert_eq!(shape_count("cube(abs(-2));", SUBSET), 1);
    assert_eq!(eval_err("cube(abs(\"a\"));", SUBSET), "abs() argument 1 must be a number");
    assert_eq!(eval_err("cube(abs(1, 2));", SUBSET), "abs() expects 1 argument");
    assert_eq!(
        eval_err("cube(min(1, b = 2));", SUBSET),
        "min() does not accept named arguments in this engine revision"
    );
    assert_eq!(eval_err("cube(nosuchfn(1));", SUBSET), "Unsupported function nosuchfn()");
    assert_eq!(
        eval_err("cube(assert(true));", SUBSET),
        "Expression-form assert() is not supported; use statement assert()"
    );
    assert_eq!(shape_count("cube(str(1, \"x\") == \"1x\" ? 1 : 2);", SUBSET), 1);
}

#[test]
fn subset_budget_limits() {
    big_stack(|| {
        // Nested empty loops burn the evaluation-step budget.
        let source = "for (i = [0:999]) for (j = [0:999]) for (k = [0:9]) {}".to_string();
        let message = eval_err(&source, SUBSET);
        // Repeatedly materialized loop ranges also draw from the value budget;
        // either limit may fire first for this shape of program.
        assert!(
            message == format!("Model exceeds the {} evaluation step limit", crate::value::locale(MAX_EVAL_OPS as usize))
                || message.contains("value-allocation budget"),
            "{message}"
        );
        // Ranges are capped.
        assert_eq!(eval_err("cube([0:100000]);", SUBSET), "Range exceeds 10,000 items");
        // Growing a vector exponentially hits the value-allocation budget.
        let mut doubling = String::from("a = [0];");
        for _ in 0..30 {
            doubling.push_str("a = concat(a, a);");
        }
        doubling.push_str("cube(1);");
        let message = eval_err(&doubling, SUBSET);
        assert!(message.contains("value-allocation budget") || message.contains("exceeds"), "{message}");
    });
}

// ------------------------------------------------------- stable evaluator ---

#[test]
fn stable_lazy_scope_semantics() {
    // The last assignment supplies the value everywhere in the scope.
    assert_eq!(shape_count("x = 1; cube(x); x = 2;", STABLE), 1);
    // An unknown variable warns and yields undef instead of failing.
    let evaluation = eval_ok("cube(unknown_name);", STABLE);
    assert!(evaluation.warnings.contains(&"Ignoring unknown variable 'unknown_name'".to_string()));
    // A forward reference sees no later binding; the reference is unknown.
    let evaluation = eval_ok("a = b; b = 1; cube(is_undef(a) ? 1 : 2);", STABLE);
    assert_eq!(evaluation.shapes.len(), 1);
}

#[test]
fn stable_dynamic_special_variables() {
    // $fn assigned inside a module does not leak back to the caller scope.
    let evaluation = eval_ok(
        "module m() { $fn = 32; sphere(1); } m(); sphere(1);",
        STABLE,
    );
    // The stable profile unions all top-level geometry into one shape.
    assert_eq!(evaluation.shapes.len(), 1);
    // $t and the viewport variables exist with their runtime defaults.
    assert_eq!(shape_count("cube($t);", STABLE), 1);
    assert_eq!(shape_count("cube($vpd / 140);", STABLE), 1);
    assert_eq!(shape_count("cube($vpr[0] / 55);", STABLE), 1);
    assert_eq!(shape_count("cube($preview ? 1 : 1);", STABLE), 1);
}

#[test]
fn stable_functions_and_recursion() {
    assert_eq!(shape_count("function f(x) = x * 2; cube(f(1));", STABLE), 1);
    assert_eq!(
        shape_count("function fib(n) = n <= 1 ? n : fib(n - 1) + fib(n - 2); cube(fib(10) == 55 ? 1 : 2);", STABLE),
        1
    );
    // Closures capture their definition environment.
    assert_eq!(
        shape_count("a = 2; f = function(x) x + a; cube(f(1) == 3 ? 1 : 2);", STABLE),
        1
    );
    // Named, positional and default binding per 2021.01.
    assert_eq!(shape_count("function g(a, b = 10) = a + b; cube(g(b = 1, a = 2) == 3 ? 1 : 0);", STABLE), 1);
    let evaluation = eval_ok("function h(a) = a; cube(h(1, 2));", STABLE);
    assert!(evaluation.warnings.contains(&"Ignoring excess positional argument".to_string()));
    let evaluation = eval_ok("function h(a) = a; cube(h(1, bogus = 2));", STABLE);
    assert!(evaluation.warnings.contains(&"Ignoring unknown argument bogus".to_string()));
    // Functions shadowed by variables and first-class calls.
    assert_eq!(shape_count("sq = function(x) x * x; cube(sq(3) == 9 ? 1 : 0);", STABLE), 1);
    // Recursion depth is bounded.
    let message = big_stack(|| eval_err("function f(n) = f(n + 1); cube(f(0));", STABLE));
    assert!(message.contains("128 nested function calls"), "{message}");
}

#[test]
fn stable_list_comprehensions() {
    assert_eq!(shape_count("v = [for (i = [0:2]) i * 2]; cube(v[2] == 4 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("v = [for (i = [0:2]) if (i % 2 == 0) i]; cube(len(v) == 2 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("v = [for (i = [0:2]) let(j = i * 10) j]; cube(v[1] == 10 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("v = [each [[1, 2], [3]]]; cube(len(v) == 3 ? 1 : 0);", STABLE), 1);
    // C-style for.
    assert_eq!(shape_count("v = [for (i = 0; i < 3; i = i + 1) i * 2]; cube(v[2] == 4 ? 1 : 0);", STABLE), 1);
    // Nested comprehension flattening inside vector literals.
    assert_eq!(shape_count("v = [0, [for (i = [1:2]) i], 3]; cube(len(v) == 4 ? 1 : 0);", STABLE), 1);
}

#[test]
fn stable_ranges_and_undef_semantics() {
    let evaluation = eval_ok("r = [1:0]; cube(is_undef(r) ? 1 : 0);", STABLE);
    assert!(evaluation.warnings.contains(&"begin is greater than the end, but step is positive".to_string()));
    assert_eq!(evaluation.shapes.len(), 1);
    let evaluation = eval_ok("cube([0:\"a\":2]);", STABLE);
    assert!(evaluation.warnings.contains(&"Invalid range bounds produce undef".to_string()));
    // undef arithmetic: undefined operations warn and yield undef.
    let evaluation = eval_ok("cube(undef + 1);", STABLE);
    assert!(evaluation.warnings.iter().any(|w| w.starts_with("Undefined operation (undefined + number)")));
    // Vector arithmetic is elementwise with min length.
    assert_eq!(shape_count("v = [1,2,3] + [10,20]; cube(v[1] == 22 ? 1 : 0);", STABLE), 1);
    // String and vector indexing.
    assert_eq!(shape_count("cube(\"hello\"[1] == \"e\" ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube([3, 4, 5].y == 4 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("r = [10:2:20]; cube(r[2] == 20 ? 1 : 0);", STABLE), 1);
}

#[test]
fn stable_echo_and_assert() {
    let evaluation = eval_ok("echo(\"hello\"); cube(1);", STABLE);
    assert!(evaluation.warnings.contains(&"ECHO: \"hello\"".to_string()));
    let evaluation = eval_ok("echo(a = 1, 2); cube(1);", STABLE);
    assert!(evaluation.warnings.contains(&"ECHO: a = 1, 2".to_string()));
    // Stable assignments are lazy: the echo fires when the variable is read.
    let evaluation = eval_ok("x = echo(40 + 2) 0; cube(x + 1);", STABLE);
    assert!(evaluation.warnings.contains(&"ECHO: 42".to_string()));
    assert_eq!(eval_err("assert(false) cube(1);", STABLE), "Assertion 'false' failed");
    assert_eq!(
        eval_err("assert(false, \"why\") cube(1);", STABLE),
        "Assertion 'false' failed: \"why\""
    );
    // Expression assert returns its body when the condition holds.
    assert_eq!(shape_count("cube(assert(true) 1);", STABLE), 1);
    assert_eq!(eval_err("cube(assert(1 == 2) 1);", STABLE), "Assertion '1 == 2' failed");
}

#[test]
fn stable_modules_children_and_stack() {
    assert_eq!(shape_count("module peg(x) { translate([x, 0, 0]) children(); } for (i = [0:2]) peg(i) cube(1);", STABLE), 1);
    // children() selection by index, vector and range.
    assert_eq!(shape_count("module m() { children(0); } m() { cube(1); cube(2); }", STABLE), 1);
    assert_eq!(shape_count("module m() { children([0, 1]); } m() { cube(1); cube(2); }", STABLE), 1);
    // $children is visible inside the module.
    assert_eq!(shape_count("module m() { if ($children == 2) cube(1); } m() { cube(1); cube(2); }", STABLE), 1);
    // parent_module walks the module stack.
    let evaluation = eval_ok(
        "module inner() { echo(parent_module(1)); } module outer() { inner(); } outer(); cube(1);",
        STABLE,
    );
    assert!(evaluation.warnings.contains(&"ECHO: \"outer\"".to_string()));
    // Lexical scope: module bodies see their definition scope, not the caller's.
    assert_eq!(
        shape_count("x = 1; module m() { cube(x); } module caller() { x = 99; m(); } caller();", STABLE),
        1
    );
}

#[test]
fn stable_viewport_modifiers() {
    // Disable suppresses the complete call subtree.
    assert_eq!(shape_count("*cube(1); sphere(1);", STABLE), 1);
    // Background omits geometry from the result.
    assert_eq!(shape_count("%cube(1); sphere(1);", STABLE), 1);
    // Root selects only the marked subtree.
    assert_eq!(shape_count("cube(1); !sphere(1); cube(2);", STABLE), 1);
}

#[test]
fn stable_shape_algebra_merges_like_the_ts_evaluator() {
    assert_eq!(shape_count("cube(1); cube(1);", STABLE), 1); // top-level union
    assert_eq!(shape_count("translate([1,0,0]) { cube(1); cube(1); }", STABLE), 1);
    assert_eq!(shape_count("group() { cube(1); cube(1); }", STABLE), 1);
    assert_eq!(shape_count("for (i = [0:2]) cube(1);", STABLE), 1);
    assert_eq!(shape_count("minkowski() { cube(1); cube(1); }", STABLE), 1);
    assert_eq!(eval_err("resize([1,1,1]) { cube(1); square(1); }", STABLE), "resize() cannot mix 2D and 3D children");
    assert_eq!(eval_err("minkowski() { cube(1); square(1); }", STABLE), "minkowski() cannot mix 2D and 3D children");
}

#[test]
fn stable_host_dependent_modules_fail_without_project() {
    for (source, message) in [
        ("import(\"part.stl\");", "import() requires an OpenSCAD project so its file is resolved inside the bounded project VFS."),
        ("surface(file = \"height.dat\");", "surface() requires an OpenSCAD project so its file is resolved inside the bounded project VFS."),
        ("text(\"CAD\");", "text() requires an OpenSCAD project so fonts are resolved inside the bounded project VFS."),
    ] {
        assert_eq!(eval_err(source, STABLE), message);
    }
    // dxf query functions degrade to a warning + undef without a project.
    let evaluation = eval_ok("x = dxf_cross(file = \"a.dxf\"); cube(is_undef(x) ? 1 : 0);", STABLE);
    assert!(evaluation.warnings.contains(&"Can't open DXF file 'a.dxf'!".to_string()));
}

// ---------------------------------------------------------------- builtins ---

#[test]
fn builtins_numeric_functions() {
    assert_eq!(shape_count("cube(sin(30) == 0.5 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(cos(180) == -1 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(tan(45) == 1 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(asin(0.5) == 30 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(atan2(1, 1) == 45 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(round(-2.5) == -3 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(round(2.5) == 3 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(pow(2, 10) == 1024 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(log(1000) == 3 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(log(2, 8) == 3 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(ln(exp(2)) == 2 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(sign(-7) == -1 && sign(0) == 0 && sign(7) == 1 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(min([3, 1, 2]) == 1 && max(1, 5, 2) == 5 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(min(\"b\", \"a\") == \"a\" ? 1 : 0);", STABLE), 1);
    // Stable profile degrades built-in value errors to a warning + undef.
    let evaluation = eval_ok("cube(min());", STABLE);
    assert!(evaluation.warnings.contains(&"min() expects at least 1 argument".to_string()));
}

#[test]
fn builtins_string_and_vector_functions() {
    assert_eq!(shape_count("cube(str(\"a\", 1, [2], undef, true) == \"a1[2]undeftrue\" ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(len(\"héllo\") == 5 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(len([1, [2]]) == 2 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(chr(65) == \"A\" ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(chr([65, 66]) == \"AB\" ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(ord(\"A\") == 65 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(is_undef(ord(\"\")) ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(concat([1], 2, [3, 4]) == [1, 2, 3, 4] ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(norm([3, 4]) == 5 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(cross([1, 0, 0], [0, 1, 0]) == [0, 0, 1] ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(cross([1, 0], [0, 1]) == 1 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(version() == [2021, 1, 0] ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(version_num() == 20210100 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(version_num([2019, 5]) == 20190500 ? 1 : 0);", STABLE), 1);
}

#[test]
fn builtins_lookup_and_search() {
    assert_eq!(
        shape_count("cube(lookup(15, [[0, 0], [10, 100], [20, 200]]) == 150 ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(lookup(-5, [[0, 0], [10, 100]]) == 100 ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(shape_count("cube(search(\"ab\", \"abcabc\") == [[0, 3], [1, 4]] ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(search(3, [1, 2, 3, 3]) == [2, 3] ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(search(\"b\", [[\"a\", 1], [\"b\", 2]], 1) == [1] ? 1 : 0);", STABLE), 1);
}

#[test]
fn builtins_predicates_and_is_undef_probe() {
    assert_eq!(shape_count("cube(is_undef(never_declared) ? 1 : 0);", STABLE), 1);
    let evaluation = eval_ok("cube(is_undef(never_declared) ? 1 : 0);", STABLE);
    assert!(!evaluation.warnings.iter().any(|w| w.contains("never_declared")));
    assert_eq!(shape_count("cube(is_list([1]) && !is_list(\"a\") ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(is_num(0/1) && !is_num(\"1\") ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(is_bool(false) && is_string(\"s\") ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(is_function(function(x) x) ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("f = function(x) x; cube(is_function(f) ? 1 : 0);", STABLE), 1);
}

#[test]
fn builtins_seeded_rands_are_repeatable() {
    // Seeded rands uses the pinned mt19937 stream; two calls with the same
    // seed must agree, and values stay in range.
    assert_eq!(
        shape_count("a = rands(0, 1, 5, 42); b = rands(0, 1, 5, 42); cube(a == b ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("v = rands(5, 5, 3, 1); cube(v == [5, 5, 5] ? 1 : 0);", STABLE),
        1
    );
}

#[test]
fn str_formats_function_literals_like_source() {
    assert_eq!(
        shape_count("cube(str(function(x) x + 1) == \"function(x) (x + 1)\" ? 1 : 0);", STABLE),
        1
    );
}
