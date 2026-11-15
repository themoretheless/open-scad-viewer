//! Linear-elastic static analysis on an automatic Tet4 mesh of the finished body,
//! with layer-anisotropic (transversely isotropic) print material and a mesh
//! convergence helper.
//!
//! * Elements: constant-strain linear tetrahedra. They are stiff in bending
//!   (shear locking); bending results need refinement, which
//!   [`convergence_study`] makes explicit instead of hiding.
//! * Boundary conditions are chosen by boundary face groups from
//!   [`crate::solid_mesh`]: fixed groups clamp all three displacement
//!   components; a group load spreads a total force over the group's faces in
//!   proportion to area (consistent nodal loads for linear triangles).
//! * Solver: sparse block-CSR stiffness, block-Jacobi preconditioned conjugate
//!   gradient. Reaction balance is reported.
//! * Material: isotropic, or transversely isotropic about the build (layer
//!   stacking) direction given in part coordinates. Changing the print
//!   orientation changes the answer; rotating the part, its loads and the build
//!   direction together does not.
//!
//! Units: mm, N, MPa. No failure criteria or interlayer strength checks here.

use crate::solid_mesh::TetMesh;
use crate::{Error, Result};
use nalgebra::{Matrix3, Matrix6, SMatrix, Vector3};

pub const MAX_NODES: usize = 200_000;

fn invalid(s: &str) -> Error {
    Error::new("SOLID_FEM_INVALID_INPUT", s)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Isotropic {
        young_mpa: f64,
        poisson: f64,
    },
    /// Isotropic in the layer plane, different along `build_dir`.
    TransverselyIsotropic {
        /// Young's modulus within the layer plane.
        young_in_plane_mpa: f64,
        /// Young's modulus along the build direction (usually lower for FDM).
        young_build_mpa: f64,
        /// In-plane Poisson ratio (in-plane strain / in-plane strain).
        poisson_in_plane: f64,
        /// In-plane contraction under uniaxial stress along the build direction.
        poisson_build: f64,
        /// Shear modulus for planes containing the build direction.
        shear_build_mpa: f64,
        /// Build (layer stacking) direction in part coordinates; normalized internally.
        build_dir: [f64; 3],
    },
}

/// Voigt order [xx, yy, zz, xy, yz, zx] with engineering shear strains.
const VOIGT: [(usize, usize); 6] = [(0, 0), (1, 1), (2, 2), (0, 1), (1, 2), (2, 0)];

fn voigt_index(i: usize, j: usize) -> usize {
    match (i.min(j), i.max(j)) {
        (0, 0) => 0,
        (1, 1) => 1,
        (2, 2) => 2,
        (0, 1) => 3,
        (1, 2) => 4,
        _ => 5,
    }
}

