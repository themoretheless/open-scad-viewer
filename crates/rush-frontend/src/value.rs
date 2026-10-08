//! Move JSON subtrees into their parents instead of serializing/cloning them.
use value_codec::Value;
pub trait IntoJson {
    fn into_json(self) -> Value;
}
impl IntoJson for Value {
    fn into_json(self) -> Value {
        self
    }
}
impl IntoJson for &Value {
    fn into_json(self) -> Value {
        self.clone()
    }
}
impl IntoJson for String {
    fn into_json(self) -> Value {
        Value::String(self)
    }
}
impl IntoJson for &String {
    fn into_json(self) -> Value {
        Value::String(self.clone())
    }
}
impl IntoJson for &str {
    fn into_json(self) -> Value {
        Value::String(self.into())
    }
}
macro_rules! numeric { ($($t:ty),*) => {$ (impl IntoJson for $t {fn into_json(self)->Value {Value::from(self)}})*}; }
numeric!(usize, u32, u64, i32, i64, f64, bool);
impl<T: IntoJson> IntoJson for Vec<T> {
    fn into_json(self) -> Value {
        Value::Array(self.into_iter().map(IntoJson::into_json).collect())
    }
}
impl<T: Clone + IntoJson> IntoJson for &[T] {
    fn into_json(self) -> Value {
        self.to_vec().into_json()
    }
}
impl<T: Clone + IntoJson> IntoJson for &Vec<T> {
    fn into_json(self) -> Value {
        self.as_slice().into_json()
    }
}
impl<T: IntoJson> IntoJson for Option<T> {
    fn into_json(self) -> Value {
        self.map_or(Value::Null, IntoJson::into_json)
    }
}
macro_rules! json {
 ([]) => {value_codec::Value::Array(Vec::new())};
 ({$($fields:tt)*}) => {{ let mut object=value_codec::Map::new();json!(@fields object; $($fields)*);value_codec::Value::Object(object) }};
 ([$($v:expr),* $(,)?]) => {value_codec::Value::Array(vec![$(json!($v)),*])};
 ([$($items:tt)*]) => {{ let mut items=Vec::<value_codec::Value>::new();json!(@items items; $($items)*);value_codec::Value::Array(items) }};
 (@fields $o:ident;) => {};
 (@fields $o:ident; $k:literal : {$($v:tt)*} $(, $($tail:tt)*)?) => {$o.insert($k.into(),json!({$($v)*})); $(json!(@fields $o; $($tail)*);)?};
 (@fields $o:ident; $k:literal : [$($v:tt)*] $(, $($tail:tt)*)?) => {$o.insert($k.into(),json!([$($v)*])); $(json!(@fields $o; $($tail)*);)?};
 (@fields $o:ident; $k:literal : $v:expr $(, $($tail:tt)*)?) => {$o.insert($k.into(),json!($v)); $(json!(@fields $o; $($tail)*);)?};
 (@items $o:ident;) => {};
 (@items $o:ident; {$($v:tt)*} $(, $($tail:tt)*)?) => {$o.push(json!({$($v)*})); $(json!(@items $o; $($tail)*);)?};
 (@items $o:ident; [$($v:tt)*] $(, $($tail:tt)*)?) => {$o.push(json!([$($v)*])); $(json!(@items $o; $($tail)*);)?};
 (@items $o:ident; $v:expr $(, $($tail:tt)*)?) => {$o.push(json!($v)); $(json!(@items $o; $($tail)*);)?};
 ($v:expr) => {crate::value::IntoJson::into_json($v)};
}
pub(crate) use json;
