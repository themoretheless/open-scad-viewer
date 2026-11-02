//! Exact regularized subtraction of represented positive-winding polyhedra.
//! All source planes participate in one arrangement, so crossing cuts create
//! caps and adjacent retained cells share identical boundary subdivisions.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DifferenceCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub vertices: Vec<ExactPoint>,
    pub triangles: Vec<[u32; 3]>,
    pub retained_cells: usize,
}

#[derive(Clone, Debug)]
pub struct DifferenceResult {
    pub certificate: DifferenceCertificate,
    pub positions: Vec<f64>,
    pub indices: Vec<u32>,
    pub exact_volume_mm3: Ratio,
    pub rounding_topology: RoundingTopologyReport,
}

fn construction(a: &SourceMesh, b: &SourceMesh) -> Result<DifferenceCertificate> {
    let aa = input(a)?;
    let bb = input(b)?;
    let mut work = Work::default();
    let planes = aa
        .planes
        .iter()
        .chain(&bb.planes)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut material = Vec::new();
    // The centroid is strictly interior to each full-dimensional convex cell.
    // Every source triangle plane is in the arrangement, hence winding is
    // constant throughout that cell's interior (including nonconvex sources).
    for cell in partition(vec![bounding_box(&aa.points)?], &planes, &mut work)? {
        let p = centroid(&cell.vertices());
        if winding(&p, &aa.faces, &mut work)? > 0 && winding(&p, &bb.faces, &mut work)? <= 0 {
            material.push(cell);
        }
    }
    let c = certificate_for(
        &material,
        [identity(a), identity(b)],
        [0, 0],
        material.len(),
        &mut work,
    )?;
    Ok(DifferenceCertificate {
        version: 1,
        sources: c.sources,
        vertices: c.vertices,
        triangles: c.triangles,
        retained_cells: material.len(),
    })
}
// A regularized set may still have point/edge contacts. Such a boundary is
// not a manifold solid; reject it rather than advertising a closed B-rep.
pub(super) fn manifold(triangles: &[[u32; 3]]) -> Result<()> {
    let mut edges = BTreeMap::<(u32, u32), usize>::new();
    let mut links = BTreeMap::<u32, BTreeMap<u32, Vec<u32>>>::new();
    for t in triangles {
        for i in 0..3 {
            let (v, a, b) = (t[i], t[(i + 1) % 3], t[(i + 2) % 3]);
            *edges.entry((v.min(a), v.max(a))).or_default() += 1;
            let link = links.entry(v).or_default();
            link.entry(a).or_default().push(b);
            link.entry(b).or_default().push(a);
        }
    }
    if edges.values().any(|n| *n != 2) {
        return Err(fail("difference boundary has a nonmanifold edge"));
    }
    for link in links.values() {
        if link.values().any(|neighbors| neighbors.len() != 2) {
            return Err(fail("difference vertex link is not a cycle"));
        }
        let mut seen = BTreeSet::new();
        let mut todo = vec![*link.keys().next().unwrap()];
        while let Some(v) = todo.pop() {
            if seen.insert(v) {
                todo.extend(&link[&v]);
            }
        }
        if seen.len() != link.len() {
            return Err(fail("difference boundary has disconnected vertex links"));
        }
    }
    Ok(())
}
fn output(c: DifferenceCertificate) -> Result<DifferenceResult> {
    manifold(&c.triangles)?;
    // Reuse the exact-boundary rounding proof, not a floating point Boolean.
    let v = display(&Certificate {
        version: 1,
        sources: c.sources.clone(),
        vertices: c.vertices.clone(),
        triangles: c.triangles.clone(),
        source_cells: [0, 0],
        sum_cells: c.retained_cells,
    })?;
    Ok(DifferenceResult {
        certificate: c,
        positions: v.positions,
        indices: v.indices,
        exact_volume_mm3: v.exact_volume_mm3,
        rounding_topology: v.rounding_topology,
    })
}
pub fn certify(container: &SourceMesh, tool: &SourceMesh) -> Result<DifferenceResult> {
    output(construction(container, tool)?)
}
/// Replay from the original meshes; certificate-supplied vertices, caps and
/// cell counts are never trusted as classification witnesses.
pub fn verify(
    container: &SourceMesh,
    tool: &SourceMesh,
    c: &DifferenceCertificate,
) -> Result<DifferenceResult> {
    let expected = construction(container, tool)?;
    if c != &expected {
        return Err(fail("polyhedral difference certificate mismatch"));
    }
    output(expected)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{cube, mesh};
    use super::*;
    fn volume(a: &SourceMesh, b: &SourceMesh, n: &str) -> DifferenceCertificate {
        let r = certify(a, b).unwrap();
        assert_eq!(r.exact_volume_mm3, [n.to_string(), "1".to_string()]);
        assert_eq!(
            verify(a, b, &r.certificate).unwrap().certificate,
            r.certificate
        );
        r.certificate
    }
    #[test]
    fn crossing_cut_caps_and_forgery() {
        let a = cube([0, 0, 0], [4, 4, 4]);
        let b = cube([2, -1, -1], [5, 5, 5]);
        let mut c = volume(&a, &b, "32");
        assert!(!c.triangles.is_empty());
        c.triangles.pop();
        assert!(verify(&a, &b, &c).is_err());
        let c = certify(&a, &b).unwrap().certificate;
        assert!(verify(&cube([0, 0, 0], [5, 4, 4]), &b, &c).is_err());
    }
    #[test]
    fn nonconvex_container_and_crossing_slot() {
        let a = mesh(vec![
            bounding_box(&[[0, 0, 0].map(integer), [4, 1, 2].map(integer)]).unwrap(),
            bounding_box(&[[0, 1, 0].map(integer), [1, 4, 2].map(integer)]).unwrap(),
        ]);
        volume(&a, &cube([2, -1, -1], [3, 2, 3]), "12");
    }
    #[test]
    fn cavity_and_disjoint_tool() {
        let a = cube([0, 0, 0], [4, 4, 4]);
        volume(&a, &cube([1, 1, 1], [3, 3, 3]), "56");
        volume(&a, &cube([5, 5, 5], [6, 6, 6]), "64");
    }
    #[test]
    fn complete_removal_and_split_components() {
        let a = cube([0, 0, 0], [4, 4, 4]);
        let c = volume(&a, &a, "0");
        assert!(c.vertices.is_empty() && c.triangles.is_empty());
        volume(&a, &cube([1, -1, -1], [3, 5, 5]), "32");
    }
    #[test]
    fn nonconvex_tool_and_boundary_contact() {
        let a = cube([0, 0, 0], [4, 4, 4]);
        let tool = mesh(vec![
            bounding_box(&[[0, 0, 0].map(integer), [4, 1, 2].map(integer)]).unwrap(),
            bounding_box(&[[0, 1, 0].map(integer), [1, 4, 2].map(integer)]).unwrap(),
        ]);
        volume(&a, &tool, "50");
        volume(&a, &cube([4, 0, 0], [5, 4, 4]), "64");
    }
    #[test]
    fn singular_vertex_link_is_refused() {
        assert!(manifold(&[[0, 1, 2], [0, 3, 4]]).is_err());
        // Two closed tetrahedra touching at exactly one abstract vertex.
        let t = [
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [0, 5, 4],
            [0, 4, 6],
            [4, 5, 6],
            [5, 0, 6],
        ];
        assert!(manifold(&t).is_err());
    }
}
