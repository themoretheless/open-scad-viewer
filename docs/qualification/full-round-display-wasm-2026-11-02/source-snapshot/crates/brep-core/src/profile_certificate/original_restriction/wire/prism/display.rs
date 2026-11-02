//! Display-only sampling of an admitted exact extrusion. This is not a
//! geometric approximation certificate, a replacement Body or a mass proof.
use super::*;
use num_traits::ToPrimitive;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct DisplayMesh {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    /// Actual topology face index for each output triangle.
    pub face_ids: Vec<usize>,
}
fn key(p: [f64; 2]) -> [u64; 2] {
    p.map(|v| if v == 0. { 0 } else { v.to_bits() })
}
fn finite(q: &Q) -> Result<f64> {
    q.to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| refused("Exact display coordinate exceeds binary64"))
}
fn sample(net: &[[Q; 3]], i: usize, n: usize) -> [Q; 2] {
    let t = Q::new(i.into(), n.into());
    let a = Q::one() - &t;
    let mut h = net.to_vec();
    for count in (1..h.len()).rev() {
        for j in 0..count {
            h[j] = std::array::from_fn(|axis| &a * &h[j][axis] + &t * &h[j + 1][axis]);
        }
    }
    [&h[0][0] / &h[0][2], &h[0][1] / &h[0][2]]
}
/// Samples each original-owned exact span in its natural Bezier clock.
/// Cap triangulation may refuse. It cannot substitute or modify exact geometry.
pub fn tessellate(solid: &Solid, segments: usize, max_triangles: usize) -> Result<DisplayMesh> {
    if !(1..=32).contains(&segments) || max_triangles == 0 || max_triangles > 20_000 {
        return Err(refused("Invalid exact display sampling or triangle budget"));
    }
    let shell = solid.shell_geometry();
    let points = shell
        .sources
        .iter()
        .flat_map(|w| w.pieces.iter().flatten())
        .try_fold(0usize, |sum, span| {
            sum.checked_add(if span.net.len() == 2 { 1 } else { segments })
        })
        .ok_or_else(|| refused("Exact display sampling budget overflow"))?;
    if points.saturating_add(2 * shell.sources.len().saturating_sub(1)) > 4096
        || points.saturating_mul(2) > max_triangles
    {
        return Err(refused("Exact display profile exceeds sampling budget"));
    }
    let height = [finite(&shell.height[0])?, finite(&shell.height[1])?];
    let mut result = DisplayMesh {
        positions: vec![],
        indices: vec![],
        face_ids: vec![],
    };
    let mut rings = Vec::new();
    let mut projected: BTreeMap<[u64; 2], usize> = BTreeMap::new();
    let mut wall = 0;
    for wire in &shell.sources {
        let mut ring: Vec<usize> = Vec::new();
        let mut owners = Vec::new();
        for span in wire.pieces.iter().flatten() {
            let count = if span.net.len() == 2 { 1 } else { segments };
            for i in 0..count {
                let exact = sample(&span.net, i, count);
                let xy = [finite(&exact[0])?, finite(&exact[1])?];
                let id = result.positions.len() / 6;
                if projected.insert(key(xy), id).is_some() {
                    return Err(refused(
                        "Distinct exact profile vertices merge in display coordinates",
                    ));
                }
                for z in height {
                    result.positions.extend([xy[0], xy[1], z]);
                }
                ring.push(id);
                owners.push(wall);
            }
            wall += 1;
        }
        if ring.len() < 3 {
            return Err(refused("Insufficient exact display ring vertices"));
        }
        for i in 0..ring.len() {
            let a = 2 * ring[i];
            let b = 2 * ring[(i + 1) % ring.len()];
            result.indices.extend([a, b, b + 1, a, b + 1, a + 1]);
            result.face_ids.extend([owners[i]; 2]);
        }
        rings.push(ring);
    }
    if wall != shell.wall_sources.len() || solid.model().faces.len() != wall + 2 {
        return Err(refused(
            "Exact display face ownership differs from admitted topology",
        ));
    }
    let xy = |id: usize| [result.positions[6 * id], result.positions[6 * id + 1]];
    let contours = rings
        .iter()
        .map(|r| r.iter().map(|i| xy(*i)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let cap = planar_geometry::triangulation::triangulate_profile(&contours[0], &contours[1..])?;
    if result.indices.len() / 3 + 2 * cap.indices.len() / 3 > max_triangles {
        return Err(refused("Exact display exceeds triangle budget"));
    }
    let cap_ids =
        cap.positions
            .iter()
            .map(|p| {
                projected.get(&key(*p)).copied().ok_or_else(|| {
                    refused("Cap triangulation introduced an unowned display vertex")
                })
            })
            .collect::<Result<Vec<_>>>()?;
    for t in cap.indices.chunks_exact(3) {
        let [a, b, c] = [
            2 * cap_ids[t[0] as usize],
            2 * cap_ids[t[1] as usize],
            2 * cap_ids[t[2] as usize],
        ];
        result.indices.extend([a, c, b, a + 1, b + 1, c + 1]);
        result.face_ids.extend([wall, wall + 1]);
    }
    Ok(result)
}
