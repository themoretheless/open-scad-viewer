//! Repository-owned document values and bounded JSON/binary codecs.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
use std::{
    collections::BTreeMap,
    fmt,
    ops::{Index, IndexMut},
};
pub type Map<K, V> = BTreeMap<K, V>;
#[derive(Debug, Clone)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(s: impl Into<String>) -> Error {
    Error(s.into())
}
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}
impl Number {
    pub fn from_f64(v: f64) -> Option<Self> {
        v.is_finite().then_some(Self::Float(v))
    }
    pub fn as_f64(&self) -> Option<f64> {
        Some(match self {
            Self::Unsigned(v) => *v as f64,
            Self::Signed(v) => *v as f64,
            Self::Float(v) => *v,
        })
    }
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Unsigned(v) => Some(*v),
            Self::Signed(v) => u64::try_from(*v).ok(),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Signed(v) => Some(*v),
            Self::Unsigned(v) => i64::try_from(*v).ok(),
            _ => None,
        }
    }
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsigned(v) => write!(f, "{v}"),
            Self::Signed(v) => write!(f, "{v}"),
            Self::Float(v) => {
                let s = format!("{v:?}");
                if let Some((a, b)) = s.split_once('e')
                    && !b.starts_with('-')
                {
                    return write!(f, "{a}e+{b}");
                }
                f.write_str(&s)
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}
impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }
    pub fn is_f64(&self) -> bool {
        matches!(self, Self::Number(Number::Float(_)))
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Number(v) = self {
            v.as_f64()
        } else {
            None
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        if let Self::Number(v) = self {
            v.as_u64()
        } else {
            None
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            v.as_i64()
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object_mut(&mut self) -> Option<&mut Map<String, Value>> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn get<I: ValueIndex>(&self, i: I) -> Option<&Value> {
        i.get(self)
    }
    pub fn get_mut<I: ValueIndex>(&mut self, i: I) -> Option<&mut Value> {
        i.get_mut(self)
    }
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
    pub fn pointer(&self, path: &str) -> Option<&Value> {
        if path.is_empty() {
            return Some(self);
        }
        if !path.starts_with('/') {
            return None;
        }
        let mut v = self;
        for k in path[1..].split('/') {
            let k = k.replace("~1", "/").replace("~0", "~");
            v = if v.is_array() {
                v.get(k.parse::<usize>().ok()?)?
            } else {
                v.get(k.as_str())?
            };
        }
        Some(v)
    }
}
pub trait ValueIndex {
    fn get(self, v: &Value) -> Option<&Value>;
    fn get_mut(self, v: &mut Value) -> Option<&mut Value>;
    fn insert(self, v: &mut Value) -> &mut Value;
}
impl ValueIndex for &str {
    fn get(self, v: &Value) -> Option<&Value> {
        v.as_object()?.get(self)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        v.as_object_mut()?.get_mut(self)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        if v.is_null() {
            *v = Value::Object(Map::new())
        }
        v.as_object_mut()
            .expect("object index")
            .entry(self.into())
            .or_default()
    }
}
impl ValueIndex for &String {
    fn get(self, v: &Value) -> Option<&Value> {
        ValueIndex::get(self.as_str(), v)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        ValueIndex::get_mut(self.as_str(), v)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        ValueIndex::insert(self.as_str(), v)
    }
}
impl ValueIndex for usize {
    fn get(self, v: &Value) -> Option<&Value> {
        v.as_array()?.get(self)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        v.as_array_mut()?.get_mut(self)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        &mut v.as_array_mut().expect("array index")[self]
    }
}
static NULL: Value = Value::Null;
impl<I: ValueIndex> Index<I> for Value {
    type Output = Value;
    fn index(&self, i: I) -> &Value {
        i.get(self).unwrap_or(&NULL)
    }
}
impl<I: ValueIndex> IndexMut<I> for Value {
    fn index_mut(&mut self, i: I) -> &mut Value {
        i.insert(self)
    }
}
pub trait Serialize {
    fn to_value(&self) -> Value;
}
pub trait Deserialize<'a>: Sized {
    fn from_value(v: Value) -> Result<Self>;
}
pub fn to_value<T: Serialize>(v: T) -> Result<Value> {
    Ok(v.to_value())
}
pub fn from_value<T: for<'a> Deserialize<'a>>(v: Value) -> Result<T> {
    T::from_value(v)
}
impl Serialize for Value {
    fn to_value(&self) -> Value {
        self.clone()
    }
}
impl<'a> Deserialize<'a> for Value {
    fn from_value(v: Value) -> Result<Self> {
        Ok(v)
    }
}
impl<T: Serialize + ?Sized> Serialize for &T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}
impl Serialize for str {
    fn to_value(&self) -> Value {
        Value::String(self.into())
    }
}
impl Serialize for String {
    fn to_value(&self) -> Value {
        self.as_str().to_value()
    }
}
impl<'a> Deserialize<'a> for String {
    fn from_value(v: Value) -> Result<Self> {
        if let Value::String(s) = v {
            Ok(s)
        } else {
            Err(error("Expected string"))
        }
    }
}
impl Serialize for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}
impl<'a> Deserialize<'a> for bool {
    fn from_value(v: Value) -> Result<Self> {
        v.as_bool().ok_or_else(|| error("Expected boolean"))
    }
}
macro_rules! ints{($($t:ty),*)=>{$(impl Serialize for $t{fn to_value(&self)->Value{if *self as i128>=0{Value::Number(Number::Unsigned(*self as u64))}else{Value::Number(Number::Signed(*self as i64))}}}impl From<$t> for Value{fn from(v:$t)->Self{v.to_value()}}impl From<$t> for Number{fn from(v:$t)->Self{match v.to_value(){Value::Number(n)=>n,_=>unreachable!()}}}impl<'a> Deserialize<'a> for $t{fn from_value(v:Value)->Result<Self>{let n=match v{Value::Number(Number::Unsigned(n))=>n as i128,Value::Number(Number::Signed(n))=>n as i128,_=>return Err(error("Expected integer"))};Self::try_from(n).map_err(|_|error("Integer out of range"))}}impl PartialEq<$t> for Value{fn eq(&self,v:&$t)->bool{self.as_f64()==Some(*v as f64)}})*}}
ints!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
macro_rules! floats{($($t:ty),*)=>{$(impl Serialize for $t{fn to_value(&self)->Value{Number::from_f64(*self as f64).map(Value::Number).unwrap_or(Value::Null)}}impl From<$t> for Value{fn from(v:$t)->Self{v.to_value()}}impl<'a> Deserialize<'a> for $t{fn from_value(v:Value)->Result<Self>{v.as_f64().map(|n|n as $t).ok_or_else(||error("Expected number"))}}impl PartialEq<$t> for Value{fn eq(&self,v:&$t)->bool{self.as_f64()==Some(*v as f64)}})*}}
floats!(f32, f64);
impl PartialEq<bool> for Value {
    fn eq(&self, v: &bool) -> bool {
        self.as_bool() == Some(*v)
    }
}
impl PartialEq<str> for Value {
    fn eq(&self, v: &str) -> bool {
        self.as_str() == Some(v)
    }
}
impl PartialEq<&str> for Value {
    fn eq(&self, v: &&str) -> bool {
        self.as_str() == Some(*v)
    }
}
impl PartialEq<String> for Value {
    fn eq(&self, v: &String) -> bool {
        self.as_str() == Some(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}
impl<T: Serialize> Serialize for [T] {
    fn to_value(&self) -> Value {
        Value::Array(self.iter().map(Serialize::to_value).collect())
    }
}
impl<T: Serialize> Serialize for Vec<T> {
    fn to_value(&self) -> Value {
        self.as_slice().to_value()
    }
}
impl<T: Serialize, const N: usize> Serialize for [T; N] {
    fn to_value(&self) -> Value {
        self.as_slice().to_value()
    }
}
impl<'a, T: Deserialize<'a>> Deserialize<'a> for Vec<T> {
    fn from_value(v: Value) -> Result<Self> {
        if let Value::Array(v) = v {
            v.into_iter().map(T::from_value).collect()
        } else {
            Err(error("Expected array"))
        }
    }
}
impl<'a, T: Deserialize<'a>, const N: usize> Deserialize<'a> for [T; N] {
    fn from_value(v: Value) -> Result<Self> {
        Vec::<T>::from_value(v)?
            .try_into()
            .map_err(|_| error(format!("Expected array length {N}")))
    }
}
impl<T: Serialize> Serialize for Option<T> {
    fn to_value(&self) -> Value {
        self.as_ref().map_or(Value::Null, Serialize::to_value)
    }
}
impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<T> {
    fn from_value(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(None)
        } else {
            T::from_value(v).map(Some)
        }
    }
}
impl<T: Serialize + ?Sized> Serialize for Box<T> {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}
impl<'a, T: Deserialize<'a>> Deserialize<'a> for Box<T> {
    fn from_value(v: Value) -> Result<Self> {
        T::from_value(v).map(Box::new)
    }
}
impl<T: Serialize> Serialize for Map<String, T> {
    fn to_value(&self) -> Value {
        Value::Object(
            self.iter()
                .map(|(k, v)| (k.clone(), v.to_value()))
                .collect(),
        )
    }
}
impl<'a, T: Deserialize<'a>> Deserialize<'a> for Map<String, T> {
    fn from_value(v: Value) -> Result<Self> {
        if let Value::Object(v) = v {
            v.into_iter()
                .map(|(k, v)| T::from_value(v).map(|v| (k, v)))
                .collect()
        } else {
            Err(error("Expected object"))
        }
    }
}
impl<T: Serialize> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        v.to_value()
    }
}
impl<A: Serialize, B: Serialize> Serialize for (A, B) {
    fn to_value(&self) -> Value {
        Value::Array(vec![self.0.to_value(), self.1.to_value()])
    }
}
pub fn to_string<T: Serialize + ?Sized>(v: &T) -> Result<String> {
    Ok(v.to_value().to_string())
}
pub fn to_string_pretty<T: Serialize + ?Sized>(v: &T) -> Result<String> {
    to_string(v)
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Number(v) => write!(f, "{v}"),
            Self::String(s) => {
                f.write_str("\"")?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\""),
                        '\\' => f.write_str("\\\\"),
                        '\n' => f.write_str("\\n"),
                        '\r' => f.write_str("\\r"),
                        '\t' => f.write_str("\\t"),
                        '\u{8}' => f.write_str("\\b"),
                        '\u{c}' => f.write_str("\\f"),
                        c if c < ' ' => write!(f, "\\u{:04x}", c as u32),
                        c => write!(f, "{c}"),
                    }?
                }
                f.write_str("\"")
            }
            Self::Array(v) => {
                f.write_str("[")?;
                for (i, v) in v.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?
                    }
                    write!(f, "{v}")?
                }
                f.write_str("]")
            }
            Self::Object(v) => {
                f.write_str("{")?;
                for (i, (k, v)) in v.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?
                    }
                    write!(f, "{}:{v}", Self::String(k.clone()))?
                }
                f.write_str("}")
            }
        }
    }
}
pub fn from_str<T: for<'a> Deserialize<'a>>(s: &str) -> Result<T> {
    parse_json(s, false)
}
/// JSON decoding that rejects duplicate object keys, including escaped aliases.
pub fn from_str_strict<T: for<'a> Deserialize<'a>>(s: &str) -> Result<T> {
    parse_json(s, true)
}
fn parse_json<T: for<'a> Deserialize<'a>>(s: &str, reject_duplicates: bool) -> Result<T> {
    if s.len() > 32 * 1024 * 1024 {
        return Err(error("Document exceeds 32 MiB"));
    }
    let mut p = Parser {
        b: s.as_bytes(),
        i: 0,
        n: 0,
        reject_duplicates,
    };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(error("Trailing input"));
    }
    T::from_value(v)
}
struct Parser<'a> {
    reject_duplicates: bool,
    b: &'a [u8],
    i: usize,
    n: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1
        }
    }
    fn byte(&mut self) -> Result<u8> {
        let b = *self.b.get(self.i).ok_or_else(|| error("Unexpected end"))?;
        self.i += 1;
        Ok(b)
    }
    fn string(&mut self) -> Result<String> {
        if self.byte()? != b'"' {
            return Err(error("Expected string"));
        }
        let mut out = String::new();
        let mut start = self.i;
        loop {
            let b = self.byte()?;
            if b == b'"' || b == b'\\' {
                out.push_str(
                    std::str::from_utf8(&self.b[start..self.i - 1])
                        .map_err(|_| error("Invalid UTF-8"))?,
                );
                if b == b'"' {
                    return Ok(out);
                }
                let c = match self.byte()? {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'u' => {
                        let mut n = self.hex()?;
                        if (0xd800..=0xdbff).contains(&n) {
                            if self.byte()? != b'\\' || self.byte()? != b'u' {
                                return Err(error("Expected low surrogate"));
                            }
                            let low = self.hex()?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err(error("Invalid surrogate"));
                            }
                            n = 0x10000 + ((n - 0xd800) << 10) + (low - 0xdc00)
                        }
                        char::from_u32(n).ok_or_else(|| error("Invalid Unicode"))?
                    }
                    _ => return Err(error("Invalid escape")),
                };
                out.push(c);
                start = self.i
            } else if b < 32 {
                return Err(error("Control character in string"));
            }
        }
    }
    fn hex(&mut self) -> Result<u32> {
        let mut n = 0;
        for _ in 0..4 {
            n = n * 16
                + (self.byte()? as char)
                    .to_digit(16)
                    .ok_or_else(|| error("Invalid Unicode escape"))?
        }
        Ok(n)
    }
    fn value(&mut self, d: usize) -> Result<Value> {
        self.n += 1;
        if d > 128 || self.n > 4_000_000 {
            return Err(error("Document nesting or size limit exceeded"));
        }
        self.ws();
        match self.b.get(self.i).copied() {
            Some(b'"') => self.string().map(Value::String),
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Value::Array(v));
                }
                loop {
                    v.push(self.value(d + 1)?);
                    self.ws();
                    match self.byte()? {
                        b']' => break,
                        b',' => {}
                        _ => return Err(error("Expected comma or ]")),
                    }
                }
                Ok(Value::Array(v))
            }
            Some(b'{') => {
                self.i += 1;
                let mut v = Map::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Value::Object(v));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if self.byte()? != b':' {
                        return Err(error("Expected colon"));
                    }
                    if v.insert(k, self.value(d + 1)?).is_some() && self.reject_duplicates {
                        return Err(error("Duplicate JSON key"));
                    }
                    self.ws();
                    match self.byte()? {
                        b'}' => break,
                        b',' => {}
                        _ => return Err(error("Expected comma or }")),
                    }
                }
                Ok(Value::Object(v))
            }
            Some(b'-' | b'0'..=b'9') => {
                let start = self.i;
                if self.b[self.i] == b'-' {
                    self.i += 1
                }
                match self.byte()? {
                    b'0' => {}
                    b'1'..=b'9' => {
                        while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                            self.i += 1
                        }
                    }
                    _ => return Err(error("Invalid number")),
                }
                let mut float = false;
                if self.b.get(self.i) == Some(&b'.') {
                    float = true;
                    self.i += 1;
                    let a = self.i;
                    while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1
                    }
                    if a == self.i {
                        return Err(error("Invalid fraction"));
                    }
                }
                if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
                    float = true;
                    self.i += 1;
                    if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                        self.i += 1
                    }
                    let a = self.i;
                    while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1
                    }
                    if a == self.i {
                        return Err(error("Invalid exponent"));
                    }
                }
                let s = std::str::from_utf8(&self.b[start..self.i]).unwrap();
                if !float {
                    if let Ok(n) = s.parse::<u64>() {
                        return Ok(Value::Number(Number::Unsigned(n)));
                    }
                    if let Ok(n) = s.parse::<i64>() {
                        return Ok(Value::Number(Number::Signed(n)));
                    }
                }
                let n = s.parse::<f64>().map_err(|_| error("Invalid number"))?;
                Number::from_f64(n)
                    .map(Value::Number)
                    .ok_or_else(|| error("Nonfinite number"))
            }
            _ => Err(error("Unexpected token")),
        }
    }
    fn literal(&mut self, s: &[u8]) -> Result<()> {
        if self.b.get(self.i..self.i + s.len()) != Some(s) {
            return Err(error("Invalid literal"));
        }
        self.i += s.len();
        Ok(())
    }
}

