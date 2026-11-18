//! Joint original-shell isotopies, exact nesting and material body roles.
use super::*;
use crate::Body;

#[derive(Clone, Debug, PartialEq)]
pub struct NurbsShellForestCertificate {
    pub version: u32,
    pub source: String,
    pub depth: u8,
    pub shell_atlases: Vec<Certificate>,
    pub orientation: Vec<i8>,
    /// contains[outer][inner], established over the joint source homotopy.
    pub contains: Vec<Vec<bool>>,
    pub parents: Vec<Option<usize>>,
    pub bodies: Vec<Body>,
}
pub(super) struct ShellProof {
    pub(super) patches: Vec<Patch>,
    pub(super) points: Vec<P>,
    pub(super) faces: Vec<Face>,
}
pub(super) struct Forest {
    pub(super) certificate: NurbsShellForestCertificate,
    pub(super) shells: Vec<ShellProof>,
}
fn isolate(m: &Model, shell: usize) -> Result<Model> {
    // This is a proof-local geometry copy. Original naming/lineage and body
    // claims do not authorize an isolated shell or affect its proof identity.
    let mut temporary = Model(m.0.clone(), Default::default());
    temporary.bodies = vec![Body {
        outer_shell: shell,
        inner_shells: vec![],
    }];
    temporary.rebuild_topology_ids();
    let (out, _) = crate::body_edit::extract(&temporary, 0);
    out.validate()?;
    Ok(out)
}
fn sign(c: &Certificate, points: &[P]) -> Result<i8> {
    if points.is_empty() {
        return Err(fail("empty original shell"));
    }
    let mut edges = vec![Vec::new(); points.len()];
    let mut six_volume = Q::zero();
    for t in &c.triangles {
        let t = t.map(|i| i as usize);
        six_volume += dot(&points[t[0]], &cross(&points[t[1]], &points[t[2]]));
        for k in 0..3 {
            edges[t[k]].push(t[(k + 1) % 3]);
            edges[t[(k + 1) % 3]].push(t[k]);
        }
    }
    let mut seen = BTreeSet::new();
    let mut pending = vec![0];
    while let Some(v) = pending.pop() {
        if seen.insert(v) {
            pending.extend(&edges[v]);
        }
    }
    if seen.len() != points.len() || six_volume.is_zero() {
        return Err(fail(
            "shell is not connected with nonzero exact oriented volume",
        ));
    }
    Ok(if six_volume.is_positive() { 1 } else { -1 })
}
pub(super) fn analyze(m: &Model, depth: u8) -> Result<Forest> {
    m.validate()?;
    if m.shells.is_empty() || m.shells.len() > 16 || depth > 3 || m.shells.iter().any(|s| !s.closed)
    {
        return Err(fail(
            "shell forest needs 1..16 closed shells and bounded atlas depth",
        ));
    }
    let mut owners = vec![0; m.faces.len()];
    for s in &m.shells {
        for u in &s.faces {
            owners[u.face] += 1;
        }
    }
    if owners.iter().any(|&n| n != 1) {
        return Err(fail("shell forest requires unique complete face ownership"));
    }
    let mut atlases = Vec::new();
    let mut shells = Vec::new();
    let mut orientation = Vec::new();
    for i in 0..m.shells.len() {
        let local = isolate(m, i)?;
        let (c, patches) = checked_charts(&local, depth)?;
        let (points, faces) = boolean::nominal(&c);
        orientation.push(sign(&c, &points)?);
        atlases.push(c);
        shells.push(ShellProof {
            patches,
            points,
            faces,
        });
    }
    let n = shells.len();
    let mut contains = vec![vec![false; n]; n];
    let mut work = Work::default();
    for i in 0..n {
        for j in i + 1..n {
            // Separate ENTIRE independent homotopies before using nominal points.
            // Two individually valid atlases alone do not preserve nesting.
            for p in &shells[i].patches {
                for q in &shells[j].patches {
                    if !disjoint(&p.motion, &q.motion, 0, &mut work)? {
                        return Err(fail(
                            "joint shell homotopies do not prove original separation",
                        ));
                    }
                }
            }
            let wij = winding(&shells[j].points[0], &shells[i].faces, &mut work)?;
            let wji = winding(&shells[i].points[0], &shells[j].faces, &mut work)?;
            if ![0, orientation[i] as i32].contains(&wij)
                || ![0, orientation[j] as i32].contains(&wji)
            {
                return Err(fail("shell winding is not zero or its proved orientation"));
            }
            contains[i][j] = wij != 0;
            contains[j][i] = wji != 0;
            if contains[i][j] && contains[j][i] {
                return Err(fail("mutually containing shells"));
            }
        }
    }
    let mut parents = vec![None; n];
    for i in 0..n {
        let containers = (0..n).filter(|&j| contains[j][i]).collect::<Vec<_>>();
        for &a in &containers {
            for &b in &containers {
                if a != b && !contains[a][b] && !contains[b][a] {
                    return Err(fail("shell containment is not a laminar forest"));
                }
            }
        }
        let immediate = containers
            .iter()
            .copied()
            .filter(|&j| containers.iter().all(|&k| k == j || contains[k][j]))
            .collect::<Vec<_>>();
        if immediate.len() != usize::from(!containers.is_empty()) {
            return Err(fail("shell has no unique immediate container"));
        }
        parents[i] = immediate.first().copied();
    }
    for i in 0..n {
        let mut ancestor = parents[i];
        let mut visited = BTreeSet::new();
        while let Some(j) = ancestor {
            if j == i || !visited.insert(j) {
                return Err(fail("cyclic shell containment"));
            }
            ancestor = parents[j];
        }
        if let Some(j) = parents[i] {
            if orientation[i] == orientation[j] {
                return Err(fail(
                    "nested shell orientations do not alternate material and void",
                ));
            }
        } else if orientation[i] != 1 {
            return Err(fail("root shell points into its unbounded complement"));
        }
        for j in 0..n {
            for k in 0..n {
                if contains[i][j] && contains[j][k] && !contains[i][k] {
                    return Err(fail("intransitive shell containment"));
                }
            }
        }
    }
    let bodies = (0..n)
        .filter(|&i| orientation[i] == 1)
        .map(|i| Body {
            outer_shell: i,
            inner_shells: (0..n)
                .filter(|&j| parents[j] == Some(i) && orientation[j] == -1)
                .collect(),
        })
        .collect();
    Ok(Forest {
        certificate: NurbsShellForestCertificate {
            version: 1,
            source: source_id(&Source::Brep(m.clone()))?,
            depth,
            shell_atlases: atlases,
            orientation,
            contains,
            parents,
            bodies,
        },
        shells,
    })
}
pub(super) fn check_roles(m: &Model, c: &NurbsShellForestCertificate) -> Result<()> {
    let canonical = |bodies: &[Body]| {
        let mut out = bodies
            .iter()
            .map(|b| {
                let mut holes = b.inner_shells.clone();
                holes.sort();
                (b.outer_shell, holes)
            })
            .collect::<Vec<_>>();
        out.sort();
        out
    };
    if canonical(&m.bodies) != canonical(&c.bodies) {
        return Err(fail(
            "declared body/cavity roles disagree with proved original shell nesting",
        ));
    }
    Ok(())
}
pub fn certify_nurbs_shell_forest(m: &Model, depth: u8) -> Result<NurbsShellForestCertificate> {
    let f = analyze(m, depth)?;
    check_roles(m, &f.certificate)?;
    Ok(f.certificate)
}
pub fn verify_nurbs_shell_forest(m: &Model, c: &NurbsShellForestCertificate) -> Result<()> {
    if c.version != 1 || c.source != source_id(&Source::Brep(m.clone()))? {
        return Err(fail("shell forest source/version binding mismatch"));
    }
    if certify_nurbs_shell_forest(m, c.depth)? != *c {
        return Err(fail(
            "shell orientation, containment or cavity ownership receipt changed",
        ));
    }
    Ok(())
}
pub(crate) fn assign_bodies(m: &mut Model, depth: u8) -> Result<NurbsShellForestCertificate> {
    let mut f = analyze(m, depth)?.certificate;
    let original = m.clone();
    m.bodies = f.bodies.clone();
    m.inherit_topology_ids(&[&original]);
    m.validate()?;
    f.source = source_id(&Source::Brep(m.clone()))?;
    check_roles(m, &f)?;
    Ok(f)
}
