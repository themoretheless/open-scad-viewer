//! Explicit Tet4 discretization with intact, zero-thickness triangular bonds.
//! Small-strain isotropic elasticity; initiation screening, no damage evolution.
use crate::{Error, Result, truss};
use nalgebra::{DMatrix, Matrix3, SMatrix, SVector, Vector3};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Tet {
    pub nodes: [usize; 4],
    pub infill: bool,
    pub young_mpa: f64,
    pub poisson: f64,
}
#[derive(Clone, Debug)]
pub struct Bond {
    /// Matched nodes of shell and infill boundary triangles, respectively.
    pub shell: [usize; 3],
    pub infill: [usize; 3],
    pub normal_stiffness_mpa_per_mm: f64,
    pub shear_stiffness_mpa_per_mm: f64,
    pub tension_mpa: f64,
    pub shear_mpa: f64,
    pub compression_mpa: f64,
}
#[derive(Clone, Debug)]
pub struct Model {
    pub nodes_mm: Vec<[f64; 3]>,
    pub tets: Vec<Tet>,
    pub bonds: Vec<Bond>,
    pub restrained: Vec<[bool; 3]>,
    pub forces_n: Vec<[f64; 3]>,
    pub safety_factor: f64,
}
#[derive(Debug)]
pub struct BondResponse {
    pub area_mm2: f64,
    pub normal: [f64; 3],
    /// Force exerted by the infill on the shell; opposite force acts on infill.
    pub force_on_shell_n: [f64; 3],
    pub opening_mm: [f64; 3],
    pub normal_traction_mpa: [f64; 3],
    pub shear_traction_mpa: [f64; 3],
    pub utilization: f64,
}
#[derive(Debug)]
pub struct Response {
    pub linear: truss::Response,
    /// xx, yy, zz, xy, yz, xz; tensor shear stresses (not engineering strains).
    pub stresses_mpa: Vec<[f64; 6]>,
    pub volumes_mm3: Vec<f64>,
    pub bonds: Vec<BondResponse>,
    pub limit_reached: bool,
}
fn invalid(s: &str) -> Error {
    Error::new("BONDED_SOLID_INVALID_INPUT", s)
}
fn finite_positive(x: f64) -> bool {
    x.is_finite() && x > 0.
}
type BMatrix = SMatrix<f64, 6, 12>;
type DMatrix6 = SMatrix<f64, 6, 6>;
struct Element {
    b: BMatrix,
    d: DMatrix6,
    volume: f64,
}
fn element(points: &[Vector3<f64>; 4], tet: &Tet) -> Result<Element> {
    if !finite_positive(tet.young_mpa)
        || !tet.poisson.is_finite()
        || tet.poisson <= -1.
        || tet.poisson >= 0.49
    {
        return Err(invalid("Tet4 requires E > 0 and -1 < Poisson ratio < 0.49"));
    }
    let edges = Matrix3::from_columns(&[
        points[1] - points[0],
        points[2] - points[0],
        points[3] - points[0],
    ]);
    let scale = edges.iter().map(|x| x.abs()).fold(0., f64::max);
    if !finite_positive(scale) {
        return Err(invalid("Degenerate tetrahedron"));
    }
    let normalized = edges / scale;
    let det = normalized.determinant();
    if !det.is_finite() || det.abs() < 1e-10 {
        return Err(invalid("Degenerate or excessively thin tetrahedron"));
    }
    let inverse = normalized
        .try_inverse()
        .ok_or_else(|| invalid("Singular tetrahedron"))?
        / scale;
    let mut gradients = [Vector3::zeros(); 4];
    for i in 1..4 {
        gradients[i] = inverse.row(i - 1).transpose();
    }
    gradients[0] = -gradients[1] - gradients[2] - gradients[3];
    let mut b = BMatrix::zeros();
    for (i, g) in gradients.iter().enumerate() {
        let j = i * 3;
        b[(0, j)] = g[0];
        b[(1, j + 1)] = g[1];
        b[(2, j + 2)] = g[2];
        b[(3, j)] = g[1];
        b[(3, j + 1)] = g[0];
        b[(4, j + 1)] = g[2];
        b[(4, j + 2)] = g[1];
        b[(5, j)] = g[2];
        b[(5, j + 2)] = g[0];
    }
    let mu = tet.young_mpa / (2. * (1. + tet.poisson));
    let lambda = tet.young_mpa * tet.poisson / ((1. + tet.poisson) * (1. - 2. * tet.poisson));
    let mut d = DMatrix6::zeros();
    for i in 0..3 {
        for j in 0..3 {
            d[(i, j)] = lambda + if i == j { 2. * mu } else { 0. };
        }
    }
    for i in 3..6 {
        d[(i, i)] = mu;
    }
    let volume = det.abs() * scale * scale * scale / 6.;
    if !finite_positive(volume) || b.iter().chain(d.iter()).any(|v| !v.is_finite()) {
        return Err(invalid("Element exceeds finite numeric range"));
    }
    Ok(Element { b, d, volume })
}
fn key(mut face: [usize; 3]) -> [usize; 3] {
    face.sort_unstable();
    face
}

