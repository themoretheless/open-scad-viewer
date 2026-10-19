use super::*;

#[test]
fn guard_steps_keep_statement_order_and_stop_before_later_items() {
    let document = json!({"parameters":[]});
    let expression = json!({"op":"guarded","input":{"op":"list","items":[1,2,3]},"binding":"item","steps":[
        {"kind":"where","value":{"op":"lt","args":[1,{"local":"item"}]}},
        {"kind":"assert","value":{"op":"assert_value","value":1,"constraints":[{"id":"check","left":{"local":"item"},"relation":"le","right":2,"message":"Later item must not execute"}]}},
        {"kind":"while","value":{"op":"lt","args":[{"local":"item"},2]}},
        {"kind":"assert","value":{"op":"divide","args":[1,0]}}
    ]});
    let mut evaluator = Evaluator::new(&document).unwrap();
    let value = evaluator
        .resolve(&expression, &Scope::default(), "/test", 0)
        .unwrap();
    assert!(sequence(&value, "/test").unwrap().is_empty());
    assert_eq!(evaluator.constraint_report.len(), 1);
    assert_eq!(evaluator.constraint_report[0]["actual"], 2.0);
}

#[test]
fn guarded_sequence_returns_original_values_with_lexical_binding() {
    let document = json!({"parameters":[]});
    let expression = json!({"op":"guarded","input":{"op":"list","items":[1,2,3]},"binding":"item","steps":[
        {"kind":"where","value":{"op":"lt","args":[1,{"local":"item"}]}},
        {"kind":"while","value":{"op":"lt","args":[{"local":"item"},3]}}
    ]});
    let mut evaluator = Evaluator::new(&document).unwrap();
    let value = evaluator
        .resolve(&expression, &Scope::default(), "/test", 0)
        .unwrap();
    let values = sequence(&value, "/test").unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(numeric(&values[0], "/test").unwrap(), 2.0);
}
