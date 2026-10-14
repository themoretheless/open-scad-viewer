use super::*;

#[test]
fn memo_reuses_successful_checks_only_within_the_same_scope() {
    let document = json!({"parameters":[]});
    let expression = json!({"op":"memo","id":"m1","value":{"op":"assert_value","value":7,"constraints":[{"id":"c1","left":1,"relation":"eq","right":1,"message":"Once"}]}});
    let mut evaluator = Evaluator::new(&document).unwrap();
    let scope = Scope::default();
    for _ in 0..3 {
        assert_eq!(
            evaluator.evaluate(&expression, &scope, "/memo", 0).unwrap(),
            7.0
        );
    }
    assert_eq!(evaluator.constraint_report.len(), 1);
    evaluator
        .evaluate(&expression, &Scope::default(), "/next_iteration", 0)
        .unwrap();
    assert_eq!(evaluator.constraint_report.len(), 2);
}

#[test]
fn memo_rejects_conflicting_bodies_and_caches_failures() {
    let document = json!({"parameters":[]});
    let expression = json!({"op":"memo","id":"m1","value":{"op":"divide","args":[1,0]}});
    let conflict = json!({"op":"memo","id":"m1","value":3});
    let mut evaluator = Evaluator::new(&document).unwrap();
    let scope = Scope::default();
    let first = evaluator
        .resolve(&expression, &scope, "/first", 0)
        .unwrap_err();
    let repeated = evaluator
        .resolve(&expression, &scope, "/repeated", 0)
        .unwrap_err();
    assert_eq!(first.path, repeated.path);
    assert_eq!(first.code, repeated.code);
    assert_eq!(
        evaluator
            .resolve(&conflict, &scope, "/conflict", 0)
            .unwrap_err()
            .code,
        "memo_id_conflict"
    );
}
