//! Experimental endpoint collapses with bounded vertex displacement.
use super::volume::FastMap;
use super::{
    limits::{MAX_SURFACE_TRIANGLES, MAX_SURFACE_VERTICES},
    Surface,
};
use crate::{math::*, Result};

pub fn simplify(input: &Surface, tolerance: f64, passes: usize) -> Result<Surface> {
    simplify_with_progress(input, tolerance, passes, |_, _, _| true)
}

pub fn simplify_with_progress(
    input: &Surface,
    tolerance: f64,
    passes: usize,
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    super::cancelled(&mut progress, "simplify", 0, passes)?;
    // Accept the full bounded output range of the volume extractor. These
    // are element-count caps, not a guarantee of peak allocator/RSS usage.
    if input.positions.len() > MAX_SURFACE_VERTICES || input.triangles.len() > MAX_SURFACE_TRIANGLES
    {
        return Err("Simplification input budget exceeded".into());
    }
    if !tolerance.is_finite() || tolerance < 0. || passes > 16 {
        return Err("Invalid simplification limits".into());
    }
    if input.colors.len() != input.positions.len()
        || input.positions.iter().flatten().any(|v| !v.is_finite())
        || input
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= input.positions.len())
    {
        return Err("Invalid simplification mesh".into());
    }
    let mut parent: Vec<_> = (0..input.positions.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut oriented_edges = FastMap::<(u32, u32), (u32, i32)>::default();
    for (fi, t) in input.triangles.iter().enumerate() {
        if fi % 4096 == 0 {
            super::cancelled(
                &mut progress,
                "simplify-components",
                fi,
                input.triangles.len(),
            )?;
        }
        for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
            let (a, b) = (a as usize, b as usize);
            let ra = root(&mut parent, a);
            let rb = root(&mut parent, b);
            parent[ra] = rb;
            let e = oriented_edges
                .entry((a.min(b) as u32, a.max(b) as u32))
                .or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    let component: Vec<_> = (0..parent.len()).map(|i| root(&mut parent, i)).collect();
    let mut closed = vec![true; parent.len()];
    for (&(a, _), &(count, winding)) in &oriented_edges {
        if count != 2 || winding != 0 {
            closed[component[a as usize]] = false;
        }
    }
    let mut edge_capacity_hint = oriented_edges.len();
    drop(oriented_edges);
    drop(parent);
    let contribution = |t: [u32; 3], c: usize| {
        let origin = input.positions[c];
        let p = t.map(|i| sub(input.positions[i as usize], origin));
        dot(p[0], cross(p[1], p[2])) / 6.
    };
    let mut remaining = vec![0.; input.positions.len()];
    for &t in &input.triangles {
        let c = component[t[0] as usize];
        remaining[c] += contribution(t, c);
    }
    for value in &mut remaining {
        *value = value.abs() * 0.01;
    }
    let mut faces: Vec<_> = input.triangles.iter().copied().map(Some).collect();
    let mut radius = vec![0.; input.positions.len()];
    for pass in 0..passes {
        super::cancelled(&mut progress, "simplify", pass, passes)?;
        let mut offsets = vec![0usize; input.positions.len() + 1];
        for t in faces.iter().flatten() {
            for &i in t {
                offsets[i as usize + 1] += 1;
            }
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut incident_faces = vec![0usize; *offsets.last().unwrap()];
        let mut cursor = offsets[..input.positions.len()].to_vec();
        // Input caps bound vertex/face indices and edge incidence counts
        // (at most three per face) below u32::MAX, including malformed topology.
        let mut edges = FastMap::<(u32, u32), (u32, [u32; 2])>::with_capacity_and_hasher(
            edge_capacity_hint,
            Default::default(),
        );
        for (fi, t) in faces
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.map(|t| (i, t)))
        {
            if fi % 4096 == 0 {
                super::cancelled(&mut progress, "simplify-index", fi, faces.len())?;
            }
            for &i in &t {
                let v = i as usize;
                incident_faces[cursor[v]] = fi;
                cursor[v] += 1;
            }
            for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
                let (a, b) = (a as usize, b as usize);
                let edge = edges.entry((a.min(b) as u32, a.max(b) as u32)).or_default();
                if edge.0 < 2 {
                    edge.1[edge.0 as usize] = fi as u32;
                }
                edge.0 += 1;
            }
        }
        edge_capacity_hint = edges.len();
        drop(cursor);
        let incidents = |v: usize| &incident_faces[offsets[v]..offsets[v + 1]];
        let normals: Vec<_> = faces
            .iter()
            .map(|t| {
                t.map(|t| {
                    let p = t.map(|i| input.positions[i as usize]);
                    cross(sub(p[1], p[0]), sub(p[2], p[0]))
                })
                .unwrap_or([0.; 3])
            })
            .collect();
        let mut locked = vec![false; input.positions.len()];
        for (&(a, b), f) in &edges {
            let (a, b) = (a as usize, b as usize);
            // Protect sharp creases, including the rim of thin closed shells.
            let crease = f.0 == 2
                && dot(normals[f.1[0] as usize], normals[f.1[1] as usize])
                    <= 0.5 * norm(normals[f.1[0] as usize]) * norm(normals[f.1[1] as usize]);
            if f.0 != 2 || crease {
                locked[a] = true;
                locked[b] = true;
            }
        }
        // Crease locks own the only information needed from these normals.
        drop(normals);
        // Locks and displacement radii only increase within a pass. Rejecting
        // these edges before sorting preserves the accepted collapse order.
        let mut candidates: Vec<_> = edges
            .iter()
            .filter(|&(&(a, b), f)| f.0 == 2 && !locked[a as usize] && !locked[b as usize])
            .map(|(&(a, b), f)| {
                (
                    norm(sub(
                        input.positions[a as usize],
                        input.positions[b as usize],
                    )),
                    a as usize,
                    b as usize,
                    f.1,
                )
            })
            .filter(|&(length, _, b, _)| radius[b] + length <= tolerance)
            .collect();
        candidates
            .sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut changed = false;
        let count = candidates.len();
        // Vertex valences stay small: sorted scratch vectors replace per-candidate
        // tree sets while keeping the same ascending iteration order.
        let neighbors = |faces: &[Option<[u32; 3]>], v: usize, out: &mut Vec<usize>| {
            out.clear();
            out.extend(
                incidents(v)
                    .iter()
                    .flat_map(|&f| faces[f].unwrap())
                    .map(|i| i as usize)
                    .filter(|&i| i != v),
            );
            out.sort_unstable();
            out.dedup();
        };
        let mut na = Vec::new();
        let mut nb = Vec::new();
        let mut common = Vec::new();
        let mut opposite = Vec::new();
        let mut ring = Vec::new();
        let mut unique = Vec::new();
        for (ci, (length, a, b, edge_faces)) in candidates.into_iter().enumerate() {
            if ci % 1024 == 0 {
                super::cancelled(&mut progress, "simplify-collapse", ci, count)?;
            }
            if locked[a] || locked[b] || radius[b] + length > tolerance {
                continue;
            }
            neighbors(&faces, a, &mut na);
            neighbors(&faces, b, &mut nb);
            common.clear();
            common.extend(na.iter().copied().filter(|i| nb.binary_search(i).is_ok()));
            opposite.clear();
            for &f in &edge_faces {
                opposite.extend(
                    faces[f as usize]
                        .unwrap()
                        .into_iter()
                        .map(|i| i as usize)
                        .filter(|&i| i != a && i != b),
                );
            }
            opposite.sort_unstable();
            opposite.dedup();
            if common != opposite || opposite.len() != 2 {
                continue;
            }
            ring.clear();
            ring.extend(incidents(a).iter().chain(incidents(b)).copied());
            ring.sort_unstable();
            ring.dedup();
            unique.clear();
            let mut valid = true;
            for &fi in &ring {
                let old = faces[fi].unwrap();
                if old.contains(&(a as u32)) && old.contains(&(b as u32)) {
                    continue;
                }
                let new = old.map(|i| if i as usize == b { a as u32 } else { i });
                let normal = |t: [u32; 3]| {
                    let p = t.map(|i| input.positions[i as usize]);
                    cross(sub(p[1], p[0]), sub(p[2], p[0]))
                };
                let n = normal(old);
                let m = normal(new);
                let mut key = new;
                key.sort_unstable();
                if norm(m) <= 1e-15 || dot(n, m) <= 0. || unique.contains(&key) {
                    valid = false;
                    break;
                }
                unique.push(key);
            }
            if !valid {
                continue;
            }
            let c = component[a];
            if closed[c] {
                let mut delta = 0.;
                for &fi in &ring {
                    let old = faces[fi].unwrap();
                    delta -= contribution(old, c);
                    if !(old.contains(&(a as u32)) && old.contains(&(b as u32))) {
                        delta += contribution(
                            old.map(|i| if i as usize == b { a as u32 } else { i }),
                            c,
                        );
                    }
                }
                if !delta.is_finite() || delta.abs() > remaining[c] {
                    continue;
                }
                remaining[c] -= delta.abs();
            }
            // Lock the whole affected one-ring: no later operation uses stale adjacency.
            for &fi in &ring {
                for i in faces[fi].unwrap() {
                    locked[i as usize] = true;
                }
            }
            for &fi in &ring {
                let t = faces[fi].unwrap();
                faces[fi] = if t.contains(&(a as u32)) && t.contains(&(b as u32)) {
                    None
                } else {
                    Some(t.map(|i| if i as usize == b { a as u32 } else { i }))
                };
            }
            radius[a] = radius[a].max(radius[b] + length);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    let mut result = Surface::default();
    let mut remap = vec![u32::MAX; input.positions.len()];
    let count = faces.len();
    for (fi, t) in faces.into_iter().flatten().enumerate() {
        if fi % 4096 == 0 {
            super::cancelled(&mut progress, "simplify-compact", fi, count)?;
        }
        result.triangles.push(t.map(|i| {
            let k = i as usize;
            if remap[k] == u32::MAX {
                remap[k] = result.positions.len() as u32;
                result.positions.push(input.positions[k]);
                result.colors.push(input.colors[k]);
            }
            remap[k]
        }));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduces_plane_without_moving_boundary() {
        let mut m = Surface::default();
        for y in 0..12 {
            for x in 0..12 {
                m.positions.push([x as f64, y as f64, 0.]);
                m.colors.push([100; 3]);
            }
        }
        for y in 0..11 {
            for x in 0..11 {
                let i = y * 12 + x;
                m.triangles
                    .extend([[i, i + 1, i + 12], [i + 1, i + 13, i + 12]]);
            }
        }
        let out = simplify(&m, 2., 8).unwrap();
        assert!(out.triangles.len() < m.triangles.len());
        for p in &m.positions {
            if p[0] == 0. || p[0] == 11. || p[1] == 0. || p[1] == 11. {
                assert!(out.positions.contains(p));
            }
        }
        assert!(out.positions.iter().all(|p| p[2] == 0.));
        for &p in &m.positions {
            assert!(out.positions.iter().any(|&q| norm(sub(p, q)) <= 2.));
        }
        assert_eq!(
            simplify(&m, 0., 8).unwrap().triangles.len(),
            m.triangles.len()
        );
    }
    #[test]
    fn rejects_invalid_limits_and_indices() {
        assert!(simplify(&Surface::default(), f64::NAN, 1).is_err());
        let m = Surface {
            triangles: vec![[0, 1, 2]],
            ..Default::default()
        };
        assert!(simplify(&m, 1., 1).is_err());
    }
}

#[cfg(test)]
#[test]
fn tetrahedron_does_not_collapse_into_duplicate_faces() {
    let mesh = Surface {
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        colors: vec![[0; 3]; 4],
        triangles: vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    };
    let out = simplify(&mesh, 10., 16).unwrap();
    assert_eq!(out.triangles.len(), 4);
    assert_eq!(out.positions.len(), 4);
}

#[cfg(test)]
#[test]
fn cancellation_preserves_source_and_budget_precedes_work() {
    let mesh = Surface {
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        colors: vec![[5; 3]; 3],
        triangles: vec![[0, 1, 2]],
    };
    let original = mesh.clone();
    for stop in ["simplify", "simplify-index", "simplify-compact"] {
        let result = simplify_with_progress(&mesh, 1., 2, |stage, _, _| stage != stop);
        assert!(matches!(result,Err(ref e) if e=="Cancelled"));
        assert_eq!(mesh.positions, original.positions);
        assert_eq!(mesh.triangles, original.triangles);
        assert_eq!(mesh.colors, original.colors);
    }
    let oversized_faces = Surface {
        triangles: vec![[0; 3]; 1_000_001],
        ..Default::default()
    };
    assert!(
        matches!(simplify(&oversized_faces,1.,1),Err(ref e) if e=="Simplification input budget exceeded")
    );
    let oversized = Surface {
        positions: vec![[0.; 3]; 500001],
        ..Default::default()
    };
    assert!(
        matches!(simplify(&oversized,1.,1),Err(ref e) if e=="Simplification input budget exceeded")
    );
}

#[cfg(test)]
#[test]
fn close_opposing_sheets_preserve_identity_orientation_and_area() {
    let mut mesh = Surface::default();
    let side = 20u32;
    for layer in 0..2u32 {
        let base = mesh.positions.len() as u32;
        for y in 0..side {
            for x in 0..side {
                mesh.positions
                    .push([x as f64 * 0.1, y as f64 * 0.1, layer as f64 * 0.00001]);
                mesh.colors.push([layer as u8, 0, 0]);
            }
        }
        for y in 0..side - 1 {
            for x in 0..side - 1 {
                let i = base + y * side + x;
                for mut t in [[i, i + 1, i + side], [i + 1, i + side + 1, i + side]] {
                    if layer == 1 {
                        t.swap(1, 2);
                    }
                    mesh.triangles.push(t);
                }
            }
        }
    }
    let output = simplify(&mesh, 0.4, 16).unwrap();
    assert!(output.triangles.len() < mesh.triangles.len());
    let mut areas = [0.; 2];
    for &t in &output.triangles {
        let layer = output.colors[t[0] as usize][0] as usize;
        assert!(t
            .iter()
            .all(|&i| output.colors[i as usize][0] as usize == layer));
        let p = t.map(|i| output.positions[i as usize]);
        assert!(p.iter().all(|v| v[2] == layer as f64 * 0.00001));
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        assert!(if layer == 0 { n[2] > 0. } else { n[2] < 0. });
        areas[layer] += norm(n) * 0.5;
    }
    for area in areas {
        assert!((area - 1.9 * 1.9).abs() < 1e-10);
    }
    // All perimeter points on both sheets remain, including spatially close pairs.
    for (p, c) in mesh.positions.iter().zip(&mesh.colors) {
        if p[0] == 0. || p[1] == 0. || p[0] == 19. * 0.1 || p[1] == 19. * 0.1 {
            assert!(output
                .positions
                .iter()
                .zip(&output.colors)
                .any(|(q, d)| p == q && c == d));
        }
    }
    let mut reached = false;
    let result = simplify_with_progress(&mesh, 0.4, 16, |stage, _, _| {
        if stage == "simplify-collapse" {
            reached = true;
            false
        } else {
            true
        }
    });
    assert!(reached);
    assert!(matches!(result,Err(ref e) if e=="Cancelled"));
}

#[cfg(test)]
#[test]
fn thin_closed_shell_retains_volume() {
    let mut mesh = Surface::default();
    let side = 10u32;
    let count = side * side;
    for z in [0., 0.00001] {
        for y in 0..side {
            for x in 0..side {
                mesh.positions.push([x as f64 * 0.1, y as f64 * 0.1, z]);
                mesh.colors.push([128; 3]);
            }
        }
    }
    for y in 0..side - 1 {
        for x in 0..side - 1 {
            let i = y * side + x;
            for t in [[i, i + 1, i + side], [i + 1, i + side + 1, i + side]] {
                mesh.triangles.push([t[0], t[2], t[1]]);
                mesh.triangles.push(t.map(|i| i + count));
            }
        }
    }
    let mut ring = Vec::new();
    for x in 0..side {
        ring.push(x);
    }
    for y in 1..side {
        ring.push(y * side + side - 1);
    }
    for x in (0..side - 1).rev() {
        ring.push((side - 1) * side + x);
    }
    for y in (1..side - 1).rev() {
        ring.push(y * side);
    }
    for j in 0..ring.len() {
        let a = ring[j];
        let b = ring[(j + 1) % ring.len()];
        mesh.triangles
            .extend([[a, b, b + count], [a, b + count, a + count]]);
    }
    let volume = |m: &Surface| {
        m.triangles
            .iter()
            .map(|t| {
                let p = t.map(|i| m.positions[i as usize]);
                dot(p[0], cross(p[1], p[2])) / 6.
            })
            .sum::<f64>()
            .abs()
    };
    let original = volume(&mesh);
    assert!((original - 0.81 * 0.00001).abs() < 1e-12);
    let out = simplify(&mesh, 0.4, 16).unwrap();
    assert!(out.triangles.len() < mesh.triangles.len());
    assert!(
        volume(&out) >= original * 0.99,
        "volume ratio {}",
        volume(&out) / original
    );
}

#[cfg(test)]
#[test]
fn smooth_thin_torus_volume_diagnostic() {
    let mut mesh = Surface::default();
    let (nu, nv) = (64u32, 16u32);
    for u in 0..nu {
        for v in 0..nv {
            let a = u as f64 * std::f64::consts::TAU / nu as f64;
            let b = v as f64 * std::f64::consts::TAU / nv as f64;
            let r = 1. + 0.02 * b.cos();
            mesh.positions
                .push([r * a.cos(), r * a.sin(), 0.02 * b.sin()]);
            mesh.colors.push([128; 3]);
        }
    }
    for u in 0..nu {
        for v in 0..nv {
            let a = u * nv + v;
            let b = ((u + 1) % nu) * nv + v;
            let c = u * nv + (v + 1) % nv;
            let d = ((u + 1) % nu) * nv + (v + 1) % nv;
            mesh.triangles.extend([[a, b, c], [b, d, c]]);
        }
    }
    let volume = |m: &Surface| {
        m.triangles
            .iter()
            .map(|t| {
                let p = t.map(|i| m.positions[i as usize]);
                dot(p[0], cross(p[1], p[2])) / 6.
            })
            .sum::<f64>()
            .abs()
    };
    for tolerance in [0., 0.002, 0.005, 0.01, 0.015] {
        let out = simplify(&mesh, tolerance, 16).unwrap();
        let ratio = volume(&out) / volume(&mesh);
        println!(
            "TORUS_TOLERANCE={tolerance} VOLUME_RATIO={ratio} FACES_BEFORE={} FACES_AFTER={}",
            mesh.triangles.len(),
            out.triangles.len()
        );
        assert!(ratio.is_finite());
        assert!(ratio >= 0.99 - 1e-10 && ratio <= 1.01 + 1e-10);
        if tolerance == 0. {
            assert_eq!(out.triangles.len(), mesh.triangles.len());
            assert_eq!(ratio, 1.);
        }
    }
    // Separate opposite-winding components must not cancel each other's budgets.
    for color in &mut mesh.colors {
        *color = [0; 3];
    }
    let offset = mesh.positions.len() as u32;
    let old_positions = mesh.positions.clone();
    let old_faces = mesh.triangles.clone();
    for p in old_positions {
        mesh.positions.push([p[0] + 10., p[1], p[2]]);
        mesh.colors.push([1, 0, 0]);
    }
    for t in old_faces {
        mesh.triangles
            .push([t[0] + offset, t[2] + offset, t[1] + offset]);
    }
    let output = simplify(&mesh, 0.015, 16).unwrap();
    let component_volume = |m: &Surface, layer: u8| {
        m.triangles
            .iter()
            .filter(|t| m.colors[t[0] as usize][0] == layer)
            .map(|t| {
                let p = t.map(|i| sub(m.positions[i as usize], [10. * layer as f64, 0., 0.]));
                dot(p[0], cross(p[1], p[2])) / 6.
            })
            .sum::<f64>()
            .abs()
    };
    for layer in [0, 1] {
        let ratio = component_volume(&output, layer) / component_volume(&mesh, layer);
        assert!(ratio >= 0.99 - 1e-10 && ratio <= 1.01 + 1e-10);
    }
}
