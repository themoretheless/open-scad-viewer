//! Transactional scene STL/OBJ serialization. No partial file may be committed.
use super::export_prepare;
use crate::{Result, error};
use std::fmt::{self, Write};
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
pub struct Builder {
    binary: bool,
    bytes: Vec<u8>,
    source_triangles: usize,
    triangles: u32,
    parts: usize,
    next_vertex: usize,
    failed: bool,
}
impl Builder {
    pub fn new(binary: bool, name: &str) -> Self {
        let bytes = if binary {
            let mut bytes = vec![0; 84];
            let mut offset = 0;
            for c in "OpenSCAD Viewer binary: ".chars().chain(name.chars()) {
                let mut storage = [0; 4];
                let s = c.encode_utf8(&mut storage);
                if offset + s.len() > 80 {
                    break;
                }
                bytes[offset..offset + s.len()].copy_from_slice(s.as_bytes());
                offset += s.len();
            }
            bytes
        } else {
            b"# Exported by OpenSCAD Viewer\n".to_vec()
        };
        Self {
            binary,
            bytes,
            source_triangles: 0,
            triangles: 0,
            parts: 0,
            next_vertex: 1,
            failed: false,
        }
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn poison(&mut self) {
        self.failed = true;
        self.bytes = Vec::new()
    }
    pub fn append(
        &mut self,
        vertices: &[f64],
        indices: &[u32],
        matrix: &[f64],
        limit: usize,
    ) -> Result<()> {
        if self.failed {
            return Err(error("Export session has failed"));
        }
        let result = self.append_inner(vertices, indices, matrix, limit.min(MAX_BYTES));
        if result.is_err() {
            self.poison()
        }
        result
    }
    fn append_inner(
        &mut self,
        vertices: &[f64],
        indices: &[u32],
        matrix: &[f64],
        limit: usize,
    ) -> Result<()> {
        let count = indices.len() / 3;
        if count > 750_000 - self.source_triangles {
            return Err(math_core::Error::new(
                "MESH_EXPORT_TOO_MANY_TRIANGLES",
                "Export exceeds 750000 triangles",
            ));
        }
        let mesh = export_prepare::prepare(vertices, indices, matrix, self.binary)?;
        if self.binary {
            let extra = mesh.indices.len() / 3 * 50;
            if extra > limit.saturating_sub(self.bytes.len()) || self.bytes.len() > limit {
                return Err(error("Export artifact exceeds byte budget"));
            }
            self.bytes
                .try_reserve(extra)
                .map_err(|_| error("Export artifact allocation failed"))?;
            for (triangle, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
                for &n in &mesh.normals[triangle * 3..triangle * 3 + 3] {
                    self.bytes.extend((n as f32).to_le_bytes())
                }
                for &index in t {
                    for &v in &mesh.positions[index as usize * 3..index as usize * 3 + 3] {
                        self.bytes.extend((v as f32).to_le_bytes())
                    }
                }
                self.bytes.extend([0, 0]);
            }
        } else {
            let mut writer = Bounded {
                bytes: &mut self.bytes,
                limit,
            };
            let result = (|| -> fmt::Result {
                writeln!(writer, "o result_{}", self.parts + 1)?;
                for p in mesh.positions.as_chunks::<3>().0 {
                    writeln!(
                        writer,
                        "v {} {} {}",
                        Number(p[0]),
                        Number(p[1]),
                        Number(p[2])
                    )?;
                }
                for t in mesh.indices.as_chunks::<3>().0 {
                    writeln!(
                        writer,
                        "f {} {} {}",
                        t[0] as usize + self.next_vertex,
                        t[1] as usize + self.next_vertex,
                        t[2] as usize + self.next_vertex
                    )?;
                }
                Ok(())
            })();
            result.map_err(|_| error("Export artifact exceeds byte budget"))?;
        }
        self.source_triangles += count;
        self.triangles += (mesh.indices.len() / 3) as u32;
        self.parts += 1;
        self.next_vertex += mesh.positions.len() / 3;
        Ok(())
    }
    pub fn finish(mut self) -> Result<Vec<u8>> {
        if self.failed {
            return Err(error("Cannot commit a failed export session"));
        }
        if self.binary {
            self.bytes[80..84].copy_from_slice(&self.triangles.to_le_bytes())
        }
        Ok(self.bytes)
    }
}
struct Bounded<'a> {
    bytes: &'a mut Vec<u8>,
    limit: usize,
}
impl Write for Bounded<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.limit.saturating_sub(self.bytes.len()) || self.bytes.len() > self.limit {
            return Err(fmt::Error);
        }
        self.bytes.try_reserve(s.len()).map_err(|_| fmt::Error)?;
        self.bytes.extend_from_slice(s.as_bytes());
        Ok(())
    }
}
// Shortest round-tripping decimal with browser-style exponent thresholds.
pub(crate) struct Number(pub(crate) f64);
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = self.0;
        if x == 0. {
            return f.write_str("0");
        }
        if x.abs() < 1e-6 || x.abs() >= 1e21 {
            let text = format!("{x:e}");
            let (mantissa, exponent) = text.split_once('e').ok_or(fmt::Error)?;
            write!(
                f,
                "{mantissa}e{}{exponent}",
                if exponent.starts_with('-') { "" } else { "+" }
            )
        } else {
            write!(f, "{x}")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const ID: [f64; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    const V: [f64; 18] = [
        0., 0., 0., 0., 0., 1., 1., 0., 0., 0., 0., 1., 0., 1., 0., 0., 0., 1.,
    ];
    #[test]
    fn binary_count_and_unicode_header() {
        let mut b = Builder::new(true, &"я".repeat(80));
        b.append(&V, &[0, 1, 2], &ID, MAX_BYTES).unwrap();
        b.append(&V, &[0, 1, 2], &ID, MAX_BYTES).unwrap();
        let bytes = b.finish().unwrap();
        assert_eq!(bytes.len(), 184);
        assert_eq!(&bytes[80..84], &2u32.to_le_bytes());
        assert!(std::str::from_utf8(&bytes[..80]).is_ok());
    }
    #[test]
    fn obj_offsets_and_number_notation() {
        let mut b = Builder::new(false, "");
        for _ in 0..2 {
            b.append(&V, &[0, 1, 2], &ID, MAX_BYTES).unwrap()
        }
        let text = String::from_utf8(b.finish().unwrap()).unwrap();
        assert!(text.contains("f 4 5 6\n"));
        assert_eq!(Number(-0.).to_string(), "0");
        assert_eq!(Number(1e21).to_string(), "1e+21");
        assert_eq!(Number(1e-7).to_string(), "1e-7");
    }
    #[test]
    fn refusal_poison_prevents_partial_commit() {
        let mut b = Builder::new(false, "");
        b.append(&V, &[0, 1, 2], &ID, MAX_BYTES).unwrap();
        assert!(b.append(&V, &[0, 1, 99], &ID, MAX_BYTES).is_err());
        assert_eq!(b.len(), 0);
        assert!(b.finish().is_err());
        let mut b = Builder::new(true, "");
        assert!(b.append(&V, &[0, 1, 2], &ID, 84).is_err());
        assert!(b.finish().is_err());
    }
}