// Convex tetrahedra: face normals and edge cross products are separating axes.
// Penetration within 1e-12 of pair extent is indistinguishable from contact.
fn check_overlap(a: &[Vector3<f64>; 4], b: &[Vector3<f64>; 4]) -> Result<()> {
    if (0..3).any(|k| {
        let alo = a.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
        let ahi = a.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
        let blo = b.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
        let bhi = b.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
        ahi <= blo || bhi <= alo
    }) {
        return Ok(());
    }
    let origin = a[0];
    let scale = a
        .iter()
        .chain(b)
        .map(|p| (p - origin).norm())
        .fold(0., f64::max);
    if !finite_positive(scale) {
        return Err(invalid("Invalid tetrahedron pair scale"));
    }
    let a = a.map(|p| (p - origin) / scale);
    let b = b.map(|p| (p - origin) / scale);
    let edges = |p: &[Vector3<f64>; 4]| {
        [
            p[1] - p[0],
            p[2] - p[0],
            p[3] - p[0],
            p[2] - p[1],
            p[3] - p[1],
            p[3] - p[2],
        ]
    };
    let ea = edges(&a);
    let eb = edges(&b);
    let mut axes = Vec::with_capacity(44);
    for p in [&a, &b] {
        for [i, j, k] in [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]] {
            axes.push((p[j] - p[i]).cross(&(p[k] - p[i])));
        }
    }
    for x in ea {
        for y in eb {
            axes.push(x.cross(&y));
        }
    }
    for axis in axes {
        let length = axis.norm();
        if length < 1e-14 {
            continue;
        }
        let axis = axis / length;
        let (alo, ahi) = a
            .iter()
            .map(|p| p.dot(&axis))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        let (blo, bhi) = b
            .iter()
            .map(|p| p.dot(&axis))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        if ahi.min(bhi) - alo.max(blo) <= 1e-12 {
            return Ok(());
        }
    }
    Err(invalid("Tetrahedron interiors overlap"))
}

