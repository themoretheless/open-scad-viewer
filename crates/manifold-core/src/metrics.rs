//! Geometric and topological metrics for indexed triangle meshes.
//!
//! Volumes follow the sign of the winding: consistently outward-oriented
//! (CCW) solids yield positive signed volume; a negative result indicates
//! inward orientation. Genus is reported per connected component and only
//! for closed components — open components have undefined genus.

use crate::{EdgeKey, component_labels, edge_key};

/// Metrics of one connected component.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentMetrics {
    pub triangle_count: usize,
    pub vertex_count: usize,
    pub edge_count: usize,
    /// Boundary edges of this component (0 for closed components).
    pub boundary_edges: usize,
    /// Euler characteristic V - E + F.
    pub euler_characteristic: i64,
    /// Genus (number of handles) for closed components; `None` for open
    /// components or a non-manifold vertex/edge structure where the
    /// surface interpretation breaks down.
    pub genus: Option<usize>,
    pub signed_volume: f64,
    pub surface_area: f64,
}

/// Whole-mesh metrics plus per-component breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshMetrics {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub edge_count: usize,
    /// Euler characteristic of the whole mesh (sum over components).
    pub euler_characteristic: i64,
    /// Signed volume over all components (0 for open meshes).
    pub signed_volume: f64,
    pub surface_area: f64,
    /// True when the mesh has no boundary edges (every component closed).
    pub watertight: bool,
    pub components: Vec<ComponentMetrics>,
}

/// Compute geometric and topological metrics of an indexed triangle mesh.
pub fn metrics(positions: &[f64], indices: &[usize]) -> MeshMetrics {
    let vertex_count = positions.len() / 3;
    let triangles: Vec<[usize; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| [t[0], t[1], t[2]])
        .collect();

    let labels = component_labels(&triangles);
    // Dense component ids in first-seen order.
    let mut remap: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let dense: Vec<usize> = labels
        .iter()
        .map(|&root| {
            let next = remap.len();
            *remap.entry(root).or_insert(next)
        })
        .collect();
    let component_count = remap.len();

    let mut comp_triangles: Vec<Vec<usize>> = vec![Vec::new(); component_count];
    for (ti, &c) in dense.iter().enumerate() {
        comp_triangles[c].push(ti);
    }

    let mut components = Vec::with_capacity(component_count);
    let mut total_volume = 0.0;
    let mut total_area = 0.0;
    let mut total_euler = 0i64;
    let mut watertight = true;

    for tris in &comp_triangles {
        let mut vertices = std::collections::HashSet::new();
        let mut edges = std::collections::HashSet::new();
        let mut boundary: std::collections::HashMap<EdgeKey, usize> =
            std::collections::HashMap::new();
        let mut volume = 0.0;
        let mut area = 0.0;
        for &ti in tris {
            let t = triangles[ti];
            for i in 0..3 {
                vertices.insert(t[i]);
                let key = edge_key(t[i], t[(i + 1) % 3]);
                edges.insert(key);
                *boundary.entry(key).or_default() += 1;
            }
            let (a, b, c) = triangle_points(positions, &t);
            volume += signed_tetra_volume(a, b, c);
            area += triangle_area(a, b, c);
        }
        let boundary_edges = boundary.values().filter(|&&uses| uses == 1).count();
        let euler = vertices.len() as i64 - edges.len() as i64 + tris.len() as i64;
        let genus = if boundary_edges == 0 {
            let g2 = 2 - euler;
            // A closed orientable surface has even 2 - 2g χ decomposition;
            // odd or negative results mean the component is not a clean
            // surface (non-manifold links) — report as unknown instead.
            if g2 >= 0 && g2 % 2 == 0 {
                Some((g2 / 2) as usize)
            } else {
                None
            }
        } else {
            None
        };
        if boundary_edges > 0 {
            watertight = false;
        }
        total_volume += if boundary_edges == 0 { volume } else { 0.0 };
        total_area += area;
        total_euler += euler;
        components.push(ComponentMetrics {
            triangle_count: tris.len(),
            vertex_count: vertices.len(),
            edge_count: edges.len(),
            boundary_edges,
            euler_characteristic: euler,
            genus,
            signed_volume: volume,
            surface_area: area,
        });
    }

    let edge_count = mesh_topology::EdgeUses::new(indices).edge_count();
    MeshMetrics {
        vertex_count,
        triangle_count: triangles.len(),
        edge_count,
        euler_characteristic: total_euler,
        signed_volume: total_volume,
        surface_area: total_area,
        watertight: watertight && !triangles.is_empty(),
        components,
    }
}

