//! Typed source identity encoding. Exact legacy MGV1 bytes, without Value trees
//! or an optional transport dependency. This is not a new certificate protocol.
use super::*;
mod schema;
trait Put {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()>;
}
struct Encoder {
    bytes: Vec<u8>,
}
impl Encoder {
    fn append(&mut self, b: &[u8]) -> Result<()> {
        if self
            .bytes
            .len()
            .checked_add(b.len())
            .is_none_or(|n| n > 32 * 1024 * 1024)
        {
            return Err(fail("Binary size limit"));
        }
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    fn tag(&mut self, d: usize, t: u8) -> Result<()> {
        if d > 128 {
            return Err(fail("Binary nesting limit"));
        }
        self.append(&[t])
    }
    fn length(&mut self, n: usize) -> Result<()> {
        let n = u32::try_from(n).map_err(|_| fail("Binary size limit"))?;
        self.append(&n.to_le_bytes())
    }
    fn object(&mut self, d: usize, fields: &mut [(&str, &dyn Put)]) -> Result<()> {
        self.tag(d, 6)?;
        self.length(fields.len())?;
        fields.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in fields {
            key.put(self, d + 1)?;
            value.put(self, d + 1)?;
        }
        Ok(())
    }
}
fn encode<T: Put + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let mut e = Encoder {
        bytes: b"MGV1".to_vec(),
    };
    value.put(&mut e, 0)?;
    Ok(e.bytes)
}
impl<T: Put + ?Sized> Put for &T {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        T::put(*self, e, d)
    }
}
impl Put for str {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        e.tag(d, 4)?;
        e.length(self.len())?;
        e.append(self.as_bytes())
    }
}
impl Put for String {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.as_str().put(e, d)
    }
}
impl Put for bool {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        e.tag(d, if *self { 2 } else { 1 })
    }
}
impl Put for f64 {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        if self.is_finite() {
            e.tag(d, 3)?;
            e.append(&self.to_le_bytes())
        } else {
            e.tag(d, 0)
        }
    }
}
macro_rules! unsigned {($($t:ty),*)=>{$(impl Put for $t{fn put(&self,e:&mut Encoder,d:usize)->Result<()>{e.tag(d,7)?;e.append(&(*self as u64).to_le_bytes())}})*}}
unsigned!(usize, u32, u64);
impl Put for i32 {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        if *self >= 0 {
            (*self as u64).put(e, d)
        } else {
            e.tag(d, 8)?;
            e.append(&(*self as i64).to_le_bytes())
        }
    }
}
impl<T: Put> Put for [T] {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        e.tag(d, 5)?;
        e.length(self.len())?;
        for x in self {
            x.put(e, d + 1)?
        }
        Ok(())
    }
}
impl<T: Put> Put for Vec<T> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.as_slice().put(e, d)
    }
}
impl<T: Put, const N: usize> Put for [T; N] {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.as_slice().put(e, d)
    }
}
impl<T: Put> Put for Option<T> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        match self {
            Some(x) => x.put(e, d),
            None => e.tag(d, 0),
        }
    }
}
macro_rules! object {($e:expr,$d:expr,$($key:literal=>$value:expr),*$(,)?)=>{{let mut fields:Vec<(&str,&dyn Put)>=vec![$(($key,$value as &dyn Put)),*];$e.object($d,&mut fields)}}}
use object;
pub(super) fn source(s: &Source) -> Result<Vec<u8>> {
    encode(s)
}
pub(super) fn surface(s: &crate::Surface) -> Result<Vec<u8>> {
    encode(s)
}
pub(super) fn wire(s: &[crate::Curve]) -> Result<Vec<u8>> {
    encode(s)
}
#[cfg(all(test, feature = "codec"))]
mod tests;
