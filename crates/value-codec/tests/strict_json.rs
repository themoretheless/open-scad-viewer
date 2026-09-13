use value_codec::{Value, from_str, from_str_strict};
#[test]
fn strict_json_rejects_nested_and_escaped_duplicate_keys_only_when_requested() {
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"outer":{"x":1,"x":2}}"#,
    ] {
        assert!(from_str_strict::<Value>(input).is_err());
        assert!(from_str::<Value>(input).is_ok());
    }
    assert!(from_str_strict::<Value>(r#"{"a":{"x":1},"b":{"x":2}}"#).is_ok());
}