impl Material {
    /// 6x6 stiffness in part coordinates, Voigt order of [`VOIGT`].
    pub fn stiffness(&self) -> Result<Matrix6<f64>> {
        let fin = |x: f64| x.is_finite() && x > 0.;
        match *self {
            Material::Isotropic { young_mpa, poisson } => {
                if !fin(young_mpa) || !(poisson > -1. && poisson < 0.5) {
                    return Err(invalid("Isotropic material needs E > 0 and -1 < nu < 0.5"));
                }
                let mu = young_mpa / (2. * (1. + poisson));
                let lambda = young_mpa * poisson / ((1. + poisson) * (1. - 2. * poisson));
                let mut d = Matrix6::zeros();
                for i in 0..3 {
                    for j in 0..3 {
                        d[(i, j)] = lambda + if i == j { 2. * mu } else { 0. };
                    }
                    d[(i + 3, i + 3)] = mu;
                }
                Ok(d)
            }
            Material::TransverselyIsotropic {
                young_in_plane_mpa: ep,
                young_build_mpa: ez,
                poisson_in_plane: nup,
                poisson_build: nuz,
                shear_build_mpa: gz,
                build_dir,
            } => {
                if !fin(ep) || !fin(ez) || !fin(gz) || !nup.is_finite() || !nuz.is_finite() {
                    return Err(invalid(
                        "Transversely isotropic material needs positive moduli",
                    ));
                }
                let axis = Vector3::from(build_dir);
                let len = axis.norm();
                if !fin(len) {
                    return Err(invalid("build_dir must be a non-zero finite vector"));
                }
                let e3 = axis / len;
                let helper = if e3.x.abs() < 0.9 {
                    Vector3::x()
                } else {
                    Vector3::y()
                };
                let e1 = helper.cross(&e3).normalize();
                let e2 = e3.cross(&e1);
                // Local compliance with axis 3 = build direction.
                let mut s = Matrix6::zeros();
                s[(0, 0)] = 1. / ep;
                s[(1, 1)] = 1. / ep;
                s[(2, 2)] = 1. / ez;
                s[(0, 1)] = -nup / ep;
                s[(1, 0)] = -nup / ep;
                for i in 0..2 {
                    s[(i, 2)] = -nuz / ez;
                    s[(2, i)] = -nuz / ez;
                }
                s[(3, 3)] = 2. * (1. + nup) / ep; // 12 plane
                s[(4, 4)] = 1. / gz; // 23
                s[(5, 5)] = 1. / gz; // 31
                if s.cholesky().is_none() {
                    return Err(invalid("Material compliance is not positive definite"));
                }
                let local = s
                    .try_inverse()
                    .ok_or_else(|| invalid("Singular compliance"))?;
                let r = Matrix3::from_columns(&[e1, e2, e3]); // local -> part
                let c = |i: usize, j: usize, k: usize, l: usize| {
                    local[(voigt_index(i, j), voigt_index(k, l))]
                };
                let mut d = Matrix6::zeros();
                for (p, &(i, j)) in VOIGT.iter().enumerate() {
                    for (q, &(k, l)) in VOIGT.iter().enumerate() {
                        let mut sum = 0.;
                        for a in 0..3 {
                            for b in 0..3 {
                                for cc in 0..3 {
                                    for dd in 0..3 {
                                        sum += r[(i, a)]
                                            * r[(j, b)]
                                            * r[(k, cc)]
                                            * r[(l, dd)]
                                            * c(a, b, cc, dd);
                                    }
                                }
                            }
                        }
                        d[(p, q)] = sum;
                    }
                }
                Ok(d)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupLoad {
    pub group: u32,
    /// Total force on the group, distributed by face area.
    pub force_n: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct Problem<'a> {
    pub mesh: &'a TetMesh,
    pub material: Material,
    pub fixed_groups: Vec<u32>,
    pub loads: Vec<GroupLoad>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub displacements_mm: Vec<[f64; 3]>,
    /// Constant Cauchy stress per tetrahedron, Voigt [xx, yy, zz, xy, yz, zx].
    pub stresses_mpa: Vec<[f64; 6]>,
    pub von_mises_mpa: Vec<f64>,
    pub max_displacement_mm: f64,
    pub max_von_mises_mpa: f64,
    /// |sum(reactions) + sum(applied)| / |sum(applied)|.
    pub reaction_balance_error: f64,
    pub cg_iterations: usize,
}

type B = SMatrix<f64, 6, 12>;
type Block = [[f64; 3]; 3];

fn gradients(p: &[Vector3<f64>; 4]) -> Result<(B, f64)> {
    let m = Matrix3::from_columns(&[p[1] - p[0], p[2] - p[0], p[3] - p[0]]);
    let det = m.determinant();
    if !(det.is_finite() && det > 0.) {
        return Err(invalid("Tetrahedron with non-positive volume"));
    }
    let inv = m
        .try_inverse()
        .ok_or_else(|| invalid("Singular tetrahedron"))?;
    let mut g = [Vector3::zeros(); 4];
    for i in 1..4 {
        g[i] = inv.row(i - 1).transpose();
    }
    g[0] = -g[1] - g[2] - g[3];
    let mut b = B::zeros();
    for (i, g) in g.iter().enumerate() {
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
    Ok((b, det / 6.))
}

struct BlockCsr {
    row_start: Vec<usize>,
    cols: Vec<usize>,
    vals: Vec<Block>,
}

impl BlockCsr {
    fn new(n: usize, tets: &[[usize; 4]]) -> Self {
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for t in tets {
            for &a in t {
                adj[a].extend_from_slice(t);
            }
        }
        let mut row_start = vec![0];
        let mut cols = Vec::new();
        for row in &mut adj {
            row.sort_unstable();
            row.dedup();
            cols.extend_from_slice(row);
            row_start.push(cols.len());
        }
        let vals = vec![[[0.; 3]; 3]; cols.len()];
        Self {
            row_start,
            cols,
            vals,
        }
    }
    fn slot(&self, r: usize, c: usize) -> usize {
        let s = self.row_start[r];
        s + self.cols[s..self.row_start[r + 1]]
            .binary_search(&c)
            .unwrap()
    }
    fn mul(&self, x: &[f64], y: &mut [f64]) {
        for r in 0..self.row_start.len() - 1 {
            let mut acc = [0.; 3];
            for s in self.row_start[r]..self.row_start[r + 1] {
                let c = self.cols[s];
                let b = &self.vals[s];
                for i in 0..3 {
                    acc[i] += b[i][0] * x[3 * c] + b[i][1] * x[3 * c + 1] + b[i][2] * x[3 * c + 2];
                }
            }
            y[3 * r..3 * r + 3].copy_from_slice(&acc);
        }
    }
}

fn von_mises(s: &[f64; 6]) -> f64 {
    let (xx, yy, zz, xy, yz, zx) = (s[0], s[1], s[2], s[3], s[4], s[5]);
    (0.5 * ((xx - yy).powi(2) + (yy - zz).powi(2) + (zz - xx).powi(2))
        + 3. * (xy * xy + yz * yz + zx * zx))
        .sqrt()
}

pub fn solve(problem: &Problem) -> Result<Solution> {
    let mesh = problem.mesh;
    let n = mesh.nodes_mm.len();
    if n == 0 || n > MAX_NODES || mesh.tets.is_empty() {
        return Err(invalid(
            "Mesh needs 1..=MAX_NODES nodes and at least one tetrahedron",
        ));
    }
    if mesh.tets.iter().flatten().any(|&v| v >= n)
        || mesh
            .boundary
            .iter()
            .any(|f| f.nodes.iter().any(|&v| v >= n))
    {
        return Err(invalid("Mesh index out of range"));
    }
    let d = problem.material.stiffness()?;
    let mut fixed = vec![false; n];
    for f in mesh
        .boundary
        .iter()
        .filter(|f| problem.fixed_groups.contains(&f.group))
    {
        for &v in &f.nodes {
            fixed[v] = true;
        }
    }
    if !fixed.iter().any(|&x| x) {
        return Err(invalid("No boundary face belongs to a fixed group"));
    }
    let mut force = vec![0.; 3 * n];
    let mut applied = [0.; 3];
    for load in &problem.loads {
        if load.force_n.iter().any(|x| !x.is_finite()) {
            return Err(invalid("Non-finite load"));
        }
        let faces: Vec<_> = mesh
            .boundary
            .iter()
            .filter(|f| f.group == load.group)
            .collect();
        let area = |f: &&crate::solid_mesh::BoundaryFace| {
            let [a, b, c] = f.nodes.map(|v| Vector3::from(mesh.nodes_mm[v]));
            (b - a).cross(&(c - a)).norm() / 2.
        };
        let total: f64 = faces.iter().map(area).sum();
        if !(total > 0.) {
            return Err(invalid("Loaded group has no boundary faces"));
        }
        for f in &faces {
            let w = area(f) / total / 3.;
            for &v in &f.nodes {
                for k in 0..3 {
                    force[3 * v + k] += w * load.force_n[k];
                }
            }
        }
        for k in 0..3 {
            applied[k] += load.force_n[k];
        }
    }
    let mut k = BlockCsr::new(n, &mesh.tets);
    let mut elements = Vec::with_capacity(mesh.tets.len());
    for t in &mesh.tets {
        let p = t.map(|v| Vector3::from(mesh.nodes_mm[v]));
        let (b, vol) = gradients(&p)?;
        let ke = b.transpose() * d * b * vol;
        for (a, &ra) in t.iter().enumerate() {
            for (c, &rc) in t.iter().enumerate() {
                let s = k.slot(ra, rc);
                for i in 0..3 {
                    for j in 0..3 {
                        k.vals[s][i][j] += ke[(3 * a + i, 3 * c + j)];
                    }
                }
            }
        }
        elements.push(b);
    }
    // Block-Jacobi preconditioner on free nodes.
    let mut precond = vec![Matrix3::zeros(); n];
    for r in 0..n {
        let blk = k.vals[k.slot(r, r)];
        let m = Matrix3::from_fn(|i, j| blk[i][j]);
        precond[r] = m
            .try_inverse()
            .ok_or_else(|| invalid("Singular diagonal block"))?;
    }
    let mask = |v: &mut [f64]| {
        for (i, f) in fixed.iter().enumerate() {
            if *f {
                v[3 * i..3 * i + 3].fill(0.);
            }
        }
    };
    let apply_p = |r: &[f64], z: &mut [f64]| {
        for i in 0..n {
            let v = precond[i] * Vector3::new(r[3 * i], r[3 * i + 1], r[3 * i + 2]);
            z[3 * i..3 * i + 3].copy_from_slice(v.as_slice());
        }
    };
    let dotv = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let mut u = vec![0.; 3 * n];
    let mut r = force.clone();
    mask(&mut r);
    let rhs_norm = dotv(&r, &r).sqrt();
    let mut z = vec![0.; 3 * n];
    let mut iterations = 0;
    if rhs_norm > 0. {
        apply_p(&r, &mut z);
        mask(&mut z);
        let mut p = z.clone();
        let mut rz = dotv(&r, &z);
        let mut q = vec![0.; 3 * n];
        let max_iter = 20 * 3 * n + 100;
        loop {
            k.mul(&p, &mut q);
            mask(&mut q);
            let pq = dotv(&p, &q);
            if !(pq > 0.) {
                return Err(invalid("Stiffness is not positive definite on free DOFs"));
            }
            let alpha = rz / pq;
            for i in 0..3 * n {
                u[i] += alpha * p[i];
                r[i] -= alpha * q[i];
            }
            iterations += 1;
            if dotv(&r, &r).sqrt() <= 1e-10 * rhs_norm {
                break;
            }
            if iterations >= max_iter {
                return Err(Error::new(
                    "SOLID_FEM_NOT_CONVERGED",
                    "Conjugate gradient did not converge",
                ));
            }
            apply_p(&r, &mut z);
            mask(&mut z);
            let rz_new = dotv(&r, &z);
            let beta = rz_new / rz;
            rz = rz_new;
            for i in 0..3 * n {
                p[i] = z[i] + beta * p[i];
            }
        }
    }
    // Reactions: K u - f at fixed nodes.
    let mut ku = vec![0.; 3 * n];
    k.mul(&u, &mut ku);
    let mut reaction = [0.; 3];
    for i in (0..n).filter(|&i| fixed[i]) {
        for c in 0..3 {
            reaction[c] += ku[3 * i + c] - force[3 * i + c];
        }
    }
    let applied_norm = Vector3::from(applied).norm();
    let reaction_balance_error = if applied_norm > 0. {
        (Vector3::from(reaction) + Vector3::from(applied)).norm() / applied_norm
    } else {
        0.
    };
    let displacements_mm: Vec<[f64; 3]> = (0..n)
        .map(|i| [u[3 * i], u[3 * i + 1], u[3 * i + 2]])
        .collect();
    let mut stresses = Vec::with_capacity(mesh.tets.len());
    let mut vms = Vec::with_capacity(mesh.tets.len());
    for (t, b) in mesh.tets.iter().zip(&elements) {
        let mut ue = SMatrix::<f64, 12, 1>::zeros();
        for (a, &v) in t.iter().enumerate() {
            for c in 0..3 {
                ue[3 * a + c] = u[3 * v + c];
            }
        }
        let s = d * (b * ue);
        let s = [s[0], s[1], s[2], s[3], s[4], s[5]];
        vms.push(von_mises(&s));
        stresses.push(s);
    }
    let max_displacement_mm = displacements_mm
        .iter()
        .map(|d| Vector3::from(*d).norm())
        .fold(0., f64::max);
    let max_von_mises_mpa = vms.iter().copied().fold(0., f64::max);
    if !max_displacement_mm.is_finite() || !max_von_mises_mpa.is_finite() {
        return Err(invalid("Non-finite solution"));
    }
    Ok(Solution {
        displacements_mm,
        stresses_mpa: stresses,
        von_mises_mpa: vms,
        max_displacement_mm,
        max_von_mises_mpa,
        reaction_balance_error,
        cg_iterations: iterations,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceLevel {
    pub size_mm: f64,
    pub nodes: usize,
    pub tets: usize,
    pub max_displacement_mm: f64,
    pub max_von_mises_mpa: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceReport {
    pub levels: Vec<ConvergenceLevel>,
    /// Relative change between consecutive levels (len = levels - 1).
    pub displacement_change: Vec<f64>,
    pub von_mises_change: Vec<f64>,
    /// Last displacement change below the tolerance.
    pub displacement_converged: bool,
    /// Last peak-stress change below the tolerance. Peak stress at re-entrant
    /// corners or clamped edges is singular and usually does not converge.
    pub stress_converged: bool,
    /// Overall flag; equals `displacement_converged` (the global quantity).
    pub converged: bool,
}

/// Runs `run(size)` for each mesh size (coarse to fine, 2 or 3 levels) and
/// reports the change of peak displacement and peak von Mises stress.
pub fn convergence_study<F>(
    sizes_mm: &[f64],
    tolerance: f64,
    mut run: F,
) -> Result<ConvergenceReport>
where
    F: FnMut(f64) -> Result<(TetMesh, Solution)>,
{
    if !(2..=4).contains(&sizes_mm.len()) || !(tolerance > 0. && tolerance < 1.) {
        return Err(invalid(
            "Convergence study needs 2..=4 sizes and 0 < tolerance < 1",
        ));
    }
    if sizes_mm.windows(2).any(|w| !(w[1] < w[0])) {
        return Err(invalid("Mesh sizes must strictly decrease"));
    }
    let mut levels = Vec::new();
    for &size in sizes_mm {
        let (mesh, sol) = run(size)?;
        levels.push(ConvergenceLevel {
            size_mm: size,
            nodes: mesh.nodes_mm.len(),
            tets: mesh.tets.len(),
            max_displacement_mm: sol.max_displacement_mm,
            max_von_mises_mpa: sol.max_von_mises_mpa,
        });
    }
    let rel = |a: f64, b: f64| {
        if b != 0. {
            (b - a).abs() / b.abs()
        } else {
            (b - a).abs()
        }
    };
    let displacement_change: Vec<f64> = levels
        .windows(2)
        .map(|w| rel(w[0].max_displacement_mm, w[1].max_displacement_mm))
        .collect();
    let von_mises_change: Vec<f64> = levels
        .windows(2)
        .map(|w| rel(w[0].max_von_mises_mpa, w[1].max_von_mises_mpa))
        .collect();
    let displacement_converged = *displacement_change.last().unwrap() < tolerance;
    let stress_converged = *von_mises_change.last().unwrap() < tolerance;
    Ok(ConvergenceReport {
        levels,
        displacement_change,
        von_mises_change,
        displacement_converged,
        stress_converged,
        converged: displacement_converged,
    })
}

#[cfg(test)]
#[path = "tests/solid_fem.rs"]
mod tests;
