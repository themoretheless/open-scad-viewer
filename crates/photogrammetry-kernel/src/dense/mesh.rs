use super::Surface;
use crate::{math::*, Result};

/// Lossy vertex clustering for bounded text documents. The original surface is retained.
/// It does not certify manifoldness or fill missing regions.
pub fn compact(surface: &Surface, cells: usize) -> Result<Surface> {
    if surface.colors.len() != surface.positions.len()
        || surface.positions.iter().flatten().any(|v| !v.is_finite())
        || surface
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= surface.positions.len())
    {
        return Err("Invalid surface for compaction".into());
    }
    use std::collections::{BTreeMap, BTreeSet};
    if surface.positions.is_empty() {
        return Ok(Surface::default());
    }
    let cells = cells.clamp(4, 64);
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in &surface.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let extent = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max).max(1e-12);
    let step = extent / cells as f64;
    let mut groups = BTreeMap::new();
    let mut sums = Vec::<(V3, [f64; 3], usize)>::new();
    let mut ids = Vec::new();
    for (p, color) in surface.positions.iter().zip(&surface.colors) {
        let key: [i32; 3] = std::array::from_fn(|k| ((p[k] - lo[k]) / step).floor() as i32);
        let id = *groups.entry(key).or_insert_with(|| {
            sums.push(([0.; 3], [0.; 3], 0));
            sums.len() - 1
        });
        let (sum, c, n) = &mut sums[id];
        for k in 0..3 {
            sum[k] += p[k];
            c[k] += color[k] as f64;
        }
        *n += 1;
        ids.push(id as u32);
    }
    let mut result = Surface::default();
    for (p, c, n) in sums {
        result.positions.push(scale(p, 1. / n as f64));
        result.colors.push(c.map(|v| (v / n as f64).round() as u8));
    }
    let mut faces = BTreeSet::new();
    for t in &surface.triangles {
        let t = t.map(|i| ids[i as usize]);
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        let mut key = t;
        key.sort();
        if faces.insert(key) {
            result.triangles.push(t);
        }
    }
    let mut used = BTreeMap::new();
    let mut output = Surface::default();
    for t in &result.triangles {
        let mapped = t.map(|i| {
            *used.entry(i).or_insert_with(|| {
                let id = output.positions.len() as u32;
                output.positions.push(result.positions[i as usize]);
                output.colors.push(result.colors[i as usize]);
                id
            })
        });
        output.triangles.push(mapped);
    }
    Ok(output)
}

#[cfg(test)]
mod compact_tests {
    use super::*;
    #[test]
    fn clusters_a_copy_and_removes_duplicate_faces() {
        let source = Surface {
            positions: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [0.0001, 0.0001, 0.],
            ],
            colors: vec![[128; 3]; 4],
            triangles: vec![[0, 1, 2], [3, 1, 2]],
        };
        let result = compact(&source, 24).unwrap();
        assert_eq!(source.positions.len(), 4);
        assert_eq!(result.positions.len(), 3);
        assert_eq!(result.triangles.len(), 1);
        assert!(result
            .positions
            .iter()
            .any(|p| (p[0] - 0.00005).abs() < 1e-8));
        let bad = Surface {
            positions: vec![[0.; 3]],
            colors: vec![],
            triangles: vec![],
        };
        assert!(compact(&bad, 24).is_err());
    }
}