#[macro_export]
macro_rules! json {
 (null) => {$crate::Value::Null};
 ([]) => {$crate::Value::Array(Vec::new())};
 ({$($fields:tt)*}) => {{ let mut object=$crate::Map::new();$crate::json!(@fields object; $($fields)*);$crate::Value::Object(object) }};
 ([$($items:tt)*]) => {{ let mut items=Vec::<$crate::Value>::new();$crate::json!(@items items; $($items)*);$crate::Value::Array(items) }};
 (@fields $o:ident; $k:literal : null $(, $($tail:tt)*)?) => {$o.insert($k.into(),$crate::Value::Null); $($crate::json!(@fields $o; $($tail)*);)?};
 (@fields $o:ident;) => {};
 (@fields $o:ident; $k:literal : {$($v:tt)*} $(, $($tail:tt)*)?) => {$o.insert($k.into(),$crate::json!({$($v)*})); $($crate::json!(@fields $o; $($tail)*);)?};
 (@fields $o:ident; $k:literal : [$($v:tt)*] $(, $($tail:tt)*)?) => {$o.insert($k.into(),$crate::json!([$($v)*])); $($crate::json!(@fields $o; $($tail)*);)?};
 (@fields $o:ident; $k:literal : $v:expr $(, $($tail:tt)*)?) => {$o.insert($k.into(),$crate::json!($v)); $($crate::json!(@fields $o; $($tail)*);)?};
 (@items $o:ident;) => {};
 (@items $o:ident; {$($v:tt)*} $(, $($tail:tt)*)?) => {$o.push($crate::json!({$($v)*})); $($crate::json!(@items $o; $($tail)*);)?};
 (@items $o:ident; [$($v:tt)*] $(, $($tail:tt)*)?) => {$o.push($crate::json!([$($v)*])); $($crate::json!(@items $o; $($tail)*);)?};
 (@items $o:ident; $v:expr $(, $($tail:tt)*)?) => {$o.push($crate::json!($v)); $($crate::json!(@items $o; $($tail)*);)?};
 ($v:expr) => {$crate::Serialize::to_value(&$v)};
}
impl ValueIndex for &&str {
    fn get(self, v: &Value) -> Option<&Value> {
        ValueIndex::get(*self, v)
    }
    fn get_mut(self, v: &mut Value) -> Option<&mut Value> {
        ValueIndex::get_mut(*self, v)
    }
    fn insert(self, v: &mut Value) -> &mut Value {
        ValueIndex::insert(*self, v)
    }
}
/// Versioned little-endian value wire format; JSON is only for authored files.
pub fn encode_binary(value: &Value) -> Result<Vec<u8>> {
    let mut out = b"MGV1".to_vec();
    fn put(v: &Value, o: &mut Vec<u8>, d: usize) -> Result<()> {
        if d > 128 {
            return Err(error("Binary nesting limit"));
        }
        match v {
            Value::Null => o.push(0),
            Value::Bool(false) => o.push(1),
            Value::Bool(true) => o.push(2),
            Value::Number(n) => match n {
                Number::Float(v) => {
                    o.push(3);
                    o.extend(v.to_le_bytes())
                }
                Number::Unsigned(v) => {
                    o.push(7);
                    o.extend(v.to_le_bytes())
                }
                Number::Signed(v) => {
                    o.push(8);
                    o.extend(v.to_le_bytes())
                }
            },
            Value::String(s) => {
                o.push(4);
                o.extend((s.len() as u32).to_le_bytes());
                o.extend(s.as_bytes())
            }
            Value::Array(a) => {
                o.push(5);
                o.extend((a.len() as u32).to_le_bytes());
                for v in a {
                    put(v, o, d + 1)?
                }
            }
            Value::Object(a) => {
                o.push(6);
                o.extend((a.len() as u32).to_le_bytes());
                for (k, v) in a {
                    put(&Value::String(k.clone()), o, d + 1)?;
                    put(v, o, d + 1)?
                }
            }
        }
        if o.len() > 32 * 1024 * 1024 {
            return Err(error("Binary size limit"));
        }
        Ok(())
    }
    put(value, &mut out, 0)?;
    Ok(out)
}
pub fn decode_binary(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > 32 * 1024 * 1024 || bytes.get(..4) != Some(b"MGV1") {
        return Err(error("Invalid binary header or size"));
    }
    struct Reader<'a> {
        b: &'a [u8],
        i: usize,
        n: usize,
    }
    impl Reader<'_> {
        fn read(&mut self, n: usize) -> Result<&[u8]> {
            let end = self
                .i
                .checked_add(n)
                .ok_or_else(|| error("Binary length overflow"))?;
            let b = self
                .b
                .get(self.i..end)
                .ok_or_else(|| error("Truncated binary value"))?;
            self.i = end;
            Ok(b)
        }
        fn count(&mut self) -> Result<usize> {
            Ok(u32::from_le_bytes(self.read(4)?.try_into().unwrap()) as usize)
        }
        fn value(&mut self, d: usize) -> Result<Value> {
            self.n += 1;
            if d > 128 || self.n > 4_000_000 {
                return Err(error("Binary nesting or item limit"));
            }
            let tag = self.read(1)?[0];
            Ok(match tag {
                0 => Value::Null,
                1 => Value::Bool(false),
                2 => Value::Bool(true),
                3 => {
                    let n = f64::from_le_bytes(self.read(8)?.try_into().unwrap());
                    Value::Number(
                        Number::from_f64(n).ok_or_else(|| error("Nonfinite binary number"))?,
                    )
                }
                7 => Value::Number(Number::Unsigned(u64::from_le_bytes(
                    self.read(8)?.try_into().unwrap(),
                ))),
                8 => Value::Number(Number::Signed(i64::from_le_bytes(
                    self.read(8)?.try_into().unwrap(),
                ))),
                4 => {
                    let n = self.count()?;
                    Value::String(
                        std::str::from_utf8(self.read(n)?)
                            .map_err(|_| error("Invalid binary UTF-8"))?
                            .into(),
                    )
                }
                5 | 6 => {
                    let n = self.count()?;
                    if n > self.b.len() - self.i || n > 4_000_000 - self.n {
                        return Err(error("Binary item limit"));
                    }
                    if tag == 5 {
                        let mut a = Vec::with_capacity(n);
                        for _ in 0..n {
                            a.push(self.value(d + 1)?)
                        }
                        Value::Array(a)
                    } else {
                        let mut a = Map::new();
                        for _ in 0..n {
                            let k = if let Value::String(k) = self.value(d + 1)? {
                                k
                            } else {
                                return Err(error("Binary object key must be string"));
                            };
                            if a.insert(k, self.value(d + 1)?).is_some() {
                                return Err(error("Duplicate binary key"));
                            }
                        }
                        Value::Object(a)
                    }
                }
                _ => return Err(error("Invalid binary tag")),
            })
        }
    }
    let mut r = Reader {
        b: bytes,
        i: 4,
        n: 0,
    };
    let value = r.value(0)?;
    if r.i != bytes.len() {
        return Err(error("Trailing binary data"));
    }
    Ok(value)
}
impl Serialize for () {
    fn to_value(&self) -> Value {
        Value::Null
    }
}
impl<'a> Deserialize<'a> for () {
    fn from_value(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(())
        } else {
            Err(error("Expected null"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn documents_and_binary_roundtrip() {
        let text =
            r#"{"unicode":"\ud83d\ude00","items":[null,true,false,1,-2,1.25],"object":{"x":2}}"#;
        let value: Value = from_str(text).unwrap();
        assert_eq!(value["unicode"], "😀");
        assert_eq!(from_str::<Value>(&value.to_string()).unwrap(), value);
        let bytes = encode_binary(&value).unwrap();
        assert_eq!(decode_binary(&bytes).unwrap(), value);
        for n in 0..bytes.len() {
            assert!(decode_binary(&bytes[..n]).is_err())
        }
    }
    #[test]
    fn reject_invalid_documents() {
        for text in [
            "[1,]",
            "{\"a\":1,}",
            "01",
            "1.",
            "1e",
            "NaN",
            "\"\\ud800\"",
            "\"\\udfff\"",
            "\"\n\"",
            "null true",
        ] {
            assert!(from_str::<Value>(text).is_err(), "{text}")
        }
        assert!(from_str::<Value>(&format!("{}0{}", "[".repeat(130), "]".repeat(130))).is_err())
    }
    #[test]
    fn bounded_binary_reader() {
        for b in [
            b"MGV1\x05\xff\xff\xff\xff".as_slice(),
            b"MGV1\xff",
            b"MGV1\0\0",
        ] {
            assert!(decode_binary(b).is_err())
        }
        let v = json!({"n":u64::MAX,"negative":i64::MIN,"zero":-0.0});
        let out = decode_binary(&encode_binary(&v).unwrap()).unwrap();
        assert_eq!(out["n"].as_u64(), Some(u64::MAX));
        assert!(out["zero"].as_f64().unwrap().is_sign_negative())
    }
}
