use super::*;

#[test]
fn stable_ranges_and_undef_semantics() {
    let evaluation = eval_ok("r = [1:0]; cube(is_undef(r) ? 1 : 0);", STABLE);
    assert!(
        evaluation
            .warnings
            .contains(&"begin is greater than the end, but step is positive".to_string())
    );
    assert_eq!(evaluation.shapes.len(), 1);
    let evaluation = eval_ok("cube([0:\"a\":2]);", STABLE);
    assert!(
        evaluation
            .warnings
            .contains(&"Invalid range bounds produce undef".to_string())
    );
    // undef arithmetic: undefined operations warn and yield undef.
    let evaluation = eval_ok("cube(undef + 1);", STABLE);
    assert!(
        evaluation
            .warnings
            .iter()
            .any(|w| w.starts_with("Undefined operation (undefined + number)"))
    );
    // Vector arithmetic is elementwise with min length.
    assert_eq!(
        shape_count("v = [1,2,3] + [10,20]; cube(v[1] == 22 ? 1 : 0);", STABLE),
        1
    );
    // String and vector indexing.
    assert_eq!(
        shape_count("cube(\"hello\"[1] == \"e\" ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(shape_count("cube([3, 4, 5].y == 4 ? 1 : 0);", STABLE), 1);
    assert_eq!(
        shape_count("r = [10:2:20]; cube(r[2] == 20 ? 1 : 0);", STABLE),
        1
    );
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
    assert_eq!(
        eval_err("assert(false) cube(1);", STABLE),
        "Assertion 'false' failed"
    );
    assert_eq!(
        eval_err("assert(false, \"why\") cube(1);", STABLE),
        "Assertion 'false' failed: \"why\""
    );
    // Expression assert returns its body when the condition holds.
    assert_eq!(shape_count("cube(assert(true) 1);", STABLE), 1);
    assert_eq!(
        eval_err("cube(assert(1 == 2) 1);", STABLE),
        "Assertion '1 == 2' failed"
    );
}

#[test]
fn stable_modules_children_and_stack() {
    assert_eq!(
        shape_count(
            "module peg(x) { translate([x, 0, 0]) children(); } for (i = [0:2]) peg(i) cube(1);",
            STABLE
        ),
        1
    );
    // children() selection by index, vector and range.
    assert_eq!(
        shape_count(
            "module m() { children(0); } m() { cube(1); cube(2); }",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count(
            "module m() { children([0, 1]); } m() { cube(1); cube(2); }",
            STABLE
        ),
        1
    );
    // $children is visible inside the module.
    assert_eq!(
        shape_count(
            "module m() { if ($children == 2) cube(1); } m() { cube(1); cube(2); }",
            STABLE
        ),
        1
    );
    // parent_module walks the module stack.
    let evaluation = eval_ok(
        "module inner() { echo(parent_module(1)); } module outer() { inner(); } outer(); cube(1);",
        STABLE,
    );
    assert!(evaluation.warnings.contains(&"ECHO: \"outer\"".to_string()));
    // Lexical scope: module bodies see their definition scope, not the caller's.
    assert_eq!(
        shape_count(
            "x = 1; module m() { cube(x); } module caller() { x = 99; m(); } caller();",
            STABLE
        ),
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
    assert_eq!(
        shape_count("translate([1,0,0]) { cube(1); cube(1); }", STABLE),
        1
    );
    assert_eq!(shape_count("group() { cube(1); cube(1); }", STABLE), 1);
    assert_eq!(shape_count("for (i = [0:2]) cube(1);", STABLE), 1);
    assert_eq!(shape_count("minkowski() { cube(1); cube(1); }", STABLE), 1);
    assert_eq!(
        eval_err("resize([1,1,1]) { cube(1); square(1); }", STABLE),
        "resize() cannot mix 2D and 3D children"
    );
    assert_eq!(
        eval_err("minkowski() { cube(1); square(1); }", STABLE),
        "minkowski() cannot mix 2D and 3D children"
    );
}

#[test]
fn stable_host_dependent_modules_fail_without_project() {
    for (source, message) in [
        (
            "import(\"part.stl\");",
            "import() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.",
        ),
        (
            "surface(file = \"height.dat\");",
            "surface() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.",
        ),
        (
            "text(\"CAD\");",
            "text() requires an OpenSCAD project so fonts are resolved inside the bounded project VFS.",
        ),
    ] {
        assert_eq!(eval_err(source, STABLE), message);
    }
    // dxf query functions degrade to a warning + undef without a project.
    let evaluation = eval_ok(
        "x = dxf_cross(file = \"a.dxf\"); cube(is_undef(x) ? 1 : 0);",
        STABLE,
    );
    assert!(
        evaluation
            .warnings
            .contains(&"Can't open DXF file 'a.dxf'!".to_string())
    );
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
    assert_eq!(
        shape_count(
            "cube(sign(-7) == -1 && sign(0) == 0 && sign(7) == 1 ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count(
            "cube(min([3, 1, 2]) == 1 && max(1, 5, 2) == 5 ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count("cube(min(\"b\", \"a\") == \"a\" ? 1 : 0);", STABLE),
        1
    );
    // Stable profile degrades built-in value errors to a warning + undef.
    let evaluation = eval_ok("cube(min());", STABLE);
    assert!(
        evaluation
            .warnings
            .contains(&"min() expects at least 1 argument".to_string())
    );
}

#[test]
fn builtins_string_and_vector_functions() {
    assert_eq!(
        shape_count(
            "cube(str(\"a\", 1, [2], undef, true) == \"a1[2]undeftrue\" ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(shape_count("cube(len(\"héllo\") == 5 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(len([1, [2]]) == 2 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(chr(65) == \"A\" ? 1 : 0);", STABLE), 1);
    assert_eq!(
        shape_count("cube(chr([65, 66]) == \"AB\" ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(shape_count("cube(ord(\"A\") == 65 ? 1 : 0);", STABLE), 1);
    assert_eq!(shape_count("cube(is_undef(ord(\"\")) ? 1 : 0);", STABLE), 1);
    assert_eq!(
        shape_count(
            "cube(concat([1], 2, [3, 4]) == [1, 2, 3, 4] ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(shape_count("cube(norm([3, 4]) == 5 ? 1 : 0);", STABLE), 1);
    assert_eq!(
        shape_count(
            "cube(cross([1, 0, 0], [0, 1, 0]) == [0, 0, 1] ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count("cube(cross([1, 0], [0, 1]) == 1 ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(version() == [2021, 1, 0] ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(version_num() == 20210100 ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(version_num([2019, 5]) == 20190500 ? 1 : 0);", STABLE),
        1
    );
}

#[test]
fn builtins_lookup_and_search() {
    assert_eq!(
        shape_count(
            "cube(lookup(15, [[0, 0], [10, 100], [20, 200]]) == 150 ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count(
            "cube(lookup(-5, [[0, 0], [10, 100]]) == 100 ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count(
            "cube(search(\"ab\", \"abcabc\") == [[0, 3], [1, 4]] ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count("cube(search(3, [1, 2, 3, 3]) == [2, 3] ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count(
            "cube(search(\"b\", [[\"a\", 1], [\"b\", 2]], 1) == [1] ? 1 : 0);",
            STABLE
        ),
        1
    );
}

#[test]
fn builtins_predicates_and_is_undef_probe() {
    assert_eq!(
        shape_count("cube(is_undef(never_declared) ? 1 : 0);", STABLE),
        1
    );
    let evaluation = eval_ok("cube(is_undef(never_declared) ? 1 : 0);", STABLE);
    assert!(
        !evaluation
            .warnings
            .iter()
            .any(|w| w.contains("never_declared"))
    );
    assert_eq!(
        shape_count("cube(is_list([1]) && !is_list(\"a\") ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(is_num(0/1) && !is_num(\"1\") ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(is_bool(false) && is_string(\"s\") ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("cube(is_function(function(x) x) ? 1 : 0);", STABLE),
        1
    );
    assert_eq!(
        shape_count("f = function(x) x; cube(is_function(f) ? 1 : 0);", STABLE),
        1
    );
}

#[test]
fn builtins_seeded_rands_are_repeatable() {
    // Seeded rands uses the pinned mt19937 stream; two calls with the same
    // seed must agree, and values stay in range.
    assert_eq!(
        shape_count(
            "a = rands(0, 1, 5, 42); b = rands(0, 1, 5, 42); cube(a == b ? 1 : 0);",
            STABLE
        ),
        1
    );
    assert_eq!(
        shape_count(
            "v = rands(5, 5, 3, 1); cube(v == [5, 5, 5] ? 1 : 0);",
            STABLE
        ),
        1
    );
}

#[test]
fn str_formats_function_literals_like_source() {
    assert_eq!(
        shape_count(
            "cube(str(function(x) x + 1) == \"function(x) (x + 1)\" ? 1 : 0);",
            STABLE
        ),
        1
    );
}
