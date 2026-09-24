//! Tolerance-bounded classification of disjoint, nested boundary shells.
//! This establishes geometric material regions, never a mechanical bond strength.
use super::*;

#[derive(Debug)]
pub struct MaterialShell {
    pub first_triangle: usize,
    pub parent: Option<usize>,
    pub depth: usize,
}
#[derive(Debug)]
pub struct MaterialAudit {
    pub tolerance_mm: f64,
    pub material_regions: usize,
    pub shells: Vec<MaterialShell>,
}

pub fn audit_material(mesh: &Mesh) -> Result<MaterialAudit> {
    audit_with_budget(mesh, 8_000_000)
}

fn audit_with_budget(mesh: &Mesh, max_work: usize) -> Result<MaterialAudit> {
    mesh.validate()?;
    if mesh.indices.is_empty() || mesh.indices.len() / 3 > 100_000 {
        return Err(invalid("Material audit requires 1-100000 triangles"));
    }
    let (lo, hi) = bounds(mesh).ok_or_else(|| invalid("Material audit requires bounds"))?;
    let extent = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max);
    if !extent.is_finite() || extent <= 0. {
        return Err(numeric("Material bounds cannot be normalized"));
    }
    let origin = std::array::from_fn(|k| lo[k] / 2. + hi[k] / 2.);
    let mesh = normalized(mesh, origin, extent)?;
    validate_solid(&mesh)?;
    let eps = default_tolerance();
    let mut budget = Budget {
        stage: "material shell audit",
        work: 0,
        fragments: 0,
        options: Options {
            max_work,
            ..Options::default()
        },
    };
    validation::geometry(&mesh, eps, &mut budget)?;
    validation::orientation(&mesh, eps, &mut budget)?;

    // Vertex-manifold validation above makes vertex and edge components identical.
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let triangles = mesh.indices.as_chunks::<3>().0;
    let mut parent: Vec<_> = (0..triangles.len()).collect();
    let mut owner = vec![None; mesh.positions.len() / 3];
    for (i, t) in triangles.iter().enumerate() {
        for &v in t {
            if let Some(j) = owner[v] {
                let a = root(&mut parent, i);
                let b = root(&mut parent, j);
                parent[a] = b;
            } else {
                owner[v] = Some(i);
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for i in 0..triangles.len() {
        groups.entry(root(&mut parent, i)).or_default().push(i);
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by_key(|g| g[0]);
    if groups.len() > 128 {
        return Err(limit());
    }
    // With intersections/touching ruled out, a boundary point of one shell is
    // strictly inside or outside every other shell. No centroid-in-solid guess.
    let mut containers = vec![Vec::new(); groups.len()];
    for (i, group) in groups.iter().enumerate() {
        let point = mesh.point(triangles[group[0]][0])?;
        for (j, other) in groups.iter().enumerate() {
            if i == j {
                continue;
            }
            let w =
                validation::winding_triangles(&mesh, point, other.iter().copied(), &mut budget)?
                    .abs();
            if !w.is_finite() || (w > 1e-4 && (w - 1.).abs() > 1e-4) {
                return Err(invalid(
                    "Ambiguous shell containment at material audit tolerance",
                ));
            }
            if w > 0.5 {
                containers[i].push(j);
            }
        }
    }
    let mut shells = Vec::new();
    for (i, group) in groups.iter().enumerate() {
        let depth = containers[i].len();
        let parent = containers[i]
            .iter()
            .copied()
            .max_by_key(|&j| containers[j].len());
        if let Some(p) = parent {
            if containers[p].len() + 1 != depth
                || containers[p].iter().any(|j| !containers[i].contains(j))
            {
                return Err(invalid(
                    "Shell containment does not form a nested hierarchy",
                ));
            }
        }
        shells.push(MaterialShell {
            first_triangle: group[0],
            parent,
            depth,
        });
    }
    Ok(MaterialAudit {
        tolerance_mm: eps * extent,
        material_regions: shells.iter().filter(|s| s.depth % 2 == 0).count(),
        shells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::modeling::{Profile, extrude};
    fn cube(size: f64, offset: [f64; 3], reversed: bool) -> Mesh {
        let mut mesh = extrude(
            &Profile {
                outer: vec![[0., 0.], [size, 0.], [size, size], [0., size]],
                holes: vec![],
            },
            [0., 0., size],
        )
        .unwrap()
        .mesh;
        for p in mesh.positions.chunks_exact_mut(3) {
            for k in 0..3 {
                p[k] += offset[k];
            }
        }
        if reversed {
            for t in mesh.indices.chunks_exact_mut(3) {
                t.swap(1, 2);
            }
        }
        mesh
    }
    fn append(a: &mut Mesh, b: Mesh) {
        let offset = a.positions.len() / 3;
        a.positions.extend(b.positions);
        a.indices.extend(b.indices.into_iter().map(|i| i + offset));
    }
    #[test]
    fn nested_cavity_and_island_have_alternating_material_regions() {
        let mut mesh = cube(10., [0.; 3], false);
        append(&mut mesh, cube(6., [2.; 3], true));
        append(&mut mesh, cube(2., [4.; 3], false));
        let audit = audit_material(&mesh).unwrap();
        assert_eq!(audit.material_regions, 2);
        assert_eq!(
            audit.shells.iter().map(|s| s.depth).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            audit.shells.iter().map(|s| s.parent).collect::<Vec<_>>(),
            vec![None, Some(0), Some(1)]
        );
    }
    #[test]
    fn separated_solids_and_translated_cavity() {
        let mut mesh = cube(10., [1e6; 3], false);
        append(&mut mesh, cube(6., [1e6 + 2.; 3], true));
        append(&mut mesh, cube(10., [1e6 + 20., 1e6, 1e6], false));
        let audit = audit_material(&mesh).unwrap();
        assert_eq!(audit.material_regions, 2);
        assert_eq!(audit.shells[1].parent, Some(0));
        assert_eq!(audit.shells[2].parent, None);
    }
    #[test]
    fn refuses_overlap_contact_wrong_orientation_and_budget_exhaustion() {
        for offset in [[5., 0., 0.], [10., 0., 0.], [10., 10., 0.], [10.; 3]] {
            let mut mesh = cube(10., [0.; 3], false);
            append(&mut mesh, cube(10., offset, false));
            assert!(audit_material(&mesh).is_err(), "offset {offset:?}");
        }
        let mut mesh = cube(10., [0.; 3], false);
        append(&mut mesh, cube(6., [2.; 3], false));
        assert!(audit_material(&mesh).is_err());
        assert!(audit_with_budget(&cube(10., [0.; 3], false), 1).is_err());
    }
    #[test]
    fn parent_may_follow_child_in_source_order() {
        let mut mesh = cube(6., [2.; 3], true);
        append(&mut mesh, cube(10., [0.; 3], false));
        let audit = audit_material(&mesh).unwrap();
        assert_eq!(audit.material_regions, 1);
        assert_eq!(audit.shells[0].parent, Some(1));
        assert_eq!(audit.shells[0].depth, 1);
        assert_eq!(audit.shells[1].depth, 0);
    }
}
