use super::Surface;
use crate::{Result, math::*};

fn validate(surface: &Surface, operation: &str) -> Result<()> {
    if surface.colors.len() != surface.positions.len()
        || surface.positions.iter().flatten().any(|v| !v.is_finite())
        || surface
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= surface.positions.len())
    {
        return Err(crate::error(format!("Invalid surface for {operation}")));
    }
    Ok(())
}

/// Drop connected components with fewer than `min_faces` triangles.
/// Vertex unions and output follow ascending index order, so the result is
/// deterministic; when nothing is below the threshold the input is cloned
/// unchanged. Removing faces also drops vertices left unreferenced.
pub fn filter_small_components(surface: &Surface, min_faces: usize) -> Result<Surface> {
    validate(surface, "component filtering")?;
    if min_faces <= 1 || surface.triangles.is_empty() {
        return Ok(surface.clone());
    }
    let mut parent: Vec<usize> = (0..surface.positions.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for t in &surface.triangles {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let ra = root(&mut parent, a);
        parent[ra] = b;
        let rb = root(&mut parent, b);
        parent[rb] = c;
    }
    let mut faces = vec![0usize; surface.positions.len()];
    for t in &surface.triangles {
        faces[root(&mut parent, t[0] as usize)] += 1;
    }
    let mut removed = false;
    let keep: Vec<bool> = surface
        .triangles
        .iter()
        .map(|t| {
            let keep = faces[root(&mut parent, t[0] as usize)] >= min_faces;
            removed |= !keep;
            keep
        })
        .collect();
    if !removed {
        return Ok(surface.clone());
    }
    let mut output = Surface::default();
    let mut remap = vec![u32::MAX; surface.positions.len()];
    for (t, &keep) in surface.triangles.iter().zip(&keep) {
        if !keep {
            continue;
        }
        output.triangles.push(t.map(|i| {
            let k = i as usize;
            if remap[k] == u32::MAX {
                remap[k] = output.positions.len() as u32;
                output.positions.push(surface.positions[k]);
                output.colors.push(surface.colors[k]);
            }
            remap[k]
        }));
    }
    Ok(output)
}

/// Lossy vertex clustering for bounded text documents. The original surface is retained.
/// It does not certify manifoldness or fill missing regions.
pub fn compact(surface: &Surface, cells: usize) -> Result<Surface> {
    validate(surface, "compaction")?;
    use super::volume::{FastMap, FastSet};
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
    let mut groups = FastMap::default();
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
    let mut faces = FastSet::default();
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
    let mut used = FastMap::default();
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
        assert!(
            result
                .positions
                .iter()
                .any(|p| (p[0] - 0.00005).abs() < 1e-8)
        );
        let bad = Surface {
            positions: vec![[0.; 3]],
            colors: vec![],
            triangles: vec![],
        };
        assert!(compact(&bad, 24).is_err());
    }
    #[test]
    fn small_components_are_removed_deterministically() {
        // One quad (two faces) plus a detached single-triangle fragment.
        let surface = Surface {
            positions: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [1., 1., 0.],
                [5., 5., 5.],
                [6., 5., 5.],
                [5., 6., 5.],
            ],
            colors: vec![
                [10; 3], [20; 3], [30; 3], [40; 3], [50; 3], [60; 3], [70; 3],
            ],
            triangles: vec![[0, 1, 2], [1, 3, 2], [4, 5, 6]],
        };
        let filtered = filter_small_components(&surface, 2).unwrap();
        assert_eq!(filtered.triangles, vec![[0, 1, 2], [1, 3, 2]]);
        assert_eq!(filtered.positions, surface.positions[..4]);
        assert_eq!(filtered.colors, surface.colors[..4]);
        let repeat = filter_small_components(&surface, 2).unwrap();
        assert_eq!(
            filtered
                .positions
                .iter()
                .map(|p| p.map(f64::to_bits))
                .collect::<Vec<_>>(),
            repeat
                .positions
                .iter()
                .map(|p| p.map(f64::to_bits))
                .collect::<Vec<_>>()
        );
        assert_eq!(filtered.triangles, repeat.triangles);
        // Thresholds that keep everything clone the input unchanged; a
        // threshold above every component size empties the surface.
        for min_faces in [0, 1] {
            let kept = filter_small_components(&surface, min_faces).unwrap();
            assert_eq!(kept.positions, surface.positions);
            assert_eq!(kept.triangles, surface.triangles);
            assert_eq!(kept.colors, surface.colors);
        }
        assert!(
            filter_small_components(&surface, 3)
                .unwrap()
                .triangles
                .is_empty()
        );
        // Shared vertices join faces into one component; nothing is removed.
        let joined = Surface {
            triangles: vec![[0, 1, 2], [4, 5, 0]],
            ..surface.clone()
        };
        assert_eq!(
            filter_small_components(&joined, 2).unwrap().triangles.len(),
            2
        );
        assert!(
            filter_small_components(&joined, 3)
                .unwrap()
                .triangles
                .is_empty()
        );
        let bad = Surface {
            positions: vec![[0.; 3]],
            colors: vec![],
            triangles: vec![],
        };
        assert!(filter_small_components(&bad, 2).is_err());
    }
}
