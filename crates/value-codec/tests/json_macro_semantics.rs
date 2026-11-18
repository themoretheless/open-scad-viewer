use std::cell::RefCell;
use value_codec::{Map, Serialize, Value, json};
struct Probe<'a> {
    trace: &'a RefCell<Vec<&'static str>>,
    label: &'static str,
    value: i32,
}
impl Serialize for Probe<'_> {
    fn to_value(&self) -> Value {
        self.trace.borrow_mut().push(self.label);
        self.value.to_value()
    }
}
impl Drop for Probe<'_> {
    fn drop(&mut self) {
        self.trace.borrow_mut().push("drop");
    }
}
#[test]
fn fields_keep_single_evaluation_order_temporary_drops_and_duplicate_replacement() {
    let trace = RefCell::new(Vec::new());
    let v = json!({"a":Probe{trace:&trace,label:"first",value:1},"a":Probe{trace:&trace,label:"second",value:2},"nested":{"z":Probe{trace:&trace,label:"nested",value:3}},"list":[Probe{trace:&trace,label:"list",value:4},Value::Null]});
    assert_eq!(
        *trace.borrow(),
        vec![
            "first", "drop", "second", "drop", "nested", "drop", "list", "drop"
        ]
    );
    assert_eq!(
        v.to_string(),
        "{\"a\":2,\"list\":[4,null],\"nested\":{\"z\":3}}"
    );
}
#[test]
fn literal_key_types_hygiene_and_low_level_map_rule_remain_compatible() {
    let object = 7;
    let key = 8;
    let value = 9;
    let v = json!({'c':object,"key":key,"value":value});
    assert_eq!(v["c"], 7);
    assert_eq!(v["key"], 8);
    assert_eq!(v["value"], 9);
    let mut object: Map<String, Value> = Map::new();
    json!(@fields object; "before":object.len(),"after":object.len());
    assert_eq!(object["before"], 0usize);
    assert_eq!(object["after"], 1usize);
}
#[test]
fn number_categories_nested_values_and_ordered_text_remain_unchanged() {
    let v = json!({"u":u64::MAX,"i":i64::MIN,"f":f64::from_bits(1),"zero":-0.0,"nested":[{"z":true},[],{}]});
    assert_eq!(v["u"].as_u64(), Some(u64::MAX));
    assert_eq!(v["i"].as_i64(), Some(i64::MIN));
    assert_eq!(v["f"].as_f64().unwrap().to_bits(), 1);
    assert!(v["zero"].as_f64().unwrap().is_sign_negative());
    let binary = value_codec::encode_binary(&v).unwrap();
    assert_eq!(value_codec::decode_binary(&binary).unwrap(), v);
    assert_eq!(value_codec::from_str::<Value>(&v.to_string()).unwrap(), v);
}
