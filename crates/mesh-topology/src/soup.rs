//! Prepare float32 triangle soup for display without trusting file normals.
use crate::{Error, Result};
pub struct SoupRender {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub discarded: usize,
}
pub fn render(positions: &[f32]) -> Result<SoupRender> {
    if positions.is_empty() || !positions.len().is_multiple_of(9) || positions.len() / 9 > 250_000 {
        return Err(Error::new(
            "STL_INVALID_SIZE",
            "Decoded STL triangle storage is inconsistent",
        ));
    }
    if positions.iter().any(|x| !x.is_finite()) {
        return Err(Error::new(
            "STL_NON_FINITE_COORDINATE",
            "Decoded STL contains a non-finite coordinate",
        ));
    }
    let mut out = SoupRender {
        vertices: Vec::new(),
        indices: Vec::new(),
        face_ids: Vec::new(),
        discarded: 0,
    };
    for t in positions.as_chunks::<9>().0 {
        let ab = [
            t[3] as f64 - t[0] as f64,
            t[4] as f64 - t[1] as f64,
            t[5] as f64 - t[2] as f64,
        ];
        let ac = [
            t[6] as f64 - t[0] as f64,
            t[7] as f64 - t[1] as f64,
            t[8] as f64 - t[2] as f64,
        ];
        let n = math_core::cross(ab, ac);
        let length = n[0].hypot(n[1]).hypot(n[2]);
        if length <= 0. || !length.is_finite() {
            out.discarded += 1;
            continue;
        }
        let id = out.face_ids.len() as u32;
        out.face_ids.push(id);
        for p in t.as_chunks::<3>().0 {
            out.indices.push((out.vertices.len() / 6) as u32);
            out.vertices.extend(*p);
            out.vertices.extend(n.map(|x| (x / length) as f32));
        }
    }
    if out.indices.is_empty() {
        return Err(Error::new(
            "STL_EMPTY_MESH",
            "STL contains no non-degenerate triangles",
        ));
    }
    Ok(out)
}