pub fn solve(model: &Model) -> Result<Response> {
    let n = model.nodes_mm.len();
    if n == 0
        || n > 125
        || model.tets.is_empty()
        || model.tets.len() > 400
        || model.bonds.is_empty()
        || model.bonds.len() > 200
        || model.restrained.len() != n
        || model.forces_n.len() != n
        || !model.safety_factor.is_finite()
        || model.safety_factor < 1.
        || model
            .nodes_mm
            .iter()
            .chain(&model.forces_n)
            .flatten()
            .any(|v| !v.is_finite())
    {
        return Err(invalid(
            "Require 1-125 finite nodes, 1-400 tets, 1-200 bonds, explicit loads/supports and safety factor >= 1",
        ));
    }
    let points: Vec<_> = model.nodes_mm.iter().map(|p| Vector3::from(*p)).collect();
    let mut regions = vec![None; n];
    let mut faces = BTreeMap::<[usize; 3], Vec<(usize, usize)>>::new();
    let mut unique = BTreeSet::new();
    let mut stiffness = DMatrix::<f64>::zeros(n * 3, n * 3);
    let mut elements = Vec::new();
    for (index, tet) in model.tets.iter().enumerate() {
        let mut sorted = tet.nodes;
        sorted.sort_unstable();
        if sorted[3] >= n || sorted.windows(2).any(|w| w[0] == w[1]) || !unique.insert(sorted) {
            return Err(invalid("Invalid or duplicate tetrahedron nodes"));
        }
        for &i in &tet.nodes {
            if regions[i].is_some_and(|r| r != tet.infill) {
                return Err(invalid(
                    "Shell and infill must use separate node IDs; shared IDs would bypass bonds",
                ));
            }
            regions[i] = Some(tet.infill);
        }
        for omitted in 0..4 {
            let face: Vec<_> = tet
                .nodes
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != omitted)
                .map(|(_, v)| *v)
                .collect();
            faces
                .entry(key([face[0], face[1], face[2]]))
                .or_default()
                .push((index, tet.nodes[omitted]));
        }
        let el = element(&tet.nodes.map(|i| points[i]), tet)?;
        let k = el.b.transpose() * el.d * el.b * el.volume;
        for i in 0..12 {
            for j in 0..12 {
                stiffness[(tet.nodes[i / 3] * 3 + i % 3, tet.nodes[j / 3] * 3 + j % 3)] +=
                    k[(i, j)];
            }
        }
        elements.push(el);
    }
    // AABB pre-filter: check_overlap's own first test is exactly this strict
    // per-axis separation (no tolerance), so skipping on disjoint AABBs is a
    // pure early-exit and cannot change which pairs reach the full SAT.
    let aabbs: Vec<[[f64; 2]; 3]> = model
        .tets
        .iter()
        .map(|tet| {
            let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            for &node in &tet.nodes {
                for (k, axis) in bounds.iter_mut().enumerate() {
                    axis[0] = axis[0].min(points[node][k]);
                    axis[1] = axis[1].max(points[node][k]);
                }
            }
            bounds
        })
        .collect();
    for i in 0..model.tets.len() {
        for j in 0..i {
            let (a, b) = (&aabbs[i], &aabbs[j]);
            if (0..3).any(|k| a[k][1] <= b[k][0] || b[k][1] <= a[k][0]) {
                continue;
            }
            check_overlap(
                &model.tets[i].nodes.map(|k| points[k]),
                &model.tets[j].nodes.map(|k| points[k]),
            )?;
        }
    }
    if regions.iter().any(Option::is_none) {
        return Err(invalid("Unused nodes are not admitted"));
    }
    for (face, owners) in &faces {
        if owners.len() > 2 {
            return Err(invalid("Nonmanifold tetrahedron face"));
        }
        if owners.len() == 2 {
            let normal =
                (points[face[1]] - points[face[0]]).cross(&(points[face[2]] - points[face[0]]));
            let a = normal.dot(&(points[owners[0].1] - points[face[0]]));
            let b = normal.dot(&(points[owners[1].1] - points[face[0]]));
            if !a.is_finite() || !b.is_finite() || a.signum() == b.signum() {
                return Err(invalid(
                    "Adjacent tetrahedra must lie on opposite sides of their face",
                ));
            }
        }
    }
    let mut used = BTreeSet::new();
    let mut interfaces = Vec::new();
    for bond in &model.bonds {
        if [
            bond.normal_stiffness_mpa_per_mm,
            bond.shear_stiffness_mpa_per_mm,
            bond.tension_mpa,
            bond.shear_mpa,
            bond.compression_mpa,
        ]
        .iter()
        .any(|&v| !finite_positive(v))
        {
            return Err(invalid(
                "Bond stiffnesses and measured strengths must be finite and positive",
            ));
        }
        for (face, infill) in [(bond.shell, false), (bond.infill, true)] {
            let owners = faces
                .get(&key(face))
                .ok_or_else(|| invalid("Bond must reference an existing tetrahedron face"))?;
            if owners.len() != 1
                || model.tets[owners[0].0].infill != infill
                || !used.insert(key(face))
            {
                return Err(invalid(
                    "Bond faces must be unique exposed faces of the stated region",
                ));
            }
        }
        for i in 0..3 {
            if points[bond.shell[i]] != points[bond.infill[i]] {
                return Err(invalid(
                    "Bond node pairs must coincide exactly in the explicit zero-thickness mesh",
                ));
            }
        }
        let a = points[bond.shell[0]];
        let cross = (points[bond.shell[1]] - a).cross(&(points[bond.shell[2]] - a));
        let area = cross.norm() / 2.;
        if !finite_positive(area) {
            return Err(invalid("Invalid interface area"));
        }
        let mut normal = cross / (2. * area);
        let shell_opposite = faces[&key(bond.shell)][0].1;
        if normal.dot(&(points[shell_opposite] - a)) > 0. {
            normal = -normal;
        }
        let infill_opposite = faces[&key(bond.infill)][0].1;
        if normal.dot(&(points[infill_opposite] - a)) <= 0. {
            return Err(invalid(
                "Bonded tetrahedra must be on opposite sides of the interface",
            ));
        }
        let d = Matrix3::identity() * bond.shear_stiffness_mpa_per_mm
            + normal
                * normal.transpose()
                * (bond.normal_stiffness_mpa_per_mm - bond.shear_stiffness_mpa_per_mm);
        // Exact integral of linear triangle Ni*Nj: A/6 diagonal, A/12 off-diagonal.
        for i in 0..3 {
            for j in 0..3 {
                let factor = area / 12. * if i == j { 2. } else { 1. };
                for (ai, sa) in [(bond.shell[i], -1.), (bond.infill[i], 1.)] {
                    for (bj, sb) in [(bond.shell[j], -1.), (bond.infill[j], 1.)] {
                        for k in 0..3 {
                            for l in 0..3 {
                                stiffness[(ai * 3 + k, bj * 3 + l)] += sa * sb * factor * d[(k, l)];
                            }
                        }
                    }
                }
            }
        }
        interfaces.push((area, normal, d));
    }
    let linear = truss::solve_stiffness(
        &truss::Model {
            nodes_mm: model.nodes_mm.clone(),
            members: vec![],
            restrained: model.restrained.clone(),
            forces_n: model.forces_n.clone(),
        },
        &stiffness,
    )
    .map_err(|e| Error::new("BONDED_SOLID_SOLVE", format!("{}: {}", e.code, e.message)))?;
    let u: Vec<_> = linear
        .displacements_mm
        .iter()
        .map(|p| Vector3::from(*p))
        .collect();
    let mut stresses = Vec::new();
    for (tet, el) in model.tets.iter().zip(&elements) {
        let ue = SVector::<f64, 12>::from_fn(|i, _| u[tet.nodes[i / 3]][i % 3]);
        stresses.push((el.d * el.b * ue).into());
    }
    let mut responses = Vec::new();
    for (bond, (area, normal, d)) in model.bonds.iter().zip(interfaces) {
        let mut normal_traction = [0.; 3];
        let mut shear = [0.; 3];
        let mut opening = [0.; 3];
        let mut force = Vector3::zeros();
        let mut utilization = 0f64;
        for i in 0..3 {
            let delta = u[bond.infill[i]] - u[bond.shell[i]];
            let traction = d * delta;
            opening[i] = normal.dot(&delta);
            normal_traction[i] = normal.dot(&traction);
            shear[i] = (traction - normal * normal_traction[i]).norm();
            let initiation =
                (normal_traction[i].max(0.) / bond.tension_mpa).hypot(shear[i] / bond.shear_mpa);
            let compression = (-normal_traction[i]).max(0.) / bond.compression_mpa;
            utilization = utilization.max(initiation.max(compression) * model.safety_factor);
            force += traction * (area / 3.);
        }
        responses.push(BondResponse {
            area_mm2: area,
            normal: normal.into(),
            force_on_shell_n: force.into(),
            opening_mm: opening,
            normal_traction_mpa: normal_traction,
            shear_traction_mpa: shear,
            utilization,
        });
    }
    if stresses
        .iter()
        .flat_map(|s: &[f64; 6]| s.iter())
        .any(|v| !v.is_finite())
        || responses.iter().any(|b| {
            !b.utilization.is_finite()
                || b.force_on_shell_n
                    .iter()
                    .chain(&b.opening_mm)
                    .chain(&b.normal_traction_mpa)
                    .chain(&b.shear_traction_mpa)
                    .any(|v| !v.is_finite())
        })
    {
        return Err(invalid("Response exceeds finite numeric range"));
    }
    Ok(Response {
        limit_reached: responses.iter().any(|b| b.utilization >= 1.),
        linear,
        stresses_mpa: stresses,
        volumes_mm3: elements.iter().map(|e| e.volume).collect(),
        bonds: responses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(force: [f64; 3]) -> Model {
        Model {
            nodes_mm: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., -1.],
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
            ],
            tets: vec![
                Tet {
                    nodes: [0, 1, 2, 3],
                    infill: false,
                    young_mpa: 2000.,
                    poisson: 0.3,
                },
                Tet {
                    nodes: [4, 5, 6, 7],
                    infill: true,
                    young_mpa: 2000.,
                    poisson: 0.3,
                },
            ],
            bonds: vec![Bond {
                shell: [0, 1, 2],
                infill: [4, 5, 6],
                normal_stiffness_mpa_per_mm: 100.,
                shear_stiffness_mpa_per_mm: 50.,
                tension_mpa: 10.,
                shear_mpa: 8.,
                compression_mpa: 30.,
            }],
            restrained: vec![
                [true; 3], [true; 3], [true; 3], [true; 3], [false; 3], [false; 3], [false; 3],
                [false; 3],
            ],
            forces_n: vec![
                [0.; 3],
                [0.; 3],
                [0.; 3],
                [0.; 3],
                force.map(|x| x / 3.),
                force.map(|x| x / 3.),
                force.map(|x| x / 3.),
                [0.; 3],
            ],
            safety_factor: 1.,
        }
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9 * b.abs().max(1.), "{a} vs {b}");
    }
    #[test]
    fn uniform_opening_and_shear_transfer_equal_opposite_forces() {
        let result = solve(&sample([1., 0., 2.])).unwrap();
        for node in 4..8 {
            close(result.linear.displacements_mm[node][0], 0.04);
            close(result.linear.displacements_mm[node][2], 0.04);
        }
        let bond = &result.bonds[0];
        close(bond.area_mm2, 0.5);
        close(bond.force_on_shell_n[0], 1.);
        close(bond.force_on_shell_n[2], 2.);
        for i in 0..3 {
            close(bond.normal_traction_mpa[i], 4.);
            close(bond.shear_traction_mpa[i], 2.);
        }
        close(bond.utilization, 0.4f64.hypot(0.25));
        close(result.linear.reactions_n.iter().map(|r| r[2]).sum(), -2.);
        for s in result.stresses_mpa {
            for x in s {
                close(x, 0.);
            }
        }
    }
    #[test]
    fn bond_compression_is_separate_from_tensile_initiation() {
        let result = solve(&sample([0., 0., -3.])).unwrap();
        close(result.bonds[0].normal_traction_mpa[0], -6.);
        close(result.bonds[0].utilization, 0.2);
        let mut m = sample([0., 0., 3.]);
        m.safety_factor = 2.;
        let result = solve(&m).unwrap();
        assert!(result.limit_reached);
        close(result.bonds[0].utilization, 1.2);
    }
    #[test]
    fn material_stiffness_controls_tip_deflection_and_bond_still_transfers_load() {
        let mut m = sample([0.; 3]);
        m.forces_n[7] = [0., 0., 1.];
        let a = solve(&m).unwrap();
        m.tets[1].young_mpa *= 0.1;
        let b = solve(&m).unwrap();
        assert!(b.linear.displacements_mm[7][2] > a.linear.displacements_mm[7][2]);
        close(a.bonds[0].force_on_shell_n[2], 1.);
        close(b.bonds[0].force_on_shell_n[2], 1.);
        assert!(a.stresses_mpa[1].iter().any(|v| v.abs() > 0.1));
    }
    #[test]
    fn strain_patch_and_rigid_rotation_are_reproduced() {
        let tet = Tet {
            nodes: [0, 1, 2, 3],
            infill: false,
            young_mpa: 1000.,
            poisson: 0.25,
        };
        let p = [
            Vector3::new(0., 0., 0.),
            Vector3::new(1., 0., 0.),
            Vector3::new(0., 1., 0.),
            Vector3::new(0., 0., 1.),
        ];
        let el = element(&p, &tet).unwrap();
        let affine = Matrix3::new(0.01, 0.02, 0., 0.03, -0.01, 0., 0., 0., 0.02);
        let u = SVector::<f64, 12>::from_fn(|i, _| (affine * p[i / 3])[i % 3]);
        let strain = el.b * u;
        for (a, b) in strain.iter().zip([0.01, -0.01, 0.02, 0.05, 0., 0.]) {
            close(*a, b);
        }
        let rotation = Matrix3::new(0., -0.1, 0., 0.1, 0., 0., 0., 0., 0.);
        let u = SVector::<f64, 12>::from_fn(|i, _| (rotation * p[i / 3])[i % 3] + 1.);
        for v in (el.b * u).iter() {
            close(*v, 0.);
        }
    }
    #[test]
    fn refuses_bypass_missing_bonds_mismatch_and_unrestrained_modes() {
        let mut m = sample([0., 0., 1.]);
        m.restrained.fill([false; 3]);
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.tets[1].nodes[0] = 0;
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.bonds.push(m.bonds[0].clone());
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.nodes_mm[4][0] = 0.01;
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.tets[1].poisson = 0.499;
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.nodes_mm[7][2] = 0.;
        assert!(solve(&m).is_err());
        let mut m = sample([0.; 3]);
        m.bonds[0].shear_mpa = 0.;
        assert!(solve(&m).is_err());
    }
    #[test]
    fn rigid_frame_rotation_preserves_bond_utilization_and_forces() {
        let m = sample([1., 0., 2.]);
        let a = solve(&m).unwrap();
        let mut rotated = m.clone();
        let transform = |p: [f64; 3]| [-p[2], p[1], p[0]];
        rotated.nodes_mm = m
            .nodes_mm
            .iter()
            .map(|&p| transform(p).map(|x| x + 100.))
            .collect();
        rotated.forces_n = m.forces_n.iter().map(|&p| transform(p)).collect();
        let b = solve(&rotated).unwrap();
        close(a.bonds[0].utilization, b.bonds[0].utilization);
        for (x, y) in b.bonds[0]
            .force_on_shell_n
            .iter()
            .zip(transform(a.bonds[0].force_on_shell_n))
        {
            close(*x, y);
        }
    }
    #[test]
    fn geometric_overlap_is_refused_even_with_distinct_node_ids() {
        let a = [
            Vector3::new(0., 0., 0.),
            Vector3::new(1., 0., 0.),
            Vector3::new(0., 1., 0.),
            Vector3::new(0., 0., 1.),
        ];
        assert!(check_overlap(&a, &a.map(|p| p + Vector3::repeat(0.1))).is_err());
        assert!(check_overlap(&a, &a.map(|p| p + Vector3::repeat(2.))).is_ok());
    }
}