/// Signed volume of the tetrahedron (origin, a, b, c).
fn signed_tetra_volume(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let cross = [
        b[1] * c[2] - b[2] * c[1],
        b[2] * c[0] - b[0] * c[2],
        b[0] * c[1] - b[1] * c[0],
    ];
    (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
}

fn triangle_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() / 2.0
}

fn triangle_points(positions: &[f64], t: &[usize; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let p = |i: usize| [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]];
    (p(t[0]), p(t[1]), p(t[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(origin: [f64; 3], size: f64) -> (Vec<f64>, Vec<usize>) {
        let [x, y, z] = origin;
        let s = size;
        #[rustfmt::skip]
        let positions = vec![
            x, y, z, x + s, y, z, x + s, y + s, z, x, y + s, z,
            x, y, z + s, x + s, y, z + s, x + s, y + s, z + s, x, y + s, z + s,
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // front
            2, 3, 7, 2, 7, 6, // back
            1, 2, 6, 1, 6, 5, // right
            3, 0, 4, 3, 4, 7, // left
        ];
        (positions, indices)
    }

    #[test]
    fn cube_metrics() {
        let (p, i) = cube([0.0, 0.0, 0.0], 2.0);
        let m = metrics(&p, &i);
        assert_eq!(m.vertex_count, 8);
        assert_eq!(m.triangle_count, 12);
        assert_eq!(m.edge_count, 18);
        assert_eq!(m.euler_characteristic, 2);
        assert!(m.watertight);
        assert!((m.signed_volume - 8.0).abs() < 1e-12);
        assert!((m.surface_area - 24.0).abs() < 1e-12);
        assert_eq!(m.components.len(), 1);
        assert_eq!(m.components[0].genus, Some(0));
    }

    #[test]
    fn two_cubes_are_two_components() {
        let (p1, i1) = cube([0.0, 0.0, 0.0], 1.0);
        let (p2, i2) = cube([5.0, 0.0, 0.0], 1.0);
        let mut p = p1;
        p.extend(p2);
        let mut i = i1;
        i.extend(i2.iter().map(|v| v + 8));
        let m = metrics(&p, &i);
        assert_eq!(m.components.len(), 2);
        assert!((m.signed_volume - 2.0).abs() < 1e-12);
        assert!(m.components.iter().all(|c| c.genus == Some(0)));
    }

    #[test]
    fn flipped_cube_has_negative_volume() {
        let (p, mut i) = cube([0.0, 0.0, 0.0], 1.0);
        for t in i.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
        let m = metrics(&p, &i);
        assert!((m.signed_volume + 1.0).abs() < 1e-12);
    }

    #[test]
    fn open_quad_is_not_watertight_and_has_no_genus() {
        let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let i = vec![0, 1, 2, 0, 2, 3];
        let m = metrics(&p, &i);
        assert!(!m.watertight);
        assert_eq!(m.components.len(), 1);
        assert_eq!(m.components[0].boundary_edges, 4);
        assert_eq!(m.components[0].genus, None);
        assert_eq!(m.components[0].euler_characteristic, 1);
        assert!((m.surface_area - 1.0).abs() < 1e-12);
        assert_eq!(m.signed_volume, 0.0);
    }

    #[test]
    fn torus_has_genus_one() {
        // Coarse torus: R=2, r=1, 8x4 segments.
        let (major, minor) = (2.0f64, 1.0f64);
        let (nu, nv) = (8usize, 4usize);
        let mut p = Vec::new();
        for u in 0..nu {
            let a = u as f64 / nu as f64 * std::f64::consts::TAU;
            for v in 0..nv {
                let b = v as f64 / nv as f64 * std::f64::consts::TAU;
                let r = major + minor * b.cos();
                p.extend_from_slice(&[r * a.cos(), r * a.sin(), minor * b.sin()]);
            }
        }
        let at = |u: usize, v: usize| (u % nu) * nv + (v % nv);
        let mut i = Vec::new();
        for u in 0..nu {
            for v in 0..nv {
                let (a, b, c, d) = (at(u, v), at(u + 1, v), at(u + 1, v + 1), at(u, v + 1));
                i.extend_from_slice(&[a, c, b, a, d, c]);
            }
        }
        let m = metrics(&p, &i);
        assert!(m.watertight, "{m:?}");
        assert_eq!(m.components.len(), 1);
        assert_eq!(m.components[0].euler_characteristic, 0);
        assert_eq!(m.components[0].genus, Some(1));
    }
}
