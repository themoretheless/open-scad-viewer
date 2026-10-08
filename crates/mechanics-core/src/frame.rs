//! Small-displacement 3D Timoshenko frame elements with explicit restraints.
//!
//! Linear static analysis only: no buckling, P-Δ, plasticity, or dynamics.
//! Each member is a two-node beam (12 DOF) with axial, torsion, and bending in
//! both local planes. Shear deformation is included when shear areas are given;
//! without them the element is the exact Euler–Bernoulli beam. Rotational end
//! releases (hinges) are removed by static condensation. Member loads produce
//! work-equivalent nodal loads from Euler–Bernoulli Hermite shape functions;
//! with shear areas enabled the load vector keeps that EB form (exact for
//! slender members, which is the admitted use).
//!
//! Sign conventions. Station resultants are the force/moment that the part of
//! the member beyond the station exerts on the part behind it, in local axes;
//! positive axial is tension. For a member along +x with local z up, a downward
//! uniform load on a simply supported span gives shear_z(0⁺) = −wL/2 and
//! moment_y(L/2) = −wL²/8 (sagging is negative about +y); a cantilever with a
//! downward tip load gives moment_y(0) = +PL (hogging positive). Local axes:
//! x runs from node A to node B; z is the component of `local_z_hint` (default
//! global +Z, automatic fallback when parallel) orthogonal to x; y = z × x.
//!
//! Millimeters, newtons, MPa (N/mm²); rotations are radians. Not a certified
//! strength calculation. The host (CAD) supplies geometry; no CAD handles here.
use crate::buckling::{MAX_MODES, solve_modes};
use crate::combos::{Combination, MinMax};
use crate::diagnostics::{MIN_NORMALIZED_PIVOT, SingularityDiagnosis, diagnose_singular, summarize};
use crate::{Error, MassModel, Result};
use nalgebra::{Cholesky, DMatrix, DVector, Dyn, Matrix3, Vector3};
use std::collections::BTreeSet;

pub const MAX_NODES: usize = 125;
pub const MAX_MEMBERS: usize = 400;
pub const MAX_MEMBER_LOADS: usize = 512;
/// Springs and unilateral contacts together, 6 DOF per node at most.
pub const MAX_SUPPORTS: usize = 750;
/// Member loads of one combination merged from up to 32 load cases.
pub const MAX_COMBINED_MEMBER_LOADS: usize = 4096;
/// Stations per member diagram, endpoints included.
pub const DIAGRAM_STATIONS: usize = 21;
/// Global DOF names per node, used in singularity diagnostics.
pub const DOF_NAMES: [&str; 6] = ["x", "y", "z", "rx", "ry", "rz"];
const MAX_RELATIVE_RESIDUAL: f64 = 1e-9;

#[derive(Clone, Debug)]
pub struct Member {
    pub nodes: [usize; 2],
    pub young_mpa: f64,
    pub poisson: f64,
    pub area_mm2: f64,
    /// Second moment about the local y axis (bending in the xz plane).
    pub iyy_mm4: f64,
    /// Second moment about the local z axis (bending in the xy plane).
    pub izz_mm4: f64,
    /// Torsion constant about the local x axis.
    pub j_mm4: f64,
    /// Shear area for shear along local y. None means Euler–Bernoulli in xy.
    pub shear_area_y_mm2: Option<f64>,
    /// Shear area for shear along local z. None means Euler–Bernoulli in xz.
    pub shear_area_z_mm2: Option<f64>,
    /// Preferred local z direction; default global +Z with automatic fallback.
    pub local_z_hint: Option<[f64; 3]>,
    /// Rotational releases at node A: [torsion, about local y, about local z].
    pub release_a: [bool; 3],
    pub release_b: [bool; 3],
}

#[derive(Clone, Debug)]
pub enum MemberLoad {
    /// Point force at `at_mm` from node A, vector in local or global axes.
    PointForce {
        member: usize,
        at_mm: f64,
        force_n: [f64; 3],
        local_axes: bool,
    },
    /// Concentrated moment at `at_mm` from node A.
    PointMoment {
        member: usize,
        at_mm: f64,
        moment_nmm: [f64; 3],
        local_axes: bool,
    },
    /// Full-span uniform line load (force per unit length).
    Uniform {
        member: usize,
        force_n_per_mm: [f64; 3],
        local_axes: bool,
    },
    /// Full-span linearly varying line load, value at A to value at B.
    Trapezoidal {
        member: usize,
        from_n_per_mm: [f64; 3],
        to_n_per_mm: [f64; 3],
        local_axes: bool,
    },
}

/// Nodal and member loads of one load case; the structure is shared.
#[derive(Clone, Debug)]
pub struct LoadSet {
    pub forces_n: Vec<[f64; 3]>,
    pub moments_nmm: Vec<[f64; 3]>,
    pub loads: Vec<MemberLoad>,
}

/// Nodal support condition beyond the rigid restraint mask. Springs carry a
/// stiffness (N/mm on translations, N·mm/rad on rotations); bounds are rigid
/// unilateral contacts solved by an active-set iteration.
///
/// Unilateral supports make the response nonlinear: superposition across load
/// cases is exact only for bidirectional springs, and `solve_envelopes`
/// switches to solving every combination directly when unilaterals are present.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Support {
    /// Bidirectional linear spring on one DOF.
    Spring { node: usize, dof: usize, stiffness: f64 },
    /// Linear spring engaged only when the DOF goes negative (contact from
    /// below; the force it exerts is positive).
    LowerSpring { node: usize, dof: usize, stiffness: f64 },
    /// Linear spring engaged only when the DOF goes positive.
    UpperSpring { node: usize, dof: usize, stiffness: f64 },
    /// Rigid unilateral support: the DOF may not go negative (prop below).
    LowerBound { node: usize, dof: usize },
    /// Rigid unilateral support: the DOF may not go positive (stop above).
    UpperBound { node: usize, dof: usize },
}

impl Support {
    fn node_dof(&self) -> (usize, usize) {
        match *self {
            Support::Spring { node, dof, .. }
            | Support::LowerSpring { node, dof, .. }
            | Support::UpperSpring { node, dof, .. }
            | Support::LowerBound { node, dof }
            | Support::UpperBound { node, dof } => (node, dof),
        }
    }
    fn stiffness(&self) -> Option<f64> {
        match *self {
            Support::Spring { stiffness, .. }
            | Support::LowerSpring { stiffness, .. }
            | Support::UpperSpring { stiffness, .. } => Some(stiffness),
            _ => None,
        }
    }
    fn is_unilateral(&self) -> bool {
        !matches!(self, Support::Spring { .. })
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub nodes_mm: Vec<[f64; 3]>,
    pub members: Vec<Member>,
    /// One 6-component mask per node: 3 translations, then 3 rotations.
    pub restrained: Vec<[bool; 6]>,
    /// Springs and unilateral contacts; empty for a purely rigid support plan.
    pub supports: Vec<Support>,
    pub forces_n: Vec<[f64; 3]>,
    pub moments_nmm: Vec<[f64; 3]>,
    pub loads: Vec<MemberLoad>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub x_mm: f64,
    pub axial_n: f64,
    pub shear_y_n: f64,
    pub shear_z_n: f64,
    pub torsion_nmm: f64,
    pub moment_y_nmm: f64,
    pub moment_z_nmm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberResult {
    pub stations: Vec<Station>,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub displacements_mm: Vec<[f64; 3]>,
    pub rotations_rad: Vec<[f64; 3]>,
    /// Nonzero only at restrained translational DOFs, global XYZ.
    pub reactions_n: Vec<[f64; 3]>,
    /// Nonzero only at restrained rotational DOFs, global XYZ.
    pub reaction_moments_nmm: Vec<[f64; 3]>,
    pub members: Vec<MemberResult>,
    pub max_deflection_mm: f64,
    pub max_relative_residual: f64,
    pub free_dofs: usize,
}

/// Envelope of one station's resultants across combinations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationEnvelope {
    pub x_mm: f64,
    pub axial_n: MinMax,
    pub shear_y_n: MinMax,
    pub shear_z_n: MinMax,
    pub torsion_nmm: MinMax,
    pub moment_y_nmm: MinMax,
    pub moment_z_nmm: MinMax,
}

/// Min/max of every response quantity across load combinations, with the
/// governing combination index per scalar. See `solve_envelopes`.
#[derive(Clone, Debug)]
pub struct EnvelopeResponse {
    pub load_cases: usize,
    /// Combination names in input order.
    pub combinations: Vec<String>,
    pub displacements_mm: Vec<[MinMax; 3]>,
    pub rotations_rad: Vec<[MinMax; 3]>,
    pub reactions_n: Vec<[MinMax; 3]>,
    pub reaction_moments_nmm: Vec<[MinMax; 3]>,
    /// One station envelope per member, same station grid as `Response`.
    pub members: Vec<Vec<StationEnvelope>>,
    /// Envelope over combinations of each combination's maximum nodal
    /// displacement norm (a nonlinear quantity, enveloped as a scalar,
    /// not componentwise).
    pub max_deflection_mm: MinMax,
    pub max_relative_residual: f64,
    pub free_dofs: usize,
}

fn invalid(message: &str) -> Error {
    Error::new("FRAME_INVALID_INPUT", message)
}
fn numeric() -> Error {
    Error::new(
        "FRAME_NUMERIC_RANGE",
        "Frame calculation exceeds finite numeric range",
    )
}
fn singular_diagnosed(stiffness: &DMatrix<f64>, free: &[usize]) -> Error {
    let diagnosis = diagnose_singular(stiffness, free, &DOF_NAMES);
    Error::new(
        "FRAME_SINGULAR",
        format!(
            "Frame has an unrestrained or numerically singular mode: {}",
            summarize(&diagnosis)
        ),
    )
}

/// Member load converted into the member-local frame.
#[derive(Clone, Debug)]
enum LocalLoad {
    PointForce { at_mm: f64, force: Vector3<f64> },
    PointMoment { at_mm: f64, moment: Vector3<f64> },
    Uniform { w: Vector3<f64> },
    Trapezoidal { w_a: Vector3<f64>, w_b: Vector3<f64> },
}

/// Load-independent member data: geometry, stiffness, condensation factors.
struct Beam {
    nodes: [usize; 2],
    length: f64,
    /// Columns are the local axes in global coordinates.
    rotation: Matrix3<f64>,
    k_local: DMatrix<f64>,
    /// Condensed stiffness on kept DOFs (local axes).
    k_eff: DMatrix<f64>,
    kept: Vec<usize>,
    released: Vec<usize>,
    k_rk: DMatrix<f64>,
    k_rr: Option<Cholesky<f64, Dyn>>,
}

/// Per-load-case member data: member loads and their nodal equivalents.
struct BeamForces {
    loads: Vec<LocalLoad>,
    /// Full local work-equivalent load vector (12).
    f_eq: DVector<f64>,
    /// Condensed equivalent loads on kept DOFs (local axes).
    f_eff: DVector<f64>,
}

/// Shared structure factored once: stiffness, equilibration, Cholesky factor.
struct Assembled {
    beams: Vec<Beam>,
    n: usize,
    restrained: Vec<[bool; 6]>,
    stiffness: DMatrix<f64>,
    free: Vec<usize>,
    scales: Vec<f64>,
    factor: Option<Cholesky<f64, Dyn>>,
    matrix_norm: f64,
}

fn finite3(v: &[f64; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Hermite cubics on [0,1]; rotation rows carry the member length factor.
fn hermite(xi: f64, l: f64) -> [f64; 4] {
    let xi2 = xi * xi;
    let xi3 = xi2 * xi;
    [
        1. - 3. * xi2 + 2. * xi3,
        l * (xi - 2. * xi2 + xi3),
        3. * xi2 - 2. * xi3,
        l * (xi3 - xi2),
    ]
}

/// d/dξ of the Hermite cubics (rotation rows carry the length factor).
fn hermite_dxi(xi: f64, l: f64) -> [f64; 4] {
    [
        -6. * xi + 6. * xi * xi,
        l * (1. - 4. * xi + 3. * xi * xi),
        6. * xi - 6. * xi * xi,
        l * (-2. * xi + 3. * xi * xi),
    ]
}

/// 4-point Gauss–Legendre nodes and weights on [0,1]; exact through degree 7.
const GAUSS_XI: [f64; 4] = [
    0.069_431_844_202_973_55,
    0.330_009_478_207_571_87,
    0.669_990_521_792_428_1,
    0.930_568_155_797_026_4,
];
const GAUSS_W: [f64; 4] = [
    0.173_927_422_568_726_92,
    0.326_072_577_431_273_05,
    0.326_072_577_431_273_05,
    0.173_927_422_568_726_92,
];

/// Local 12x12 Timoshenko stiffness, DOFs [u,v,w,rx,ry,rz] per node.
/// Standard 3D frame element (Logan): θy couples to w with flipped signs
/// because θy = −dw/dx while θz = +dv/dx.
#[allow(clippy::too_many_arguments)]
fn beam_stiffness(
    e: f64,
    g: f64,
    a: f64,
    iyy: f64,
    izz: f64,
    j: f64,
    asy: Option<f64>,
    asz: Option<f64>,
    l: f64,
) -> DMatrix<f64> {
    let mut k = DMatrix::<f64>::zeros(12, 12);
    let axial = e * a / l;
    k[(0, 0)] = axial;
    k[(0, 6)] = -axial;
    k[(6, 0)] = -axial;
    k[(6, 6)] = axial;
    let torsion = g * j / l;
    k[(3, 3)] = torsion;
    k[(3, 9)] = -torsion;
    k[(9, 3)] = -torsion;
    k[(9, 9)] = torsion;
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let phi_z = asy.map_or(0., |as_| 12. * e * izz / (g * as_ * l * l));
    let cz = e * izz / (l * l * l * (1. + phi_z));
    let bz = [
        [12., 6. * l, -12., 6. * l],
        [6. * l, (4. + phi_z) * l * l, -6. * l, (2. - phi_z) * l * l],
        [-12., -6. * l, 12., -6. * l],
        [6. * l, (2. - phi_z) * l * l, -6. * l, (4. + phi_z) * l * l],
    ];
    let dz = [1usize, 5, 7, 11];
    for i in 0..4 {
        for jj in 0..4 {
            k[(dz[i], dz[jj])] = cz * bz[i][jj];
        }
    }
    // Bending about local y, DOFs [w1, ry1, w2, ry2].
    let phi_y = asz.map_or(0., |as_| 12. * e * iyy / (g * as_ * l * l));
    let cy = e * iyy / (l * l * l * (1. + phi_y));
    let by = [
        [12., -6. * l, -12., -6. * l],
        [-6. * l, (4. + phi_y) * l * l, 6. * l, (2. - phi_y) * l * l],
        [-12., 6. * l, 12., 6. * l],
        [-6. * l, (2. - phi_y) * l * l, 6. * l, (4. + phi_y) * l * l],
    ];
    let dy = [2usize, 4, 8, 10];
    for i in 0..4 {
        for jj in 0..4 {
            k[(dy[i], dy[jj])] = cy * by[i][jj];
        }
    }
    k
}

/// Add the v/w bending rows of a transverse load contribution.
/// θy rows flip sign because θy = −dw/dx.
fn add_transverse(f: &mut DVector<f64>, xi: f64, l: f64, py: f64, pz: f64) {
    let h = hermite(xi, l);
    f[1] += py * h[0];
    f[5] += py * h[1];
    f[7] += py * h[2];
    f[11] += py * h[3];
    f[2] += pz * h[0];
    f[4] -= pz * h[1];
    f[8] += pz * h[2];
    f[10] -= pz * h[3];
}

/// Work-equivalent nodal loads of one member load, local axes, 12-vector.
fn equivalent_load(load: &LocalLoad, l: f64, f: &mut DVector<f64>) {
    match load {
        LocalLoad::PointForce { at_mm, force } => {
            let xi = at_mm / l;
            f[0] += force.x * (1. - xi);
            f[6] += force.x * xi;
            add_transverse(f, xi, l, force.y, force.z);
        }
        LocalLoad::PointMoment { at_mm, moment } => {
            let xi = at_mm / l;
            f[3] += moment.x * (1. - xi);
            f[9] += moment.x * xi;
            let hd = hermite_dxi(xi, l);
            // θz = dv/dx: work m_z·θz(a).
            f[1] += moment.z * hd[0] / l;
            f[5] += moment.z * hd[1] / l;
            f[7] += moment.z * hd[2] / l;
            f[11] += moment.z * hd[3] / l;
            // θy = −dw/dx with w = H·[w, −θy]: work m_y·θy(a).
            f[2] -= moment.y * hd[0] / l;
            f[4] += moment.y * hd[1] / l;
            f[8] -= moment.y * hd[2] / l;
            f[10] += moment.y * hd[3] / l;
        }
        LocalLoad::Uniform { w } => {
            for i in 0..4 {
                let (xi, weight) = (GAUSS_XI[i], GAUSS_W[i] * l);
                f[0] += w.x * (1. - xi) * weight;
                f[6] += w.x * xi * weight;
                add_transverse(f, xi, l, w.y * weight, w.z * weight);
            }
        }
        LocalLoad::Trapezoidal { w_a, w_b } => {
            for i in 0..4 {
                let (xi, weight) = (GAUSS_XI[i], GAUSS_W[i] * l);
                let p = *w_a + (*w_b - *w_a) * xi;
                f[0] += p.x * (1. - xi) * weight;
                f[6] += p.x * xi * weight;
                add_transverse(f, xi, l, p.y * weight, p.z * weight);
            }
        }
    }
}

fn submatrix(k: &DMatrix<f64>, rows: &[usize], cols: &[usize]) -> DMatrix<f64> {
    DMatrix::from_fn(rows.len(), cols.len(), |i, j| k[(rows[i], cols[j])])
}

fn subvector(f: &DVector<f64>, rows: &[usize]) -> DVector<f64> {
    DVector::from_iterator(rows.len(), rows.iter().map(|&i| f[i]))
}

/// Block-diagonal 12x12 local→global transform (R on all four 3-blocks).
fn transformation(r: &Matrix3<f64>) -> DMatrix<f64> {
    let mut t = DMatrix::<f64>::zeros(12, 12);
    for block in 0..4 {
        for i in 0..3 {
            for j in 0..3 {
                t[(block * 3 + i, block * 3 + j)] = r[(i, j)];
            }
        }
    }
    t
}

/// Member local axes: x along the member, z from the hint, y = z × x.
fn local_frame(x_axis: Vector3<f64>, hint: [f64; 3]) -> Result<Matrix3<f64>> {
    let hint = Vector3::from(hint);
    if !hint.iter().all(|v| v.is_finite()) || hint.norm() == 0. {
        return Err(invalid("Local z hint must be finite and nonzero"));
    }
    let mut z = hint - x_axis * hint.dot(&x_axis);
    if z.norm() <= 1e-12 * hint.norm() {
        // Hint parallel to the member: deterministic fallback.
        let fallback = if x_axis.y.abs() < 0.9 {
            Vector3::new(0., 1., 0.)
        } else {
            Vector3::new(1., 0., 0.)
        };
        z = fallback - x_axis * fallback.dot(&x_axis);
    }
    let z_axis = z.normalize();
    let y_axis = z_axis.cross(&x_axis);
    Ok(Matrix3::from_columns(&[x_axis, y_axis, z_axis]))
}

/// Validate the shared structure and build load-independent member data,
/// including the static condensation factors of end releases.
fn validate_structure(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
) -> Result<Vec<Beam>> {
    let n = nodes_mm.len();
    if n == 0 || n > MAX_NODES || members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(invalid("Frame requires 1-125 nodes and 1-400 members"));
    }
    if restrained.len() != n {
        return Err(invalid("Each node requires a restraint mask"));
    }
    if nodes_mm.iter().any(|v| !finite3(v)) {
        return Err(invalid("Node coordinates must be finite"));
    }
    let mut seen = BTreeSet::new();
    let mut beams = Vec::with_capacity(members.len());
    for member in members {
        let [a, b] = member.nodes;
        if a >= n || b >= n || a == b || !seen.insert([a.min(b), a.max(b)]) {
            return Err(invalid(
                "Members require distinct valid nodes and unique unordered edges",
            ));
        }
        let positive = [
            member.young_mpa,
            member.area_mm2,
            member.iyy_mm4,
            member.izz_mm4,
            member.j_mm4,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.)
            || !member.poisson.is_finite()
            || member.poisson <= -1.
            || member.poisson >= 0.5
        {
            return Err(invalid(
                "Member E, A, I, J must be finite and positive; Poisson in (-1, 0.5)",
            ));
        }
        for area in [member.shear_area_y_mm2, member.shear_area_z_mm2]
            .into_iter()
            .flatten()
        {
            if !area.is_finite() || area <= 0. {
                return Err(invalid("Shear areas must be finite and positive"));
            }
        }
        let pa = Vector3::from(nodes_mm[a]);
        let pb = Vector3::from(nodes_mm[b]);
        let delta = pb - pa;
        let length = delta.norm();
        if !length.is_finite() || length == 0. {
            return Err(invalid("Zero-length members are not supported"));
        }
        let rotation = local_frame(
            delta / length,
            member.local_z_hint.unwrap_or([0., 0., 1.]),
        )?;
        let e = member.young_mpa;
        let g = e / (2. * (1. + member.poisson));
        if !g.is_finite() || g <= 0. {
            return Err(numeric());
        }
        let k_local = beam_stiffness(
            e,
            g,
            member.area_mm2,
            member.iyy_mm4,
            member.izz_mm4,
            member.j_mm4,
            member.shear_area_y_mm2,
            member.shear_area_z_mm2,
            length,
        );
        if k_local.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let released: Vec<usize> = [
            (3usize, member.release_a[0]),
            (4, member.release_a[1]),
            (5, member.release_a[2]),
            (9, member.release_b[0]),
            (10, member.release_b[1]),
            (11, member.release_b[2]),
        ]
        .into_iter()
        .filter_map(|(dof, on)| on.then_some(dof))
        .collect();
        let kept: Vec<usize> = (0..12).filter(|d| !released.contains(d)).collect();
        // Static condensation of released rotational DOFs: stiffness part.
        let k_kk = submatrix(&k_local, &kept, &kept);
        let (k_eff, k_rk, k_rr) = if released.is_empty() {
            (k_kk, DMatrix::zeros(0, kept.len()), None)
        } else {
            let k_rr = submatrix(&k_local, &released, &released);
            let k_rk = submatrix(&k_local, &released, &kept);
            let chol = k_rr.clone().cholesky().ok_or_else(|| {
                invalid("End releases leave the member with a free rigid rotation")
            })?;
            // Cholesky may formally factor a semidefinite block (e.g. torsion
            // released at both ends); refuse near-zero pivots explicitly.
            let pivot_scale = (0..k_rr.nrows())
                .map(|i| k_rr[(i, i)].abs())
                .fold(0., f64::max);
            if chol
                .l_dirty()
                .diagonal()
                .iter()
                .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT * pivot_scale)
            {
                return Err(invalid(
                    "End releases leave the member with a free rigid rotation",
                ));
            }
            let k_eff = &k_kk - k_rk.transpose() * chol.solve(&k_rk);
            if k_eff.iter().any(|v| !v.is_finite()) {
                return Err(numeric());
            }
            (k_eff, k_rk, Some(chol))
        };
        beams.push(Beam {
            nodes: [a, b],
            length,
            rotation,
            k_local,
            k_eff,
            kept,
            released,
            k_rk,
            k_rr,
        });
    }
    Ok(beams)
}

/// Validate one load case against the structure and convert member loads to
/// work-equivalent nodal loads in local axes.
fn prepare_loads(beams: &[Beam], load_set: &LoadSet, n: usize) -> Result<Vec<BeamForces>> {
    if load_set.forces_n.len() != n
        || load_set.moments_nmm.len() != n
        || load_set.loads.len() > MAX_MEMBER_LOADS
    {
        return Err(invalid(
            "Each node requires a force and a moment; member loads are capped at 512",
        ));
    }
    if load_set
        .forces_n
        .iter()
        .chain(&load_set.moments_nmm)
        .any(|v| !finite3(v))
    {
        return Err(invalid("Nodal loads must be finite"));
    }
    let mut forces: Vec<BeamForces> = beams
        .iter()
        .map(|_| BeamForces {
            loads: Vec::new(),
            f_eq: DVector::zeros(12),
            f_eff: DVector::zeros(0),
        })
        .collect();
    for load in &load_set.loads {
        let (member, local_axes) = match load {
            MemberLoad::PointForce {
                member,
                local_axes,
                ..
            }
            | MemberLoad::PointMoment {
                member,
                local_axes,
                ..
            }
            | MemberLoad::Uniform {
                member,
                local_axes,
                ..
            }
            | MemberLoad::Trapezoidal {
                member,
                local_axes,
                ..
            } => (*member, *local_axes),
        };
        if member >= beams.len() {
            return Err(invalid("Member load references an unknown member"));
        }
        let beam = &beams[member];
        let to_local = |v: [f64; 3]| -> Result<Vector3<f64>> {
            if !finite3(&v) {
                return Err(invalid("Member load vectors must be finite"));
            }
            let v = Vector3::from(v);
            Ok(if local_axes {
                v
            } else {
                beam.rotation.transpose() * v
            })
        };
        let at_checked = |at: f64| -> Result<f64> {
            if !at.is_finite() || at < 0. || at > beam.length {
                return Err(invalid("Point load position must lie on the member"));
            }
            Ok(at)
        };
        let local = match load {
            MemberLoad::PointForce { at_mm, force_n, .. } => LocalLoad::PointForce {
                at_mm: at_checked(*at_mm)?,
                force: to_local(*force_n)?,
            },
            MemberLoad::PointMoment {
                at_mm,
                moment_nmm,
                ..
            } => LocalLoad::PointMoment {
                at_mm: at_checked(*at_mm)?,
                moment: to_local(*moment_nmm)?,
            },
            MemberLoad::Uniform { force_n_per_mm, .. } => LocalLoad::Uniform {
                w: to_local(*force_n_per_mm)?,
            },
            MemberLoad::Trapezoidal {
                from_n_per_mm,
                to_n_per_mm,
                ..
            } => LocalLoad::Trapezoidal {
                w_a: to_local(*from_n_per_mm)?,
                w_b: to_local(*to_n_per_mm)?,
            },
        };
        forces[member].loads.push(local);
    }
    for (beam, bf) in beams.iter().zip(&mut forces) {
        let mut f_eq = DVector::<f64>::zeros(12);
        for load in &bf.loads {
            equivalent_load(load, beam.length, &mut f_eq);
        }
        if f_eq.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        // Condensation of the equivalent load vector (stiffness part is
        // already factored in the load-independent Beam).
        let f_k = subvector(&f_eq, &beam.kept);
        bf.f_eff = if beam.released.is_empty() {
            f_k
        } else {
            let chol = beam.k_rr.as_ref().expect("validated condensation");
            let f_r = subvector(&f_eq, &beam.released);
            let f_eff = f_k - beam.k_rk.transpose() * chol.solve(&f_r);
            if f_eff.iter().any(|v| !v.is_finite()) {
                return Err(numeric());
            }
            f_eff
        };
        bf.f_eq = f_eq;
    }
    Ok(forces)
}

/// Section resultants at one station: force/moment exerted by the downstream
/// part of the member on the upstream part, in local axes. Loads exactly at
/// the station count as upstream (values are right-side limits).
fn station_resultants(beam: &Beam, forces: &BeamForces, end_on_element: &[f64; 6], x: f64) -> Station {
    let mut force = Vector3::new(end_on_element[0], end_on_element[1], end_on_element[2]);
    let mut moment = Vector3::new(end_on_element[3], end_on_element[4], end_on_element[5]);
    // Moment of the end force about the cut point (node A sits at local origin).
    moment += Vector3::new(-x, 0., 0.).cross(&force);
    for load in &forces.loads {
        match load {
            LocalLoad::PointForce { at_mm, force: p } => {
                if *at_mm <= x {
                    force += p;
                    moment += Vector3::new(at_mm - x, 0., 0.).cross(p);
                }
            }
            LocalLoad::PointMoment { at_mm, moment: m } => {
                if *at_mm <= x {
                    moment += m;
                }
            }
            LocalLoad::Uniform { w } => {
                let resultant = *w * x;
                force += resultant;
                moment += Vector3::new(x / 2. - x, 0., 0.).cross(&resultant);
            }
            LocalLoad::Trapezoidal { w_a, w_b } => {
                let slope = (*w_b - *w_a) / beam.length;
                let resultant = *w_a * x + slope * (x * x / 2.);
                let first_moment = *w_a * (x * x / 2.) + slope * (x * x * x / 3.);
                let scale = 1e-12 * (1. + w_a.norm() * beam.length);
                let centroid = if resultant.norm() > scale {
                    first_moment.dot(&resultant) / resultant.dot(&resultant)
                } else {
                    x / 2.
                };
                force += resultant;
                moment += Vector3::new(centroid - x, 0., 0.).cross(&resultant);
            }
        }
    }
    let s = -force;
    let m = -moment;
    Station {
        x_mm: x,
        axial_n: s.x,
        shear_y_n: s.y,
        shear_z_n: s.z,
        torsion_nmm: m.x,
        moment_y_nmm: m.y,
        moment_z_nmm: m.z,
    }
}

/// Validate the structure and assemble the global stiffness; no factorization.
/// `springs` adds linear spring stiffness to global DOF diagonals; `restrained`
/// is the effective mask (rigid restraints plus active unilateral bounds).
fn assemble_stiffness(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    springs: &[(usize, f64)],
) -> Result<(Vec<Beam>, DMatrix<f64>)> {
    let beams = validate_structure(nodes_mm, members, restrained)?;
    let n = nodes_mm.len();
    let ndof = n * 6;
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    let mut stiffness = DMatrix::<f64>::zeros(ndof, ndof);
    for beam in &beams {
        let t = transformation(&beam.rotation);
        // Scatter the condensed local system into a full 12-system, then rotate.
        let mut k_full = DMatrix::<f64>::zeros(12, 12);
        for (i, &di) in beam.kept.iter().enumerate() {
            for (j, &dj) in beam.kept.iter().enumerate() {
                k_full[(di, dj)] = beam.k_eff[(i, j)];
            }
        }
        let k_glob = &t * &k_full * t.transpose();
        if k_glob.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            let g1 = global_dof(beam, d1);
            for d2 in 0..12 {
                stiffness[(g1, global_dof(beam, d2))] += k_glob[(d1, d2)];
            }
        }
    }
    for &(dof, k) in springs {
        stiffness[(dof, dof)] += k;
    }
    if stiffness.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    Ok((beams, stiffness))
}

/// Assemble the global stiffness and factor it once. The factorization is
/// shared by every load case of `solve_envelopes`. A singular refusal names
/// the implicated DOFs; the diagnosing LDLᵀ runs on the error path only.
fn assemble(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    springs: &[(usize, f64)],
) -> Result<Assembled> {
    let (beams, stiffness) = assemble_stiffness(nodes_mm, members, restrained, springs)?;
    let n = nodes_mm.len();
    let ndof = n * 6;

    // Equilibrated Cholesky on free DOFs, mirroring the truss solver.
    let restraint = |dof: usize| restrained[dof / 6][dof % 6];
    let free: Vec<usize> = (0..ndof).filter(|&i| !restraint(i)).collect();
    let mut scales = Vec::new();
    let mut factor = None;
    if !free.is_empty() {
        scales.reserve(free.len());
        let mut zero_diagonal = false;
        for &i in &free {
            let diagonal = stiffness[(i, i)];
            if diagonal <= 0. {
                zero_diagonal = true;
                break;
            }
            scales.push(diagonal.sqrt());
        }
        if zero_diagonal {
            return Err(singular_diagnosed(&stiffness, &free));
        }
        let reduced = DMatrix::from_fn(free.len(), free.len(), |i, j| {
            stiffness[(free[i], free[j])] / scales[i] / scales[j]
        });
        if reduced.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let Some(chol) = reduced.cholesky() else {
            return Err(singular_diagnosed(&stiffness, &free));
        };
        if chol
            .l_dirty()
            .diagonal()
            .iter()
            .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT)
        {
            return Err(singular_diagnosed(&stiffness, &free));
        }
        factor = Some(chol);
    }
    let matrix_norm = (0..ndof)
        .map(|i| (0..ndof).map(|j| stiffness[(i, j)].abs()).sum::<f64>())
        .fold(0., f64::max);
    if !matrix_norm.is_finite() {
        return Err(numeric());
    }
    Ok(Assembled {
        beams,
        n,
        restrained: restrained.to_vec(),
        stiffness,
        free,
        scales,
        factor,
        matrix_norm,
    })
}

/// Explain why a structure would be refused as singular, or confirm it is
/// stable. Load-independent: the restraint mask, supports, and end releases
/// decide. Springs and unilateral supports are all treated as engaged,
/// matching the first active-set iteration of `solve`.
pub fn diagnose(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
) -> Result<SingularityDiagnosis> {
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    let mut mask = restrained.to_vec();
    let mut diagonal = Vec::new();
    for support in supports {
        let (node, dof) = support.node_dof();
        match support {
            Support::Spring { stiffness, .. }
            | Support::LowerSpring { stiffness, .. }
            | Support::UpperSpring { stiffness, .. } => {
                diagonal.push((node * 6 + dof, *stiffness));
            }
            Support::LowerBound { .. } | Support::UpperBound { .. } => mask[node][dof] = true,
        }
    }
    let (_beams, stiffness) = assemble_stiffness(nodes_mm, members, &mask, &diagonal)?;
    let free: Vec<usize> = (0..n * 6).filter(|&i| !mask[i / 6][i % 6]).collect();
    Ok(diagnose_singular(&stiffness, &free, &DOF_NAMES))
}

/// Simple (Euler–Bernoulli-form) geometric stiffness of one member under
/// axial force `n` (tension positive): bending DOFs only, no torsion coupling.
/// With shear areas Kg keeps this EB form, matching the equivalent-load
/// approximation. Axial member loads vary N along a member; split such members
/// so the constant-N assumption holds per element.
fn beam_geometric_stiffness(n: f64, l: f64) -> DMatrix<f64> {
    let mut kg = DMatrix::<f64>::zeros(12, 12);
    let c = n / (30. * l);
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let dz = [1usize, 5, 7, 11];
    let bz = [
        [36., 3. * l, -36., 3. * l],
        [3. * l, 4. * l * l, -3. * l, -l * l],
        [-36., -3. * l, 36., -3. * l],
        [3. * l, -l * l, -3. * l, 4. * l * l],
    ];
    // Bending about local y, DOFs [w1, ry1, w2, ry2]; θy = −dw/dx flips the
    // L-odd couplings, mirroring the elastic block.
    let dy = [2usize, 4, 8, 10];
    let by = [
        [36., -3. * l, -36., -3. * l],
        [-3. * l, 4. * l * l, 3. * l, -l * l],
        [-36., 3. * l, 36., 3. * l],
        [-3. * l, -l * l, 3. * l, 4. * l * l],
    ];
    for (dofs, block) in [(dz, bz), (dy, by)] {
        for i in 0..4 {
            for j in 0..4 {
                kg[(dofs[i], dofs[j])] = c * block[i][j];
            }
        }
    }
    kg
}

/// Restrict a full 12×12 local matrix to the kept DOFs through the elastic
/// condensation subspace u = T·u_k, T = [I; −K_rr⁻¹·K_rk] — exact for the
/// stiffness itself, a consistent subspace restriction for Kg under releases.
fn condense_local(beam: &Beam, full: &DMatrix<f64>) -> DMatrix<f64> {
    if beam.released.is_empty() {
        return submatrix(full, &beam.kept, &beam.kept);
    }
    let mut t = DMatrix::<f64>::zeros(12, beam.kept.len());
    for (col, &d) in beam.kept.iter().enumerate() {
        t[(d, col)] = 1.;
    }
    let m = -beam.k_rr.as_ref().unwrap().solve(&beam.k_rk);
    for (row, &d) in beam.released.iter().enumerate() {
        for col in 0..beam.kept.len() {
            t[(d, col)] = m[(row, col)];
        }
    }
    t.transpose() * full * &t
}

/// Global geometric stiffness from per-member reference axial forces.
fn assemble_geometric(beams: &[Beam], axial: &[f64], n: usize) -> Result<DMatrix<f64>> {
    let mats: Vec<DMatrix<f64>> = beams
        .iter()
        .zip(axial)
        .map(|(beam, &axial_n)| beam_geometric_stiffness(axial_n, beam.length))
        .collect();
    assemble_member_matrices(beams, &mats, n)
}

/// Shared condense–scatter–rotate assembly of symmetric 12×12 local member
/// matrices (geometric stiffness, mass).
fn assemble_member_matrices(beams: &[Beam], mats: &[DMatrix<f64>], n: usize) -> Result<DMatrix<f64>> {
    let ndof = n * 6;
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    let mut global = DMatrix::<f64>::zeros(ndof, ndof);
    for (beam, local) in beams.iter().zip(mats) {
        let effective = condense_local(beam, local);
        if effective.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let mut full = DMatrix::<f64>::zeros(12, 12);
        for (i, &di) in beam.kept.iter().enumerate() {
            for (j, &dj) in beam.kept.iter().enumerate() {
                full[(di, dj)] = effective[(i, j)];
            }
        }
        let t = transformation(&beam.rotation);
        let rotated = &t * &full * t.transpose();
        if rotated.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            let g1 = global_dof(beam, d1);
            for d2 in 0..12 {
                global[(g1, global_dof(beam, d2))] += rotated[(d1, d2)];
            }
        }
    }
    if global.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    Ok(global)
}

/// One buckling mode of the reference load state, nodal form.
#[derive(Clone, Debug)]
pub struct FrameBucklingMode {
    /// Signed factor on the reference loads (buckling load = λ·reference);
    /// negative means the structure buckles under the reversed load.
    pub load_factor: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    pub rotations: Vec<[f64; 3]>,
    pub relative_residual: f64,
}

/// Eigenvalue buckling of the reference load state.
#[derive(Clone, Debug)]
pub struct FrameBucklingResponse {
    /// Ascending |load_factor|; the first entry is the critical mode.
    pub modes: Vec<FrameBucklingMode>,
    /// Member axial forces of the reference state (tension positive) that the
    /// geometric stiffness is built from.
    pub axial_forces_n: Vec<f64>,
    pub free_dofs: usize,
}

/// Linear buckling of a frame under a reference load set. The elastic
/// stiffness is factored once (same path as `solve`); the geometric stiffness
/// uses each member's reference axial force. Supports must be rigid or
/// bidirectional springs — unilateral contacts make the reference state
/// nonlinear, so they are refused. Not a certified stability calculation.
pub fn buckling(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    reference: &LoadSet,
    modes: usize,
) -> Result<FrameBucklingResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Buckling returns between 1 and 8 modes"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Buckling admits rigid restraints and bidirectional springs only",
        ));
    }
    let springs: Vec<(usize, f64)> = supports
        .iter()
        .filter_map(|s| match s {
            Support::Spring {
                node,
                dof,
                stiffness,
            } => Some((node * 6 + dof, *stiffness)),
            _ => None,
        })
        .collect();
    let assembled = assemble(nodes_mm, members, restrained, &springs)?;
    let reference_response = solve_case(&assembled, reference)?;
    let axial: Vec<f64> = reference_response
        .members
        .iter()
        .map(|m| m.stations[0].axial_n)
        .collect();
    let geometric = assemble_geometric(&assembled.beams, &axial, assembled.n)?;
    let solved = match &assembled.factor {
        Some(factor) => solve_modes(
            factor,
            &assembled.scales,
            &assembled.free,
            &assembled.stiffness,
            &geometric,
            modes,
        )?,
        None => Vec::new(),
    };
    let modes_out = solved
        .into_iter()
        .map(|mode| FrameBucklingMode {
            load_factor: mode.load_factor,
            displacements: (0..assembled.n)
                .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + k]))
                .collect(),
            rotations: (0..assembled.n)
                .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + 3 + k]))
                .collect(),
            relative_residual: mode.relative_residual,
        })
        .collect();
    Ok(FrameBucklingResponse {
        modes: modes_out,
        axial_forces_n: axial,
        free_dofs: assembled.free.len(),
    })
}

/// Consistent (Euler–Bernoulli) mass matrix of one member, local axes.
/// Density in t/mm³ keeps the mm/N units consistent (1 t·mm/s² = 1 N), so
/// frequencies come out in Hz. Torsion uses the polar moment ρ(Iyy+Izz).
fn beam_mass_consistent(rho: f64, area: f64, iyy: f64, izz: f64, l: f64) -> DMatrix<f64> {
    let mut m = DMatrix::<f64>::zeros(12, 12);
    let axial = rho * area * l / 6.;
    let torsion = rho * (iyy + izz) * l / 6.;
    for (dofs, value) in [([0usize, 6], axial), ([3usize, 9], torsion)] {
        for d in dofs {
            for e in dofs {
                m[(d, e)] = value * if d == e { 2. } else { 1. };
            }
        }
    }
    let c = rho * area * l / 420.;
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let dz = [1usize, 5, 7, 11];
    let bz = [
        [156., 22. * l, 54., -13. * l],
        [22. * l, 4. * l * l, 13. * l, -3. * l * l],
        [54., 13. * l, 156., -22. * l],
        [-13. * l, -3. * l * l, -22. * l, 4. * l * l],
    ];
    // Bending about local y, DOFs [w1, ry1, w2, ry2] (θy = −dw/dx pattern).
    let dy = [2usize, 4, 8, 10];
    let by = [
        [156., -22. * l, 54., 13. * l],
        [-22. * l, 4. * l * l, -13. * l, -3. * l * l],
        [54., -13. * l, 156., 22. * l],
        [13. * l, -3. * l * l, 22. * l, 4. * l * l],
    ];
    for (dofs, block) in [(dz, bz), (dy, by)] {
        for i in 0..4 {
            for j in 0..4 {
                m[(dofs[i], dofs[j])] = c * block[i][j];
            }
        }
    }
    m
}

/// Lumped mass: half the member mass per end, translational DOFs only.
/// Rotational DOFs stay massless and drop out of the spectrum.
fn beam_mass_lumped(rho: f64, area: f64, l: f64) -> DMatrix<f64> {
    let mut m = DMatrix::<f64>::zeros(12, 12);
    let half = rho * area * l / 2.;
    for base in [0usize, 6] {
        for axis in 0..3 {
            m[(base + axis, base + axis)] = half;
        }
    }
    m
}

/// One vibration mode, nodal form.
#[derive(Clone, Debug)]
pub struct FrameModalMode {
    pub frequency_hz: f64,
    /// Angular frequency ω = 2πf.
    pub omega_rad_s: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    pub rotations: Vec<[f64; 3]>,
    /// ‖Kφ − ω²Mφ‖∞ / (‖Kφ‖∞ + ‖ω²Mφ‖∞) on the free DOFs.
    pub relative_residual: f64,
}

/// Natural frequencies and mode shapes of the structure.
#[derive(Clone, Debug)]
pub struct FrameModalResponse {
    /// Ascending frequency.
    pub modes: Vec<FrameModalMode>,
    /// Total member mass in tonnes (1 t·mm/s² = 1 N with mm units).
    pub total_mass_t: f64,
    pub free_dofs: usize,
}

/// Small-displacement modal analysis: Kφ = ω²Mφ with the member mass from
/// `densities_t_mm3` (t/mm³; steel ≈ 7.85e-9; zero means a massless member).
/// Supports must be rigid or bidirectional springs — unilateral contacts make
/// the vibration state nonlinear, so they are refused. Massless free DOFs have
/// no finite frequency and do not appear among the modes. Not a certified
/// dynamic calculation.
pub fn modal(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    densities_t_mm3: &[f64],
    mass_model: MassModel,
    modes: usize,
) -> Result<FrameModalResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Modal analysis returns between 1 and 8 modes"));
    }
    if densities_t_mm3.len() != members.len() {
        return Err(invalid("Each member requires a density (0 for massless)"));
    }
    if densities_t_mm3.iter().any(|d| !d.is_finite() || *d < 0.) {
        return Err(invalid("Densities must be finite and nonnegative"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Modal analysis admits rigid restraints and bidirectional springs only",
        ));
    }
    let springs: Vec<(usize, f64)> = supports
        .iter()
        .filter_map(|s| match s {
            Support::Spring {
                node,
                dof,
                stiffness,
            } => Some((node * 6 + dof, *stiffness)),
            _ => None,
        })
        .collect();
    let assembled = assemble(nodes_mm, members, restrained, &springs)?;
    let mats: Vec<DMatrix<f64>> = assembled
        .beams
        .iter()
        .zip(members)
        .zip(densities_t_mm3)
        .map(|((beam, member), &rho)| match mass_model {
            MassModel::Lumped => beam_mass_lumped(rho, member.area_mm2, beam.length),
            MassModel::Consistent => beam_mass_consistent(
                rho,
                member.area_mm2,
                member.iyy_mm4,
                member.izz_mm4,
                beam.length,
            ),
        })
        .collect();
    let mass = assemble_member_matrices(&assembled.beams, &mats, assembled.n)?;
    let total_mass_t: f64 = assembled
        .beams
        .iter()
        .zip(members)
        .zip(densities_t_mm3)
        .map(|((beam, member), &rho)| rho * member.area_mm2 * beam.length)
        .sum();
    // Kφ = ω²Mφ is the shared pencil with geometric := −M; load_factor = ω².
    let negative_mass = -&mass;
    let solved = match &assembled.factor {
        Some(factor) => solve_modes(
            factor,
            &assembled.scales,
            &assembled.free,
            &assembled.stiffness,
            &negative_mass,
            modes,
        )?,
        None => Vec::new(),
    };
    let modes_out = solved
        .into_iter()
        .map(|mode| {
            let omega = mode.load_factor.sqrt();
            FrameModalMode {
                frequency_hz: omega / (2. * std::f64::consts::PI),
                omega_rad_s: omega,
                displacements: (0..assembled.n)
                    .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + k]))
                    .collect(),
                rotations: (0..assembled.n)
                    .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + 3 + k]))
                    .collect(),
                relative_residual: mode.relative_residual,
            }
        })
        .collect();
    Ok(FrameModalResponse {
        modes: modes_out,
        total_mass_t,
        free_dofs: assembled.free.len(),
    })
}

/// Budget of hinge insertions in one collapse analysis.
pub const MAX_PLASTIC_HINGES: usize = 256;

/// One plastic hinge in formation order.
#[derive(Clone, Debug, PartialEq)]
pub struct PlasticHinge {
    pub member: usize,
    /// True when the hinge formed at node A (x = 0), false at node B (x = L).
    pub at_node_a: bool,
    /// Cumulative load factor at which the hinge formed.
    pub load_factor: f64,
}

/// How a collapse analysis terminated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseStatus {
    /// The last hinge insertion turned the structure into a mechanism.
    Mechanism,
    /// The hinge budget ran out while the structure still carried load.
    HingeLimit,
    /// Every yieldable section is released or carries no moment increment:
    /// the remaining structure carries the reference load elastically.
    ElasticUnlimited,
}

/// Incremental plastic collapse under proportional loading.
#[derive(Clone, Debug)]
pub struct FrameCollapseResponse {
    /// Hinges in formation order; simultaneous hinges share a load factor.
    pub hinges: Vec<PlasticHinge>,
    pub status: CollapseStatus,
    /// Collapse load factor (status `Mechanism`), else None.
    pub collapse_load_factor: Option<f64>,
}

/// Step-by-step plastic hinge analysis: the reference load set scales by a
/// single factor λ. Each step solves the current structure elastically, finds
/// the member end whose accumulated bending moment first reaches its plastic
/// moment `plastic_moments_nmm` (None = elastic member), and inserts a bending
/// hinge there (releases about local y and z; torsion stays). The moment at a
/// hinge stays at Mp, so moments accumulate across steps. The loop ends when
/// the structure becomes a mechanism (collapse), when the hinge budget runs
/// out, or when no yieldable section sees any further moment. Hinges form at
/// member ends only; under distributed loads a span interior can govern, so
/// refine the mesh where a mid-span hinge is expected. Supports must be rigid
/// or bidirectional springs — unilateral contacts are refused. Small
/// displacements, elastic-perfectly-plastic sections; not a certified
/// ultimate-limit-state calculation.
pub fn collapse(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    reference: &LoadSet,
    plastic_moments_nmm: &[Option<f64>],
    max_hinges: usize,
) -> Result<FrameCollapseResponse> {
    if plastic_moments_nmm.len() != members.len() {
        return Err(invalid(
            "Each member requires a plastic moment entry (null for elastic)",
        ));
    }
    if plastic_moments_nmm.iter().all(Option::is_none) {
        return Err(invalid("At least one member requires a plastic moment"));
    }
    if plastic_moments_nmm
        .iter()
        .flatten()
        .any(|m| !m.is_finite() || *m <= 0.)
    {
        return Err(invalid("Plastic moments must be finite and positive"));
    }
    if max_hinges == 0 || max_hinges > MAX_PLASTIC_HINGES {
        return Err(invalid(
            "Collapse analysis admits between 1 and 256 hinge insertions",
        ));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Collapse analysis admits rigid restraints and bidirectional springs only",
        ));
    }
    let mut work = members.to_vec();
    // Accumulated end moments per member end: [node A, node B], local y/z.
    let mut cum_my = vec![[0f64; 2]; members.len()];
    let mut cum_mz = vec![[0f64; 2]; members.len()];
    let mut hinged = vec![[false; 2]; members.len()];
    let mut lambda = 0f64;
    let mut hinges = Vec::new();
    loop {
        let model = Model {
            nodes_mm: nodes_mm.to_vec(),
            members: work.clone(),
            restrained: restrained.to_vec(),
            supports: supports.to_vec(),
            forces_n: reference.forces_n.clone(),
            moments_nmm: reference.moments_nmm.clone(),
            loads: reference.loads.clone(),
        };
        let response = match solve(&model) {
            Ok(r) => r,
            Err(err) => {
                if hinges.is_empty() {
                    // Unstable or unsound before any yielding: the caller's error.
                    return Err(err);
                }
                return Ok(FrameCollapseResponse {
                    hinges,
                    status: CollapseStatus::Mechanism,
                    collapse_load_factor: Some(lambda),
                });
            }
        };
        let end_station = |m: usize, end: usize| -> Result<&Station> {
            let stations = &response.members[m].stations;
            let s = if end == 0 {
                stations.first()
            } else {
                stations.last()
            };
            s.ok_or_else(numeric)
        };
        // Next yielding end: smallest λ step to bring |M_cum + Δλ·M| to Mp.
        let mut step_best = f64::INFINITY;
        let mut sites: Vec<(usize, usize)> = Vec::new();
        for (m, mp) in plastic_moments_nmm.iter().enumerate() {
            let Some(&mp) = mp.as_ref() else { continue };
            for end in 0..2 {
                if hinged[m][end] {
                    continue;
                }
                let s = end_station(m, end)?;
                let inc = s.moment_y_nmm.hypot(s.moment_z_nmm);
                if !(inc > 0.) {
                    continue;
                }
                let residual = mp - cum_my[m][end].hypot(cum_mz[m][end]);
                if !(residual > 0.) {
                    continue;
                }
                let step = residual / inc;
                if step < step_best * (1. - 1e-6) {
                    step_best = step;
                    sites.clear();
                    sites.push((m, end));
                } else if step <= step_best * (1. + 1e-6) {
                    sites.push((m, end));
                }
            }
        }
        if sites.is_empty() || !step_best.is_finite() {
            if hinges.is_empty() {
                return Err(invalid(
                    "The reference load produces no bending at yieldable sections",
                ));
            }
            return Ok(FrameCollapseResponse {
                hinges,
                status: CollapseStatus::ElasticUnlimited,
                collapse_load_factor: None,
            });
        }
        lambda += step_best;
        for (m, mp) in plastic_moments_nmm.iter().enumerate() {
            if mp.is_none() {
                continue;
            }
            for end in 0..2 {
                let s = end_station(m, end)?;
                cum_my[m][end] += step_best * s.moment_y_nmm;
                cum_mz[m][end] += step_best * s.moment_z_nmm;
            }
        }
        if hinges.len() + sites.len() > max_hinges {
            return Ok(FrameCollapseResponse {
                hinges,
                status: CollapseStatus::HingeLimit,
                collapse_load_factor: None,
            });
        }
        for (m, end) in sites {
            if end == 0 {
                work[m].release_a[1] = true;
                work[m].release_a[2] = true;
            } else {
                work[m].release_b[1] = true;
                work[m].release_b[2] = true;
            }
            hinged[m][end] = true;
            hinges.push(PlasticHinge {
                member: m,
                at_node_a: end == 0,
                load_factor: lambda,
            });
        }
    }
}

/// Maximum load positions in one influence line request.
pub const MAX_INFLUENCE_POSITIONS: usize = 1024;

/// Member resultant kind for an influence line target, local axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resultant {
    Axial,
    ShearY,
    ShearZ,
    Torsion,
    MomentY,
    MomentZ,
}

/// Response quantity whose influence line is requested.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InfluenceTarget {
    /// Nodal translation (dof 0-2) or rotation (dof 3-5).
    Displacement { node: usize, dof: usize },
    /// Support reaction component; the DOF must be restrained.
    Reaction { node: usize, dof: usize },
    /// Member resultant at `at_mm` from node A, local axes.
    MemberResultant {
        member: usize,
        at_mm: f64,
        resultant: Resultant,
    },
}

/// Influence line values, one per requested load position.
#[derive(Clone, Debug)]
pub struct InfluenceResponse {
    /// Target response under the moving force at each position.
    pub values: Vec<f64>,
}

/// Influence line of one response quantity: the structure is solved exactly
/// for the moving force vector `force_n` placed at every `positions` entry
/// (member, distance from node A), one independent solve per position — no
/// reciprocity shortcuts, so springs and end releases behave identically to
/// `solve`. Member resultants are evaluated exactly at the requested section
/// (not interpolated from the diagram grid). Supports must be rigid or
/// bidirectional springs — unilateral contacts make the response nonlinear.
pub fn influence(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    positions: &[(usize, f64)],
    force_n: [f64; 3],
    target: &InfluenceTarget,
) -> Result<InfluenceResponse> {
    if positions.is_empty() || positions.len() > MAX_INFLUENCE_POSITIONS {
        return Err(invalid("Influence lines admit 1-1024 load positions"));
    }
    if !finite3(&force_n) || force_n.iter().all(|&v| v == 0.) {
        return Err(invalid("The moving force vector must be finite and nonzero"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Influence lines admit rigid restraints and bidirectional springs only",
        ));
    }
    let springs: Vec<(usize, f64)> = supports
        .iter()
        .filter_map(|s| match s {
            Support::Spring {
                node,
                dof,
                stiffness,
            } => Some((node * 6 + dof, *stiffness)),
            _ => None,
        })
        .collect();
    let assembled = assemble(nodes_mm, members, restrained, &springs)?;
    let lengths: Vec<f64> = assembled.beams.iter().map(|b| b.length).collect();
    let on_member = |m: usize, at: f64| {
        m < members.len() && at.is_finite() && at >= 0. && at <= lengths[m] * (1. + 1e-9)
    };
    for &(m, at) in positions {
        if !on_member(m, at) {
            return Err(invalid("Load positions must lie on their member"));
        }
    }
    match *target {
        InfluenceTarget::Displacement { node, dof } => {
            if node >= nodes_mm.len() || dof >= 6 {
                return Err(invalid("Displacement target needs node < nodes, dof < 6"));
            }
        }
        InfluenceTarget::Reaction { node, dof } => {
            if node >= nodes_mm.len() || dof >= 6 {
                return Err(invalid("Reaction target needs node < nodes, dof < 6"));
            }
            if !restrained[node][dof] {
                return Err(invalid("Reaction influence requires a restrained DOF"));
            }
        }
        InfluenceTarget::MemberResultant {
            member, at_mm, ..
        } => {
            if !on_member(member, at_mm) {
                return Err(invalid("The target section must lie on its member"));
            }
        }
    }
    let mut values = Vec::with_capacity(positions.len());
    for &(m, at) in positions {
        let at = at.min(lengths[m]);
        let load_set = LoadSet {
            forces_n: vec![[0.; 3]; nodes_mm.len()],
            moments_nmm: vec![[0.; 3]; nodes_mm.len()],
            loads: vec![MemberLoad::PointForce {
                member: m,
                at_mm: at,
                force_n,
                local_axes: false,
            }],
        };
        let response = solve_case(&assembled, &load_set)?;
        let value = match *target {
            InfluenceTarget::Displacement { node, dof } => {
                if dof < 3 {
                    response.displacements_mm[node][dof]
                } else {
                    response.rotations_rad[node][dof - 3]
                }
            }
            InfluenceTarget::Reaction { node, dof } => {
                if dof < 3 {
                    response.reactions_n[node][dof]
                } else {
                    response.reaction_moments_nmm[node][dof - 3]
                }
            }
            InfluenceTarget::MemberResultant {
                member,
                at_mm,
                resultant,
            } => {
                // Recover the target member's end forces and evaluate the
                // resultant exactly at the requested section.
                let forces = prepare_loads(&assembled.beams, &load_set, assembled.n)?;
                let beam = &assembled.beams[member];
                let mut displacement = DVector::<f64>::zeros(assembled.n * 6);
                for i in 0..assembled.n {
                    for c in 0..3 {
                        displacement[i * 6 + c] = response.displacements_mm[i][c];
                        displacement[i * 6 + 3 + c] = response.rotations_rad[i][c];
                    }
                }
                let t = transformation(&beam.rotation);
                let global_dof = |local: usize| beam.nodes[local / 6] * 6 + local % 6;
                let mut u_glob = DVector::<f64>::zeros(12);
                for d in 0..12 {
                    u_glob[d] = displacement[global_dof(d)];
                }
                let mut u_local = t.transpose() * u_glob;
                if !beam.released.is_empty() {
                    let chol = beam.k_rr.as_ref().expect("validated condensation");
                    let u_k = subvector(&u_local, &beam.kept);
                    let f_r = subvector(&forces[member].f_eq, &beam.released);
                    let u_r = chol.solve(&(f_r - &beam.k_rk * u_k));
                    for (i, &d) in beam.released.iter().enumerate() {
                        u_local[d] = u_r[i];
                    }
                }
                let q = &beam.k_local * &u_local - &forces[member].f_eq;
                let mut end_on_element = [0.; 6];
                end_on_element.copy_from_slice(&q.as_slice()[..6]);
                let s = station_resultants(beam, &forces[member], &end_on_element, at_mm.min(lengths[member]));
                match resultant {
                    Resultant::Axial => s.axial_n,
                    Resultant::ShearY => s.shear_y_n,
                    Resultant::ShearZ => s.shear_z_n,
                    Resultant::Torsion => s.torsion_nmm,
                    Resultant::MomentY => s.moment_y_nmm,
                    Resultant::MomentZ => s.moment_z_nmm,
                }
            }
        };
        if !value.is_finite() {
            return Err(numeric());
        }
        values.push(value);
    }
    Ok(InfluenceResponse { values })
}

/// Solve only an admitted, numerically stable frame model. Errors do not carry
/// successful-looking displacement, reaction, or diagram values.
fn solve_case(a: &Assembled, load_set: &LoadSet) -> Result<Response> {
    let forces = prepare_loads(&a.beams, load_set, a.n)?;
    let n = a.n;
    let ndof = n * 6;
    let mut load = DVector::<f64>::zeros(ndof);
    for i in 0..n {
        for c in 0..3 {
            load[i * 6 + c] = load_set.forces_n[i][c];
            load[i * 6 + 3 + c] = load_set.moments_nmm[i][c];
        }
    }
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    for (beam, bf) in a.beams.iter().zip(&forces) {
        let t = transformation(&beam.rotation);
        let mut f_full = DVector::<f64>::zeros(12);
        for (i, &di) in beam.kept.iter().enumerate() {
            f_full[di] = bf.f_eff[i];
        }
        let f_glob = &t * &f_full;
        if f_glob.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            load[global_dof(beam, d1)] += f_glob[d1];
        }
    }
    let restrained = |dof: usize| a.restrained[dof / 6][dof % 6];
    let mut displacement = DVector::<f64>::zeros(ndof);
    if let Some(factor) = &a.factor {
        let rhs = DVector::from_iterator(
            a.free.len(),
            a.free
                .iter()
                .enumerate()
                .map(|(i, &dof)| load[dof] / a.scales[i]),
        );
        if rhs.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let solved = factor.solve(&rhs);
        for (i, &dof) in a.free.iter().enumerate() {
            displacement[dof] = solved[i] / a.scales[i];
        }
        if displacement.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
    }

    // Normwise backward error and reactions, same policy as the truss solver.
    let displacement_norm = displacement.iter().map(|x| x.abs()).fold(0., f64::max);
    let force_norm = load.iter().map(|x| x.abs()).fold(0., f64::max);
    let system_scale = a.matrix_norm * displacement_norm + force_norm;
    if !system_scale.is_finite() {
        return Err(numeric());
    }
    let mut reaction = DVector::<f64>::zeros(ndof);
    let mut max_relative_residual = 0f64;
    for i in 0..ndof {
        let mut residual = -load[i];
        let mut scale = load[i].abs();
        for j in 0..ndof {
            let contribution = a.stiffness[(i, j)] * displacement[j];
            residual += contribution;
            scale += contribution.abs();
        }
        if !residual.is_finite() || !scale.is_finite() {
            return Err(numeric());
        }
        if restrained(i) {
            reaction[i] = residual;
        } else {
            let relative = if system_scale == 0. {
                0.
            } else {
                residual.abs() / system_scale
            };
            max_relative_residual = max_relative_residual.max(relative);
        }
    }
    if max_relative_residual > MAX_RELATIVE_RESIDUAL {
        return Err(Error::new(
            "FRAME_RESIDUAL",
            "Frame solution does not satisfy free-DOF equilibrium",
        ));
    }

    // Recover member end forces and section diagrams in local axes.
    let t_cache: Vec<DMatrix<f64>> = a
        .beams
        .iter()
        .map(|b| transformation(&b.rotation))
        .collect();
    let mut member_results = Vec::with_capacity(a.beams.len());
    for ((beam, bf), t) in a.beams.iter().zip(&forces).zip(&t_cache) {
        let mut u_glob = DVector::<f64>::zeros(12);
        for d in 0..12 {
            u_glob[d] = displacement[global_dof(beam, d)];
        }
        let mut u_local = t.transpose() * u_glob;
        if !beam.released.is_empty() {
            let chol = beam.k_rr.as_ref().expect("validated condensation");
            let u_k = subvector(&u_local, &beam.kept);
            let f_r = subvector(&bf.f_eq, &beam.released);
            let u_r = chol.solve(&(f_r - &beam.k_rk * u_k));
            for (i, &d) in beam.released.iter().enumerate() {
                u_local[d] = u_r[i];
            }
        }
        // Force the element exerts back on its nodes, local axes; the equal
        // and opposite force is what node A applies to the element end.
        let q = &beam.k_local * &u_local - &bf.f_eq;
        let mut end_on_element = [0.; 6];
        for (c, value) in end_on_element.iter_mut().enumerate() {
            *value = q[c];
        }
        let stations: Vec<Station> = (0..DIAGRAM_STATIONS)
            .map(|i| {
                station_resultants(
                    beam,
                    bf,
                    &end_on_element,
                    beam.length * i as f64 / (DIAGRAM_STATIONS - 1) as f64,
                )
            })
            .collect();
        if stations.iter().any(|s| {
            ![
                s.axial_n,
                s.shear_y_n,
                s.shear_z_n,
                s.torsion_nmm,
                s.moment_y_nmm,
                s.moment_z_nmm,
            ]
            .iter()
            .all(|v| v.is_finite())
        }) {
            return Err(numeric());
        }
        member_results.push(MemberResult { stations });
    }

    let displacements_mm: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| displacement[i * 6 + k]))
        .collect();
    let rotations_rad: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| displacement[i * 6 + 3 + k]))
        .collect();
    let reactions_n: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| reaction[i * 6 + k]))
        .collect();
    let reaction_moments_nmm: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| reaction[i * 6 + 3 + k]))
        .collect();
    let max_deflection_mm = displacements_mm
        .iter()
        .map(|d| d[0].hypot(d[1]).hypot(d[2]))
        .fold(0., f64::max);
    if !max_deflection_mm.is_finite() {
        return Err(numeric());
    }
    Ok(Response {
        displacements_mm,
        rotations_rad,
        reactions_n,
        reaction_moments_nmm,
        members: member_results,
        max_deflection_mm,
        max_relative_residual,
        free_dofs: a.free.len(),
    })
}

/// Validate support conditions against the structure and rigid restraints.
fn validate_supports(supports: &[Support], n: usize, restrained: &[[bool; 6]]) -> Result<()> {
    if supports.len() > MAX_SUPPORTS {
        return Err(invalid("Supports are capped at 6 per node"));
    }
    let mut seen = BTreeSet::new();
    for support in supports {
        let (node, dof) = support.node_dof();
        if node >= n || dof >= 6 {
            return Err(invalid("Support references an unknown node or DOF"));
        }
        if restrained[node][dof] {
            return Err(invalid("Support duplicates a rigid restraint"));
        }
        if !seen.insert((node, dof)) {
            return Err(invalid("Duplicate support on one DOF"));
        }
        if let Some(k) = support.stiffness() {
            if !k.is_finite() || k <= 0. {
                return Err(invalid("Spring stiffness must be finite and positive"));
            }
        }
    }
    Ok(())
}

/// Reaction a support exerts on the structure, read back from a response.
fn support_reaction(r: &Response, node: usize, dof: usize) -> f64 {
    if dof < 3 {
        r.reactions_n[node][dof]
    } else {
        r.reaction_moments_nmm[node][dof - 3]
    }
}

fn support_displacement(r: &Response, node: usize, dof: usize) -> f64 {
    if dof < 3 {
        r.displacements_mm[node][dof]
    } else {
        r.rotations_rad[node][dof - 3]
    }
}

fn set_support_reaction(r: &mut Response, node: usize, dof: usize, value: f64) {
    if dof < 3 {
        r.reactions_n[node][dof] = value;
    } else {
        r.reaction_moments_nmm[node][dof - 3] = value;
    }
}

/// Solve one load set with springs and unilateral contacts. Unilateral
/// supports iterate an active set: engaged bounds contribute their DOF to the
/// restraint mask, engaged unilateral springs contribute stiffness, and the
/// set is updated from reaction signs (release) and penetrations (engagement)
/// until it stabilizes.
fn solve_supported(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    load_set: &LoadSet,
) -> Result<Response> {
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    if supports.is_empty() {
        let assembled = assemble(nodes_mm, members, restrained, &[])?;
        return solve_case(&assembled, load_set);
    }
    // Every unilateral support starts engaged (contact assumed): engagement
    // never destabilizes, while a support left out of the first solve could
    // leave the structure a mechanism. Disengagement follows from reaction
    // signs, re-engagement from penetration.
    let mut active: Vec<bool> = vec![true; supports.len()];
    let max_iterations = 6 + 2 * supports.len();
    let mut mask;
    let mut diagonal: Vec<(usize, f64)> = Vec::new();
    let mut iteration = 0;
    let response = loop {
        iteration += 1;
        if iteration > max_iterations {
            return Err(Error::new(
                "FRAME_NO_CONVERGENCE",
                "Unilateral support active set did not stabilize",
            ));
        }
        mask = restrained.to_vec();
        diagonal.clear();
        for (support, &on) in supports.iter().zip(&active) {
            let (node, dof) = support.node_dof();
            match (*support, on) {
                (Support::Spring { stiffness, .. }, _) => {
                    diagonal.push((node * 6 + dof, stiffness));
                }
                (Support::LowerBound { .. } | Support::UpperBound { .. }, true) => {
                    mask[node][dof] = true;
                }
                (Support::LowerSpring { stiffness, .. } | Support::UpperSpring { stiffness, .. }, true) => {
                    diagonal.push((node * 6 + dof, stiffness));
                }
                _ => {}
            }
        }
        let assembled = assemble(nodes_mm, members, &mask, &diagonal)?;
        let r = solve_case(&assembled, load_set)?;
        if !supports.iter().any(|s| s.is_unilateral()) {
            break r;
        }
        let u_scale = r
            .displacements_mm
            .iter()
            .chain(&r.rotations_rad)
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        let r_scale = r
            .reactions_n
            .iter()
            .chain(&r.reaction_moments_nmm)
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        let u_tol = 1e-9 * (1. + u_scale);
        let r_tol = 1e-9 * (1. + r_scale);
        let mut changed = false;
        for (i, support) in supports.iter().enumerate() {
            let (node, dof) = support.node_dof();
            let u = support_displacement(&r, node, dof);
            let reaction = support_reaction(&r, node, dof);
            let next = match *support {
                Support::LowerBound { .. } => {
                    if active[i] {
                        reaction >= -r_tol // prop cannot pull: release
                    } else {
                        u < -u_tol // penetration: re-engage
                    }
                }
                Support::UpperBound { .. } => {
                    if active[i] {
                        reaction <= r_tol
                    } else {
                        u > u_tol
                    }
                }
                Support::LowerSpring { .. } => u < -u_tol,
                Support::UpperSpring { .. } => u > u_tol,
                Support::Spring { .. } => active[i],
            };
            if next != active[i] {
                active[i] = next;
                changed = true;
            }
        }
        if !changed {
            break r;
        }
    };
    let mut response = response;
    // Report spring forces as support reactions (spring DOFs are free in the
    // factorization, so the residual there is zero by construction).
    for (support, &on) in supports.iter().zip(&active) {
        let (node, dof) = support.node_dof();
        let k = match support {
            Support::Spring { stiffness, .. } => Some(*stiffness),
            Support::LowerSpring { stiffness, .. } | Support::UpperSpring { stiffness, .. } => {
                on.then_some(*stiffness)
            }
            _ => None,
        };
        if let Some(k) = k {
            let u = support_displacement(&response, node, dof);
            set_support_reaction(&mut response, node, dof, -k * u);
        }
    }
    Ok(response)
}

pub fn solve(model: &Model) -> Result<Response> {
    solve_supported(
        &model.nodes_mm,
        &model.members,
        &model.restrained,
        &model.supports,
        &LoadSet {
            forces_n: model.forces_n.clone(),
            moments_nmm: model.moments_nmm.clone(),
            loads: model.loads.clone(),
        },
    )
}

const STATION_SCALARS: usize = 6;

/// Flat scalar layout for envelopes: per node [u, r, R, M] (12), then per
/// member per station [N, Vy, Vz, T, My, Mz]. Station positions are excluded
/// (constant for a given structure).
fn flatten_response(r: &Response, n: usize) -> Vec<f64> {
    let mut out =
        Vec::with_capacity(n * 12 + r.members.len() * DIAGRAM_STATIONS * STATION_SCALARS);
    for i in 0..n {
        out.extend_from_slice(&r.displacements_mm[i]);
        out.extend_from_slice(&r.rotations_rad[i]);
        out.extend_from_slice(&r.reactions_n[i]);
        out.extend_from_slice(&r.reaction_moments_nmm[i]);
    }
    for member in &r.members {
        for s in &member.stations {
            out.extend_from_slice(&[
                s.axial_n,
                s.shear_y_n,
                s.shear_z_n,
                s.torsion_nmm,
                s.moment_y_nmm,
                s.moment_z_nmm,
            ]);
        }
    }
    out
}

/// Scale a member load's vectors by a combination factor; position and axes
/// are preserved, so a scaled load remains an admitted load.
fn scale_member_load(load: &MemberLoad, factor: f64) -> MemberLoad {
    let s = |v: [f64; 3]| [v[0] * factor, v[1] * factor, v[2] * factor];
    match *load {
        MemberLoad::PointForce {
            member,
            at_mm,
            force_n,
            local_axes,
        } => MemberLoad::PointForce {
            member,
            at_mm,
            force_n: s(force_n),
            local_axes,
        },
        MemberLoad::PointMoment {
            member,
            at_mm,
            moment_nmm,
            local_axes,
        } => MemberLoad::PointMoment {
            member,
            at_mm,
            moment_nmm: s(moment_nmm),
            local_axes,
        },
        MemberLoad::Uniform {
            member,
            force_n_per_mm,
            local_axes,
        } => MemberLoad::Uniform {
            member,
            force_n_per_mm: s(force_n_per_mm),
            local_axes,
        },
        MemberLoad::Trapezoidal {
            member,
            from_n_per_mm,
            to_n_per_mm,
            local_axes,
        } => MemberLoad::Trapezoidal {
            member,
            from_n_per_mm: s(from_n_per_mm),
            to_n_per_mm: s(to_n_per_mm),
            local_axes,
        },
    }
}

/// Merge load cases into one combination load set: nodal loads scale
/// elementwise, member loads scale their vectors and concatenate.
fn combine_load_sets(cases: &[LoadSet], factors: &[f64], n: usize) -> Result<LoadSet> {
    let mut forces_n = vec![[0.; 3]; n];
    let mut moments_nmm = vec![[0.; 3]; n];
    let mut loads = Vec::new();
    for (case, &factor) in cases.iter().zip(factors) {
        if factor == 0. {
            continue;
        }
        for i in 0..n {
            for c in 0..3 {
                forces_n[i][c] += factor * case.forces_n[i][c];
                moments_nmm[i][c] += factor * case.moments_nmm[i][c];
            }
        }
        for load in &case.loads {
            loads.push(scale_member_load(load, factor));
        }
    }
    if loads.len() > MAX_COMBINED_MEMBER_LOADS {
        return Err(invalid("Combined member loads exceed the 4096 budget"));
    }
    if forces_n.iter().chain(&moments_nmm).any(|v| !finite3(v)) {
        return Err(numeric());
    }
    Ok(LoadSet {
        forces_n,
        moments_nmm,
        loads,
    })
}

/// Solve load cases against one shared structure and envelope the responses
/// across combinations. Two exact paths: with only rigid restraints and
/// bidirectional springs the response is linear, so the stiffness is factored
/// once and combinations are formed from the case responses; with unilateral
/// supports superposition does not hold, so every combination is merged into
/// one load set and solved directly with the active-set iteration. Errors
/// from any case abort the whole request; envelopes never mix with
/// successful-looking partial data.
pub fn solve_envelopes(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    cases: &[LoadSet],
    combinations: &[Combination],
) -> Result<EnvelopeResponse> {
    crate::combos::validate(combinations, cases.len())?;
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    let width = n * 12 + members.len() * DIAGRAM_STATIONS * STATION_SCALARS;
    // One flat response vector per combination, exact in both paths.
    let mut combined_flat: Vec<Vec<f64>> = Vec::with_capacity(combinations.len());
    let mut max_relative_residual = 0f64;
    let grid_response: Response;
    if supports.iter().any(|s| s.is_unilateral()) {
        let mut grid = None;
        for combination in combinations {
            let merged = combine_load_sets(cases, &combination.factors, n)?;
            let r = solve_supported(nodes_mm, members, restrained, supports, &merged)?;
            max_relative_residual = max_relative_residual.max(r.max_relative_residual);
            if grid.is_none() {
                grid = Some(r.clone());
            }
            combined_flat.push(flatten_response(&r, n));
        }
        grid_response = grid.expect("validated nonempty combinations");
    } else {
        let diagonal: Vec<(usize, f64)> = supports
            .iter()
            .map(|s| {
                let (node, dof) = s.node_dof();
                (node * 6 + dof, s.stiffness().expect("bidirectional spring"))
            })
            .collect();
        let assembled = assemble(nodes_mm, members, restrained, &diagonal)?;
        let responses = cases
            .iter()
            .map(|case| solve_case(&assembled, case))
            .collect::<Result<Vec<_>>>()?;
        max_relative_residual = responses
            .iter()
            .map(|r| r.max_relative_residual)
            .fold(0., f64::max);
        let flat: Vec<Vec<f64>> = responses.iter().map(|r| flatten_response(r, n)).collect();
        for combination in combinations {
            let series: Vec<&[f64]> = flat.iter().map(Vec::as_slice).collect();
            combined_flat.push(crate::combos::combine_series(&series, &combination.factors)?);
        }
        grid_response = responses.into_iter().next().expect("validated nonempty cases");
    }
    let mut envelopes: Vec<MinMax> = Vec::with_capacity(width);
    let mut deflection: Option<MinMax> = None;
    for (ci, combined) in combined_flat.iter().enumerate() {
        if ci == 0 {
            envelopes = combined.iter().map(|&v| MinMax::of(v, 0)).collect();
        } else {
            for (e, &v) in envelopes.iter_mut().zip(combined) {
                e.absorb(v, ci);
            }
        }
        let mut combo_max_deflection = 0f64;
        for node in 0..n {
            let base = node * 12;
            combo_max_deflection = combo_max_deflection.max(
                combined[base]
                    .hypot(combined[base + 1])
                    .hypot(combined[base + 2]),
            );
        }
        if !combo_max_deflection.is_finite() {
            return Err(numeric());
        }
        match &mut deflection {
            None => deflection = Some(MinMax::of(combo_max_deflection, ci)),
            Some(e) => e.absorb(combo_max_deflection, ci),
        }
    }
    let node_env = |offset: usize| -> Vec<[MinMax; 3]> {
        (0..n)
            .map(|i| std::array::from_fn(|k| envelopes[i * 12 + offset + k]))
            .collect()
    };
    let members_env: Vec<Vec<StationEnvelope>> = (0..members.len())
        .map(|mi| {
            (0..DIAGRAM_STATIONS)
                .map(|si| {
                    let b = n * 12 + (mi * DIAGRAM_STATIONS + si) * STATION_SCALARS;
                    StationEnvelope {
                        x_mm: grid_response.members[mi].stations[si].x_mm,
                        axial_n: envelopes[b],
                        shear_y_n: envelopes[b + 1],
                        shear_z_n: envelopes[b + 2],
                        torsion_nmm: envelopes[b + 3],
                        moment_y_nmm: envelopes[b + 4],
                        moment_z_nmm: envelopes[b + 5],
                    }
                })
                .collect()
        })
        .collect();
    Ok(EnvelopeResponse {
        load_cases: cases.len(),
        combinations: combinations.iter().map(|c| c.name.clone()).collect(),
        displacements_mm: node_env(0),
        rotations_rad: node_env(3),
        reactions_n: node_env(6),
        reaction_moments_nmm: node_env(9),
        members: members_env,
        max_deflection_mm: deflection.expect("validated nonempty combinations"),
        max_relative_residual,
        free_dofs: grid_response.free_dofs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: f64 = 200_000.;
    const NU: f64 = 0.3;
    const A: f64 = 100.;
    const I: f64 = 1e6;
    const J: f64 = 2e6;
    const L: f64 = 1000.;
    const G: f64 = E / (2. * (1. + NU));

    fn member(nodes: [usize; 2]) -> Member {
        Member {
            nodes,
            young_mpa: E,
            poisson: NU,
            area_mm2: A,
            iyy_mm4: I,
            izz_mm4: I,
            j_mm4: J,
            shear_area_y_mm2: None,
            shear_area_z_mm2: None,
            local_z_hint: None,
            release_a: [false; 3],
            release_b: [false; 3],
        }
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-9 * b.abs().max(1.), "{a} != {b}");
    }
    fn empty_loads(n: usize) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        (vec![[0.; 3]; n], vec![[0.; 3]; n])
    }

    /// Cantilever along +X, fixed at A, downward tip load at B.
    fn cantilever() -> Model {
        let (mut forces, moments) = empty_loads(2);
        forces[1] = [0., 0., -1000.];
        Model {
            nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
            members: vec![member([0, 1])],
            restrained: vec![[true; 6], [false; 6]],
            supports: vec![],
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![],
        }
    }

    #[test]
    fn cantilever_tip_load_matches_euler_bernoulli() {
        let r = solve(&cantilever()).unwrap();
        let p = 1000.;
        close(r.displacements_mm[1][2], -p * L * L * L / (3. * E * I));
        close(r.rotations_rad[1][1], p * L * L / (2. * E * I));
        close(r.reactions_n[0][2], p);
        close(r.reaction_moments_nmm[0][1], -p * L);
        let stations = &r.members[0].stations;
        close(stations[0].shear_z_n, -p);
        close(stations[0].moment_y_nmm, p * L); // hogging positive
        close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
        close(stations[10].moment_y_nmm, p * L / 2.);
        assert!(r.max_relative_residual < 1e-12);
        assert_eq!(r.free_dofs, 6);
    }

    #[test]
    fn cantilever_shear_area_adds_timoshenko_deflection() {
        let mut model = cantilever();
        model.members[0].shear_area_z_mm2 = Some(500.);
        let r = solve(&model).unwrap();
        let p = 1000.;
        let eb = p * L * L * L / (3. * E * I);
        let shear = p * L / (G * 500.);
        close(r.displacements_mm[1][2], -(eb + shear));
    }

    /// Simply supported span along +X with a downward uniform load.
    fn ss_beam(w: f64) -> Model {
        let (forces, moments) = empty_loads(2);
        Model {
            nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
            members: vec![member([0, 1])],
            // Pins: translations fixed, bending rotation ry free at both ends.
            restrained: vec![
                [true, true, true, true, false, true],
                [false, true, true, false, false, true],
            ],
            supports: vec![],
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![MemberLoad::Uniform {
                member: 0,
                force_n_per_mm: [0., 0., -w],
                local_axes: false,
            }],
        }
    }

    #[test]
    fn simply_supported_udl_matches_closed_form() {
        let w = 10.;
        let r = solve(&ss_beam(w)).unwrap();
        close(r.reactions_n[0][2], w * L / 2.);
        close(r.reactions_n[1][2], w * L / 2.);
        close(r.rotations_rad[0][1], w * L * L * L / (24. * E * I));
        close(r.rotations_rad[1][1], -w * L * L * L / (24. * E * I));
        let stations = &r.members[0].stations;
        close(stations[0].shear_z_n, -w * L / 2.);
        close(stations[10].shear_z_n, 0.);
        close(stations[10].moment_y_nmm, -w * L * L / 8.); // sagging negative
        close(stations[0].moment_y_nmm, 0.);
        close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
        assert!(r.max_relative_residual < 1e-12);
    }

    #[test]
    fn propped_cantilever_udl_two_modeling_routes_agree() {
        let w = 10.;
        // Route 1: fixed at A, roller at B (uz restrained, rotations free).
        let (forces, moments) = empty_loads(2);
        let roller = Model {
            nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
            members: vec![member([0, 1])],
            restrained: vec![[true; 6], [false, true, true, false, false, true]],
            supports: vec![],
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![MemberLoad::Uniform {
                member: 0,
                force_n_per_mm: [0., 0., -w],
                local_axes: true,
            }],
        };
        // Route 2: fixed-fixed with a bending hinge released at B.
        let mut fixed_fixed = roller.clone();
        fixed_fixed.restrained[1] = [true; 6];
        fixed_fixed.members[0].release_b = [false, true, false];
        for model in [&roller, &fixed_fixed] {
            let r = solve(model).unwrap();
            close(r.reactions_n[1][2], 3. * w * L / 8.);
            close(r.reactions_n[0][2], 5. * w * L / 8.);
            let stations = &r.members[0].stations;
            close(stations[0].moment_y_nmm, w * L * L / 8.);
            close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
        }
        // The free-end rotation exists only in the roller model; with B fully
        // restrained the hinge rotation lives in the released element DOF.
        close(
            solve(&roller).unwrap().rotations_rad[1][1],
            -w * L * L * L / (48. * E * I),
        );
    }

    #[test]
    fn point_moment_at_midspan_jumps_the_diagram() {
        let m = 1e6;
        let mut model = ss_beam(0.);
        model.loads = vec![MemberLoad::PointMoment {
            member: 0,
            at_mm: L / 2.,
            moment_nmm: [0., m, 0.],
            local_axes: true,
        }];
        let r = solve(&model).unwrap();
        close(r.reactions_n[0][2], -m / L);
        close(r.reactions_n[1][2], m / L);
        let stations = &r.members[0].stations;
        close(stations[0].moment_y_nmm, 0.);
        close(stations[9].moment_y_nmm, 0.45 * m); // just left of the jump
        close(stations[10].moment_y_nmm, -0.5 * m); // right-side limit
        close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
    }

    #[test]
    fn trapezoidal_load_carries_correct_reactions_and_moment() {
        let w = 10.;
        let mut model = ss_beam(0.);
        model.loads = vec![MemberLoad::Trapezoidal {
            member: 0,
            from_n_per_mm: [0., 0., 0.],
            to_n_per_mm: [0., 0., -w],
            local_axes: true,
        }];
        let r = solve(&model).unwrap();
        close(r.reactions_n[0][2], w * L / 6.);
        close(r.reactions_n[1][2], w * L / 3.);
        let stations = &r.members[0].stations;
        close(stations[0].shear_z_n, -w * L / 6.);
        // M(L/2) = wL²/16 sagging for a 0→w ramp.
        close(stations[10].moment_y_nmm, -w * L * L / 16.);
    }

    #[test]
    fn point_force_on_member_matches_midspan_formulas() {
        let p = 1000.;
        let mut model = ss_beam(0.);
        model.loads = vec![MemberLoad::PointForce {
            member: 0,
            at_mm: L / 2.,
            force_n: [0., 0., -p],
            local_axes: false,
        }];
        let r = solve(&model).unwrap();
        close(r.reactions_n[0][2], p / 2.);
        close(r.reactions_n[1][2], p / 2.);
        close(r.rotations_rad[0][1], p * L * L / (16. * E * I));
        let stations = &r.members[0].stations;
        close(stations[10].moment_y_nmm, -p * L / 4.);
        close(stations[9].shear_z_n, -p / 2.);
        close(stations[10].shear_z_n, p / 2.); // right side of the point load
    }

    #[test]
    fn axial_and_torsion_respond_independently() {
        let (mut forces, mut moments) = empty_loads(2);
        forces[1] = [100., 0., 0.];
        moments[1] = [1e6, 0., 0.];
        let model = Model {
            nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
            members: vec![member([0, 1])],
            restrained: vec![[true; 6], [false; 6]],
            supports: vec![],
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![],
        };
        let r = solve(&model).unwrap();
        close(r.displacements_mm[1][0], 100. * L / (E * A));
        close(r.rotations_rad[1][0], 1e6 * L / (G * J));
        close(r.reactions_n[0][0], -100.);
        close(r.reaction_moments_nmm[0][0], -1e6);
        let stations = &r.members[0].stations;
        close(stations[5].axial_n, 100.); // tension positive
        close(stations[5].torsion_nmm, 1e6);
        close(stations[5].moment_y_nmm, 0.);
    }

    #[test]
    fn rotated_cantilever_preserves_response_in_global_axes() {
        let base = solve(&cantilever()).unwrap();
        // Rotate 90° about Z: X→Y, Y→−X, member along +Y, load stays −Z.
        let mut rotated = cantilever();
        rotated.nodes_mm = vec![[0., 0., 0.], [0., L, 0.]];
        // Local z hint must stay global +Z; local y = z × x = Z × Y = −X.
        let r = solve(&rotated).unwrap();
        close(r.displacements_mm[1][2], base.displacements_mm[1][2]);
        // Bending was about local y (= global Y); now about local y (= −X).
        close(r.rotations_rad[1][0], -base.rotations_rad[1][1]);
        close(r.reactions_n[0][2], base.reactions_n[0][2]);
        close(r.reaction_moments_nmm[0][0], -base.reaction_moments_nmm[0][1]);
    }

    #[test]
    fn refuses_mechanisms_and_unstable_releases() {
        // Cantilever with a ball-joint release at the wall: free rigid rotation.
        let mut model = cantilever();
        model.members[0].release_a = [true, true, true];
        assert_eq!(solve(&model).unwrap_err().code, "FRAME_SINGULAR");
        // Torsion released at both ends: singular released block.
        let mut model = cantilever();
        model.members[0].release_a[0] = true;
        model.members[0].release_b[0] = true;
        assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
    }

    #[test]
    fn rejects_invalid_input_before_matrix_allocation() {
        let mut models = Vec::new();
        let mut m = cantilever();
        m.members[0].poisson = 0.5;
        models.push(m);
        let mut m = cantilever();
        m.members[0].iyy_mm4 = 0.;
        models.push(m);
        let mut m = cantilever();
        m.members[0].shear_area_z_mm2 = Some(-1.);
        models.push(m);
        let mut m = cantilever();
        m.restrained = vec![[true; 6]]; // one mask short
        models.push(m);
        let mut m = cantilever();
        m.moments_nmm.clear();
        models.push(m);
        let mut m = cantilever();
        m.nodes_mm[1][1] = f64::NAN;
        models.push(m);
        let mut m = cantilever();
        m.loads = vec![MemberLoad::Uniform {
            member: 1,
            force_n_per_mm: [0., 0., -1.],
            local_axes: true,
        }];
        models.push(m);
        let mut m = cantilever();
        m.loads = vec![MemberLoad::PointForce {
            member: 0,
            at_mm: L + 1.,
            force_n: [0., 0., -1.],
            local_axes: true,
        }];
        models.push(m);
        let mut m = cantilever();
        m.members.push(m.members[0].clone()); // duplicate edge
        models.push(m);
        for model in models {
            assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
        }
    }

    /// Parallel hints fall back deterministically instead of failing.
    fn solve_ok_hint_parallel() -> Model {
        let mut m = cantilever();
        m.members[0].local_z_hint = Some([1., 0., 0.]);
        m
    }

    #[test]
    fn parallel_local_z_hint_falls_back_and_solves() {
        let r = solve(&solve_ok_hint_parallel()).unwrap();
        close(r.displacements_mm[1][2], -1000. * L * L * L / (3. * E * I));
    }

    #[test]
    fn admits_node_limit_and_member_load_budget() {
        let mut model = cantilever();
        for i in 0..MAX_MEMBER_LOADS {
            model.loads.push(MemberLoad::PointForce {
                member: 0,
                at_mm: L * (i % 10) as f64 / 10.,
                force_n: [0., 0., -1.],
                local_axes: true,
            });
        }
        let r = solve(&model).unwrap();
        assert!(r.max_relative_residual < 1e-12);
        model.loads.push(MemberLoad::Uniform {
            member: 0,
            force_n_per_mm: [0., 0., -1.],
            local_axes: true,
        });
        assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
    }

    /// Case G: tip −1000 N (dead); case Q: tip +600 N (wind uplift).
    fn two_cases() -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, Vec<LoadSet>) {
        let model = cantilever();
        let (mut wind_f, wind_m) = empty_loads(2);
        wind_f[1] = [0., 0., 600.];
        let cases = vec![
            LoadSet {
                forces_n: model.forces_n.clone(),
                moments_nmm: model.moments_nmm.clone(),
                loads: vec![],
            },
            LoadSet {
                forces_n: wind_f,
                moments_nmm: wind_m,
                loads: vec![],
            },
        ];
        (model.nodes_mm, model.members, model.restrained, cases)
    }

    fn named_combos() -> Vec<Combination> {
        ["G", "Q", "G+1.5Q", "G-Q"]
            .into_iter()
            .zip([
                vec![1., 0.],
                vec![0., 1.],
                vec![1., 1.5],
                vec![1., -1.],
            ])
            .map(|(name, factors)| Combination {
                name: name.into(),
                factors,
            })
            .collect()
    }

    #[test]
    fn envelopes_match_combination_arithmetic_and_governing_indices() {
        let (nodes, members, restrained, cases) = two_cases();
        let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &named_combos()).unwrap();
        // Tip uz: G −5/3, Q +1, G+1.5Q −1/6, G−Q −8/3.
        let tip_z = &r.displacements_mm[1][2];
        close(tip_z.min, -8. / 3.);
        close(tip_z.max, 1.);
        assert_eq!(tip_z.min_combination, 3);
        assert_eq!(tip_z.max_combination, 1);
        // Fixed-end reaction moment about y: G −1e6, Q +6e5.
        let my = &r.reaction_moments_nmm[0][1];
        close(my.min, -1.6e6);
        close(my.max, 6e5);
        assert_eq!(my.min_combination, 3);
        assert_eq!(my.max_combination, 1);
        // Root station moment_y: G +1e6, Q −6e5 (hogging positive).
        let root_my = &r.members[0][0].moment_y_nmm;
        close(root_my.min, -6e5);
        close(root_my.max, 1.6e6);
        close(r.members[0][10].x_mm, L / 2.);
        // Combination max deflection: 5/3, 1, 1/6, 8/3.
        close(r.max_deflection_mm.min, 1. / 6.);
        close(r.max_deflection_mm.max, 8. / 3.);
        assert_eq!(r.max_deflection_mm.min_combination, 2);
        assert_eq!(r.max_deflection_mm.max_combination, 3);
        assert_eq!(r.combinations, vec!["G", "Q", "G+1.5Q", "G-Q"]);
        assert_eq!(r.load_cases, 2);
        assert!(r.max_relative_residual < 1e-12);
        assert_eq!(r.free_dofs, 6);
    }

    #[test]
    fn single_case_single_combo_envelope_equals_plain_solve() {
        let (nodes, members, restrained, cases) = two_cases();
        let combos = vec![Combination {
            name: "G only".into(),
            factors: vec![1., 0.],
        }];
        let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &combos).unwrap();
        let plain = solve(&cantilever()).unwrap();
        for node in 0..2 {
            for c in 0..3 {
                let e = r.displacements_mm[node][c];
                assert_eq!(e.min, e.max);
                close(e.min, plain.displacements_mm[node][c]);
            }
        }
        for (mi, member) in r.members.iter().enumerate() {
            for (si, s) in member.iter().enumerate() {
                let p = &plain.members[mi].stations[si];
                close(s.axial_n.min, p.axial_n);
                close(s.moment_y_nmm.max, p.moment_y_nmm);
                assert_eq!(s.shear_z_n.min_combination, 0);
            }
        }
    }

    #[test]
    fn envelopes_reject_bad_combinations_and_bad_cases() {
        let (nodes, members, restrained, cases) = two_cases();
        // Empty cases, empty combos, factor-count mismatch.
        assert!(solve_envelopes(&nodes, &members, &restrained, &[], &[], &named_combos()).is_err());
        assert!(solve_envelopes(&nodes, &members, &restrained, &[], &cases, &[]).is_err());
        let mut bad = named_combos();
        bad[0].factors.push(1.);
        assert_eq!(
            solve_envelopes(&nodes, &members, &restrained, &[], &cases, &bad)
                .unwrap_err()
                .code,
            "COMBO_INVALID_INPUT"
        );
        // A structurally bad case aborts the whole request.
        let mut bad_cases = cases.clone();
        bad_cases[1].forces_n.pop();
        assert_eq!(
            solve_envelopes(&nodes, &members, &restrained, &[], &bad_cases, &named_combos())
                .unwrap_err()
                .code,
            "FRAME_INVALID_INPUT"
        );
    }

    #[test]
    fn envelopes_cover_member_loads_across_cases() {
        let (nodes, members, restrained, cases) = two_cases();
        let mut uniform_case = cases[0].clone();
        uniform_case.forces_n[1] = [0., 0., 0.];
        uniform_case.loads = vec![MemberLoad::Uniform {
            member: 0,
            force_n_per_mm: [0., 0., -10.],
            local_axes: false,
        }];
        let cases = vec![uniform_case, cases[1].clone()];
        let combos = named_combos();
        let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &combos).unwrap();
        // Root moment_y: uniform case +wL²/2 = +5e6, wind case −6e5.
        // G: 5e6, Q: −6e5, G+1.5Q: 4.1e6, G-Q: 5.6e6.
        let root = &r.members[0][0].moment_y_nmm;
        close(root.max, 5.6e6);
        assert_eq!(root.max_combination, 3);
        close(root.min, -6e5);
        assert_eq!(root.min_combination, 1);
    }

    /// Cantilever with a linear spring at the tip (dof z), tip load −P.
    fn spring_propped(k: f64) -> Model {
        let mut m = cantilever();
        m.supports = vec![Support::Spring {
            node: 1,
            dof: 2,
            stiffness: k,
        }];
        m
    }

    #[test]
    fn spring_support_matches_compatibility_closed_form() {
        let k = 0.6;
        let r = solve(&spring_propped(k)).unwrap();
        // Prop force from compatibility: R = P·kL³ / (3EI + kL³).
        let rb = 1000. * k * L.powi(3) / (3. * E * I + k * L.powi(3));
        close(r.reactions_n[1][2], rb);
        close(r.displacements_mm[1][2], -rb / k);
        close(r.reactions_n[0][2], 1000. - rb);
        assert!(r.max_relative_residual < 1e-12);
    }

    #[test]
    fn unilateral_spring_engages_only_under_contact() {
        let k = 0.6;
        let mut down = spring_propped(k);
        down.supports = vec![Support::LowerSpring {
            node: 1,
            dof: 2,
            stiffness: k,
        }];
        let r = solve(&down).unwrap();
        // Downward tip load presses the spring: full spring behavior.
        let rb = 1000. * k * L.powi(3) / (3. * E * I + k * L.powi(3));
        close(r.reactions_n[1][2], rb);
        // Upward load lifts off: the spring vanishes, pure cantilever.
        let mut up = down.clone();
        up.forces_n[1] = [0., 0., 1000.];
        let r = solve(&up).unwrap();
        close(r.reactions_n[1][2], 0.);
        close(r.displacements_mm[1][2], 1000. * L.powi(3) / (3. * E * I));
    }

    /// Cantilever with a rigid prop below the tip, uniform load either way.
    fn ground_propped(w: f64) -> Model {
        let mut m = cantilever();
        m.forces_n[1] = [0., 0., 0.];
        m.loads = vec![MemberLoad::Uniform {
            member: 0,
            force_n_per_mm: [0., 0., w],
            local_axes: false,
        }];
        m.supports = vec![Support::LowerBound { node: 1, dof: 2 }];
        m
    }

    #[test]
    fn lower_bound_engages_and_lifts_off_by_load_direction() {
        // Downward UDL: prop engaged, propped-cantilever closed forms.
        let r = solve(&ground_propped(-10.)).unwrap();
        close(r.reactions_n[1][2], 3. * 10. * L / 8.);
        close(r.displacements_mm[1][2], 0.);
        close(r.members[0].stations[0].moment_y_nmm, 10. * L * L / 8.);
        // Upward UDL: the prop cannot pull, the contact opens; pure cantilever.
        let r = solve(&ground_propped(10.)).unwrap();
        close(r.reactions_n[1][2], 0.);
        close(r.displacements_mm[1][2], 10. * L.powi(4) / (8. * E * I));
        close(r.members[0].stations[0].moment_y_nmm, -10. * L * L / 2.);
    }

    #[test]
    fn support_validation_rejects_bad_conditions() {
        // On a rigidly restrained DOF.
        let mut m = spring_propped(1.);
        m.supports = vec![Support::LowerBound { node: 0, dof: 2 }];
        assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
        // Duplicate DOF.
        m.supports = vec![
            Support::Spring {
                node: 1,
                dof: 2,
                stiffness: 1.,
            },
            Support::LowerBound { node: 1, dof: 2 },
        ];
        assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
        // Bad stiffness, DOF, node.
        for supports in [
            vec![Support::Spring {
                node: 1,
                dof: 2,
                stiffness: 0.,
            }],
            vec![Support::UpperSpring {
                node: 1,
                dof: 2,
                stiffness: f64::NAN,
            }],
            vec![Support::LowerBound { node: 1, dof: 6 }],
            vec![Support::LowerBound { node: 7, dof: 0 }],
        ] {
            m.supports = supports;
            assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
        }
    }

    #[test]
    fn envelope_with_unilateral_support_solves_combinations_directly() {
        // Prop below the tip; case G presses down, case Q lifts harder.
        let down = ground_propped(-10.);
        let up = ground_propped(20.);
        let cases = vec![
            LoadSet {
                forces_n: down.forces_n.clone(),
                moments_nmm: down.moments_nmm.clone(),
                loads: down.loads.clone(),
            },
            LoadSet {
                forces_n: up.forces_n.clone(),
                moments_nmm: up.moments_nmm.clone(),
                loads: up.loads.clone(),
            },
        ];
        let combos = ["G", "Q", "G+Q"]
            .into_iter()
            .zip([vec![1., 0.], vec![0., 1.], vec![1., 1.]])
            .map(|(name, factors)| Combination {
                name: name.into(),
                factors,
            })
            .collect::<Vec<_>>();
        let r = solve_envelopes(
            &down.nodes_mm,
            &down.members,
            &down.restrained,
            &down.supports,
            &cases,
            &combos,
        )
        .unwrap();
        // G: prop engaged, R_B = 3wL/8 = 3750. Q and G+Q: net uplift, contact
        // open, R_B = 0. Superposing case responses would wrongly give 3750
        // for G+Q; the direct solve gives 0.
        let prop = &r.reactions_n[1][2];
        close(prop.max, 3750.);
        assert_eq!(prop.max_combination, 0);
        close(prop.min, 0.);
        // Tip u_z: G 0, Q 2·6.25, G+Q 6.25.
        let tip = &r.displacements_mm[1][2];
        close(tip.min, 0.);
        assert_eq!(tip.min_combination, 0);
        close(tip.max, 20. * L.powi(4) / (8. * E * I));
        assert_eq!(tip.max_combination, 1);
        // Root moment_y: G +wL²/8, Q −wL²/2, G+Q −5e6.
        let root = &r.members[0][0].moment_y_nmm;
        close(root.max, 10. * L * L / 8.);
        close(root.min, -20. * L * L / 2.);
        assert!(r.max_relative_residual < 1e-12);
    }

    #[test]
    fn diagnose_names_released_rotations_at_a_hinge_node() {
        // Two spans along +X, fixed far ends, every rotation released where the
        // members meet: node 1 rotations have no stiffness at all.
        let mut first = member([0, 1]);
        first.release_b = [true; 3];
        let mut second = member([1, 2]);
        second.release_a = [true; 3];
        let (forces, moments) = empty_loads(3);
        let model = Model {
            nodes_mm: vec![[0., 0., 0.], [L, 0., 0.], [2. * L, 0., 0.]],
            members: vec![first, second],
            restrained: vec![[true; 6], [false; 6], [true; 6]],
            supports: vec![],
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![],
        };
        let error = solve(&model).unwrap_err();
        assert_eq!(error.code, "FRAME_SINGULAR");
        assert!(error.contains("node 1 rx unrestrained"));
        assert!(error.contains("node 1 ry unrestrained"));
        assert!(error.contains("node 1 rz unrestrained"));
        let report = diagnose(&model.nodes_mm, &model.members, &model.restrained, &model.supports)
            .unwrap();
        assert!(!report.is_stable());
        assert_eq!(report.min_normalized_pivot, None);
        assert_eq!(
            report.issues,
            (3..6)
                .map(|dof| crate::diagnostics::DofDiagnosis {
                    node: 1,
                    dof,
                    dof_name: DOF_NAMES[dof],
                    issue: crate::diagnostics::DofIssue::Unrestrained,
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn diagnose_names_rigid_body_mechanism_and_stable_margin() {
        let mut free_beam = cantilever();
        free_beam.restrained = vec![[false; 6]; 2];
        let error = solve(&free_beam).unwrap_err();
        assert_eq!(error.code, "FRAME_SINGULAR");
        assert!(error.contains("in a mechanism"));
        assert!(error.contains("normalized pivot"));
        let report = diagnose(
            &free_beam.nodes_mm,
            &free_beam.members,
            &free_beam.restrained,
            &free_beam.supports,
        )
        .unwrap();
        assert!(!report.is_stable());
        assert!(
            report
                .issues
                .iter()
                .all(|d| d.issue == crate::diagnostics::DofIssue::Mechanism)
        );
        assert!(report.min_normalized_pivot.unwrap() <= 1e-12);
        // Six springs standing in for the fixed end stabilize the beam.
        let springy = diagnose(
            &free_beam.nodes_mm,
            &free_beam.members,
            &free_beam.restrained,
            &(0..6)
                .map(|dof| Support::Spring {
                    node: 0,
                    dof,
                    stiffness: 1e9,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(springy.is_stable());
        let fixed = diagnose(
            &cantilever().nodes_mm,
            &cantilever().members,
            &cantilever().restrained,
            &[],
        )
        .unwrap();
        assert!(fixed.is_stable());
        assert!(fixed.min_normalized_pivot.unwrap() > 1e-12);
    }

    /// Four-element column along +X under a compressive (−x) tip load.
    fn column(restrained: Vec<[bool; 6]>, tip_force: f64) -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, LoadSet) {
        let nodes_mm: Vec<[f64; 3]> = (0..=4).map(|i| [i as f64 * L / 4., 0., 0.]).collect();
        let members = (0..4).map(|i| member([i, i + 1])).collect();
        let (mut forces, moments) = empty_loads(5);
        forces[4] = [tip_force, 0., 0.];
        (
            nodes_mm,
            members,
            restrained,
            LoadSet {
                forces_n: forces,
                moments_nmm: moments,
                loads: vec![],
            },
        )
    }

    #[test]
    fn buckling_cantilever_column_matches_euler() {
        let (nodes, members, restrained, reference) =
            column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), -1000.);
        let r = buckling(&nodes, &members, &restrained, &[], &reference, 4).unwrap();
        assert_eq!(r.modes.len(), 4);
        for axial in &r.axial_forces_n {
            close(*axial, -1000.);
        }
        // P_cr = π²EI/(4L²); Iyy = Izz, so the first two modes are a
        // degenerate pair buckling in the two lateral planes.
        let p_cr = std::f64::consts::PI.powi(2) * E * I / (4. * L * L);
        let lambda = p_cr / 1000.;
        for mode in &r.modes[..2] {
            assert!(
                (mode.load_factor - lambda).abs() < 0.005 * lambda,
                "{} vs {lambda}",
                mode.load_factor
            );
            assert!(mode.relative_residual < 1e-8);
            let peak = mode
                .displacements
                .iter()
                .chain(&mode.rotations)
                .flatten()
                .map(|v| v.abs())
                .fold(0., f64::max);
            assert_eq!(peak, 1.);
        }
        // Second cantilever mode: (4.694/1.875)² ≈ 6.27 times the first.
        assert!(r.modes[2].load_factor > 5. * lambda);
    }

    #[test]
    fn buckling_pinned_pinned_column_matches_euler_and_tension_flips_sign() {
        let restrained = vec![
            // Bending rotations free, torsion fixed: a torsion chain free at
            // both ends would be a rigid spin mode, not a pinned-pinned strut.
            [true, true, true, true, false, false],
            [false; 6],
            [false; 6],
            [false; 6],
            // Roller: axial DOF free so the tip load compresses the column.
            [false, true, true, false, false, false],
        ];
        let (nodes, members, restrained, reference) = column(restrained, -1000.);
        let r = buckling(&nodes, &members, &restrained, &[], &reference, 2).unwrap();
        let p_cr = std::f64::consts::PI.powi(2) * E * I / (L * L);
        let lambda = p_cr / 1000.;
        assert!((r.modes[0].load_factor - lambda).abs() < 0.005 * lambda);
        // The same column under tension buckles only under the reversed load.
        let (nodes, members, restrained, tension) = column(restrained.clone(), 1000.);
        let r = buckling(&nodes, &members, &restrained, &[], &tension, 2).unwrap();
        for axial in &r.axial_forces_n {
            close(*axial, 1000.);
        }
        for mode in &r.modes {
            assert!(mode.load_factor < 0.);
        }
        assert!((r.modes[0].load_factor + lambda).abs() < 0.005 * lambda);
    }

    #[test]
    fn buckling_rejects_unilateral_supports_bad_mode_counts_and_singular() {
        let (nodes, members, restrained, reference) =
            column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), -1000.);
        for bad_modes in [0, MAX_MODES + 1] {
            assert_eq!(
                buckling(&nodes, &members, &restrained, &[], &reference, bad_modes)
                    .unwrap_err()
                    .code,
                "FRAME_INVALID_INPUT"
            );
        }
        assert_eq!(
            buckling(
                &nodes,
                &members,
                &restrained,
                &[Support::LowerBound { node: 4, dof: 1 }],
                &reference,
                1,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
        let free: Vec<[bool; 6]> = vec![[false; 6]; 5];
        assert_eq!(
            buckling(&nodes, &members, &free, &[], &reference, 1)
                .unwrap_err()
                .code,
            "FRAME_SINGULAR"
        );
    }

    #[test]
    fn modal_cantilever_matches_beam_theory_and_scales_with_density() {
        let (nodes, members, restrained, _) =
            column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), 0.);
        let densities = [8e-9; 4];
        let r = modal(&nodes, &members, &restrained, &[], &densities, MassModel::Consistent, 5)
            .unwrap();
        assert_eq!(r.modes.len(), 5);
        close(r.total_mass_t, 8e-9 * A * L);
        // Spectrum of a cantilever with Iyy = Izz: a degenerate bending pair at
        // f1 = β₁²/(2π)·√(EI/(ρA))/L² (β₁ = 1.8751), then torsion
        // f_t = √((GJ)/(ρIp))/(4L), axial f_a = √(E/ρ)/(4L), then the second
        // bending pair at β₂ = 4.6941.
        let omega1 = 1.8751f64.powi(2) * (E * I / (8e-9 * A)).sqrt() / (L * L);
        let f1 = omega1 / (2. * std::f64::consts::PI);
        // f_t = c_t/(4L) and f_a = c_a/(4L) with wave speeds √(GJ/(ρIp)) and
        // √(E/ρ) (fixed-free rod: ω₁ = πc/(2L)).
        let f_torsion = (G * J / (8e-9 * 2. * I)).sqrt() / (4. * L);
        let f_axial = (E / 8e-9f64).sqrt() / (4. * L);
        let f2 = (4.6941f64 / 1.8751).powi(2) * f1;
        for (mode, (expected, tol)) in r
            .modes
            .iter()
            .zip([(f1, 0.005), (f1, 0.005), (f_torsion, 0.02), (f_axial, 0.02), (f2, 0.01)])
        {
            assert!(
                (mode.frequency_hz - expected).abs() < tol * expected,
                "{} vs {expected}",
                mode.frequency_hz
            );
            assert!(mode.relative_residual < 1e-8);
        }
        close(r.modes[0].omega_rad_s, 2. * std::f64::consts::PI * r.modes[0].frequency_hz);
        // Mode characters: bending is lateral, torsion is rx, axial is x.
        let lateral_peak = |m: &FrameModalMode| {
            m.displacements
                .iter()
                .map(|d| d[1].abs().max(d[2].abs()))
                .fold(0., f64::max)
        };
        assert_eq!(lateral_peak(&r.modes[0]), 1.);
        assert_eq!(lateral_peak(&r.modes[1]), 1.);
        assert_eq!(
            r.modes[2]
                .rotations
                .iter()
                .map(|r| r[0].abs())
                .fold(0., f64::max),
            1.
        );
        assert_eq!(
            r.modes[3]
                .displacements
                .iter()
                .map(|d| d[0].abs())
                .fold(0., f64::max),
            1.
        );
        // Doubling every density divides all frequencies by √2 exactly.
        let heavy = modal(
            &nodes,
            &members,
            &restrained,
            &[],
            &[16e-9; 4],
            MassModel::Consistent,
            4,
        )
        .unwrap();
        for (a, b) in r.modes.iter().zip(&heavy.modes) {
            close(a.frequency_hz / 2f64.sqrt(), b.frequency_hz);
        }
        // Lumped mass lands in the same neighborhood, converging from below.
        let lumped = modal(&nodes, &members, &restrained, &[], &densities, MassModel::Lumped, 1)
            .unwrap();
        let f1_lumped = lumped.modes[0].frequency_hz;
        assert!(f1_lumped > 0.85 * f1 && f1_lumped < 1.05 * f1, "{f1_lumped} vs {f1}");
        // Massless members leave no finite-frequency modes.
        let massless = modal(&nodes, &members, &restrained, &[], &[0.; 4], MassModel::Lumped, 2)
            .unwrap();
        assert!(massless.modes.is_empty());
        assert_eq!(massless.total_mass_t, 0.);
    }

    #[test]
    fn modal_validates_input_and_refuses_unilateral_supports() {
        let (nodes, members, restrained, _) =
            column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), 0.);
        let densities = [8e-9; 4];
        for bad in [0, MAX_MODES + 1] {
            assert_eq!(
                modal(&nodes, &members, &restrained, &[], &densities, MassModel::Lumped, bad)
                    .unwrap_err()
                    .code,
                "FRAME_INVALID_INPUT"
            );
        }
        for bad_densities in [
            vec![8e-9; 3],
            vec![8e-9, -1., 8e-9, 8e-9],
            vec![8e-9, f64::NAN, 8e-9, 8e-9],
        ] {
            assert_eq!(
                modal(&nodes, &members, &restrained, &[], &bad_densities, MassModel::Lumped, 1)
                    .unwrap_err()
                    .code,
                "FRAME_INVALID_INPUT"
            );
        }
        assert_eq!(
            modal(
                &nodes,
                &members,
                &restrained,
                &[Support::UpperBound { node: 4, dof: 2 }],
                &densities,
                MassModel::Lumped,
                1,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
    }

    /// Two-element beam of total length L with a unit downward force at the
    /// mid node; `fixed_b` selects a fully fixed end B vs a vertical prop.
    /// Out-of-plane and torsion DOFs are restrained at the free nodes, as is
    /// standard when a planar model is run through a 3D solver — a full
    /// bending hinge must not release the out-of-plane stability.
    fn propped_beam(fixed_b: bool) -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, LoadSet) {
        let nodes = vec![[0., 0., 0.], [L / 2., 0., 0.], [L, 0., 0.]];
        let members = vec![member([0, 1]), member([1, 2])];
        let restrained = vec![
            [true; 6],
            [false, false, true, true, true, false],
            if fixed_b {
                [true; 6]
            } else {
                [false, true, true, true, true, false]
            },
        ];
        let (mut forces, moments) = empty_loads(3);
        forces[1] = [0., -1., 0.];
        (
            nodes,
            members,
            restrained,
            LoadSet {
                forces_n: forces,
                moments_nmm: moments,
                loads: vec![],
            },
        )
    }

    #[test]
    fn collapse_cantilever_matches_exact_plastic_load() {
        let model = cantilever();
        let mp = 2.5e6;
        let r = collapse(
            &model.nodes_mm,
            &model.members,
            &model.restrained,
            &[],
            &LoadSet {
                forces_n: model.forces_n.clone(),
                moments_nmm: model.moments_nmm.clone(),
                loads: vec![],
            },
            &[Some(mp)],
            16,
        )
        .unwrap();
        // One base hinge at λ = Mp/(P·L); a hinged cantilever is a mechanism.
        assert_eq!(r.status, CollapseStatus::Mechanism);
        assert_eq!(r.hinges.len(), 1);
        assert_eq!(r.hinges[0].member, 0);
        assert!(r.hinges[0].at_node_a);
        let expected = mp / (1000. * L);
        close(r.hinges[0].load_factor, expected);
        close(r.collapse_load_factor.unwrap(), expected);
    }

    #[test]
    fn collapse_propped_cantilever_matches_classical_solution() {
        let (nodes, members, restrained, loads) = propped_beam(false);
        let mp = 1e6;
        let r = collapse(
            &nodes,
            &members,
            &restrained,
            &[],
            &loads,
            &[Some(mp), Some(mp)],
            16,
        )
        .unwrap();
        // M_A = 3PL/16 yields first at λ₁ = 16Mp/(3L); then both mid-span ends
        // reach Mp together and the beam is a mechanism at λ = 6Mp/L.
        assert_eq!(r.status, CollapseStatus::Mechanism);
        assert_eq!(r.hinges.len(), 3);
        assert_eq!(r.hinges[0].member, 0);
        assert!(r.hinges[0].at_node_a);
        close(r.hinges[0].load_factor, 16. * mp / (3. * L));
        let mut mid: Vec<(usize, bool)> = r.hinges[1..]
            .iter()
            .map(|h| (h.member, h.at_node_a))
            .collect();
        mid.sort();
        assert_eq!(mid, vec![(0, false), (1, true)]);
        for h in &r.hinges[1..] {
            close(h.load_factor, 6. * mp / L);
        }
        close(r.collapse_load_factor.unwrap(), 6. * mp / L);
    }

    #[test]
    fn collapse_fixed_fixed_beam_yields_all_ends_together() {
        let (nodes, members, restrained, loads) = propped_beam(true);
        let mp = 1e6;
        let r = collapse(
            &nodes,
            &members,
            &restrained,
            &[],
            &loads,
            &[Some(mp), Some(mp)],
            16,
        )
        .unwrap();
        // |M| = PL/8 at all four element ends: all yield at λ = 8Mp/L at once.
        assert_eq!(r.status, CollapseStatus::Mechanism);
        assert_eq!(r.hinges.len(), 4);
        for h in &r.hinges {
            close(h.load_factor, 8. * mp / L);
        }
        close(r.collapse_load_factor.unwrap(), 8. * mp / L);
    }

    #[test]
    fn collapse_reports_elastic_unlimited_and_hinge_limit() {
        let (nodes, members, restrained, loads) = propped_beam(true);
        let mp = 1e6;
        // Only member 0 can yield: both its ends hinge at λ = 8Mp/L, then the
        // elastic member 1 carries the mid load as a cantilever from B.
        let r = collapse(
            &nodes,
            &members,
            &restrained,
            &[],
            &loads,
            &[Some(mp), None],
            16,
        )
        .unwrap();
        assert_eq!(r.status, CollapseStatus::ElasticUnlimited);
        assert_eq!(r.collapse_load_factor, None);
        assert_eq!(r.hinges.len(), 2);
        for h in &r.hinges {
            assert_eq!(h.member, 0);
            close(h.load_factor, 8. * mp / L);
        }
        // A budget of one hinge stops the propped cantilever after the first.
        let (nodes, members, restrained, loads) = propped_beam(false);
        let r = collapse(
            &nodes,
            &members,
            &restrained,
            &[],
            &loads,
            &[Some(mp), Some(mp)],
            1,
        )
        .unwrap();
        assert_eq!(r.status, CollapseStatus::HingeLimit);
        assert_eq!(r.collapse_load_factor, None);
        assert_eq!(r.hinges.len(), 1);
        close(r.hinges[0].load_factor, 16. * mp / (3. * L));
    }

    #[test]
    fn collapse_validates_input() {
        let model = cantilever();
        let loads = LoadSet {
            forces_n: model.forces_n.clone(),
            moments_nmm: model.moments_nmm.clone(),
            loads: vec![],
        };
        let good = &[Some(1e6)];
        for bad in [
            &[][..],
            &[None],
            &[Some(0.)],
            &[Some(-1.)],
            &[Some(f64::NAN)],
            &[Some(1e6), None],
        ] {
            assert_eq!(
                collapse(
                    &model.nodes_mm,
                    &model.members,
                    &model.restrained,
                    &[],
                    &loads,
                    bad,
                    16,
                )
                .unwrap_err()
                .code,
                "FRAME_INVALID_INPUT"
            );
        }
        for bad_budget in [0, MAX_PLASTIC_HINGES + 1] {
            assert_eq!(
                collapse(
                    &model.nodes_mm,
                    &model.members,
                    &model.restrained,
                    &[],
                    &loads,
                    good,
                    bad_budget,
                )
                .unwrap_err()
                .code,
                "FRAME_INVALID_INPUT"
            );
        }
        assert_eq!(
            collapse(
                &model.nodes_mm,
                &model.members,
                &model.restrained,
                &[Support::LowerBound { node: 1, dof: 2 }],
                &loads,
                good,
                16,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
        // Axial-only load: no bending at yieldable sections.
        let mut axial = loads.clone();
        axial.forces_n[1] = [1000., 0., 0.];
        assert_eq!(
            collapse(
                &model.nodes_mm,
                &model.members,
                &model.restrained,
                &[],
                &axial,
                good,
                16,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
        // A structure that is a mechanism before any yielding keeps its error.
        assert!(
            collapse(
                &model.nodes_mm,
                &model.members,
                &vec![[false; 6]; 2],
                &[],
                &loads,
                good,
                16,
            )
            .is_err()
        );
    }

    /// Simply supported beam of length L, two elements, planar restraints.
    fn simply_supported() -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>) {
        let nodes = vec![[0., 0., 0.], [L / 2., 0., 0.], [L, 0., 0.]];
        let members = vec![member([0, 1]), member([1, 2])];
        let restrained = vec![
            [true, true, true, true, true, false],
            [false, false, true, true, true, false],
            [false, true, true, true, true, false],
        ];
        (nodes, members, restrained)
    }

    /// Load positions sweeping the beam at quarter points (both elements).
    fn sweep_positions() -> Vec<(usize, f64)> {
        vec![
            (0, 0.),
            (0, L / 4.),
            (0, L / 2.),
            (1, 0.),
            (1, L / 4.),
            (1, L / 2.),
        ]
    }

    #[test]
    fn influence_reaction_and_moment_match_classical_lines() {
        let (nodes, members, restrained) = simply_supported();
        let positions = sweep_positions();
        let down = [0., -1., 0.];
        // Support reaction at A: the line is 1 − x/L.
        let r = influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &positions,
            down,
            &InfluenceTarget::Reaction { node: 0, dof: 1 },
        )
        .unwrap();
        for (v, x) in r.values.iter().zip([0., 250., 500., 500., 750., 1000.]) {
            close(*v, 1. - x / L);
        }
        // Bending moment at mid-span: the triangle peaking at a·b/L = 250.
        let r = influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &positions,
            down,
            &InfluenceTarget::MemberResultant {
                member: 0,
                at_mm: L / 2.,
                resultant: Resultant::MomentZ,
            },
        )
        .unwrap();
        for (v, expected) in r.values.iter().zip([0., 125., 250., 250., 125., 0.]) {
            close(v.abs(), expected);
        }
    }

    #[test]
    fn influence_displacement_matches_reciprocity() {
        let (nodes, members, restrained) = simply_supported();
        let positions = sweep_positions();
        let down = [0., -1., 0.];
        // Mid-span deflection for a load at x equals, by Betti–Maxwell, the
        // deflection at x for a unit load at mid-span: v = −x(3L²−4x²)/(48EI).
        let r = influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &positions,
            down,
            &InfluenceTarget::Displacement { node: 1, dof: 1 },
        )
        .unwrap();
        let exact = |x: f64| {
            let x = x.min(L - x);
            -x * (3. * L * L - 4. * x * x) / (48. * E * I)
        };
        for (v, x) in r.values.iter().zip([0., 250., 500., 500., 750., 1000.]) {
            close(*v, exact(x));
        }
    }

    #[test]
    fn influence_validates_input() {
        let (nodes, members, restrained) = simply_supported();
        let positions = sweep_positions();
        let down = [0., -1., 0.];
        let reaction = InfluenceTarget::Reaction { node: 0, dof: 1 };
        assert!(
            influence(&nodes, &members, &restrained, &[], &[], down, &reaction).is_err()
        );
        assert!(
            influence(
                &nodes,
                &members,
                &restrained,
                &[],
                &[(0, L * 2.)],
                down,
                &reaction,
            )
            .is_err()
        );
        assert!(
            influence(&nodes, &members, &restrained, &[], &[(7, 0.)], down, &reaction)
                .is_err()
        );
        assert!(
            influence(
                &nodes,
                &members,
                &restrained,
                &[],
                &positions,
                [0., 0., 0.],
                &reaction,
            )
            .is_err()
        );
        // Reaction target on a free DOF.
        assert!(
            influence(
                &nodes,
                &members,
                &restrained,
                &[],
                &positions,
                down,
                &InfluenceTarget::Reaction { node: 1, dof: 1 },
            )
            .is_err()
        );
        // Unilateral support refused.
        assert!(
            influence(
                &nodes,
                &members,
                &restrained,
                &[Support::LowerBound { node: 1, dof: 1 }],
                &positions,
                down,
                &reaction,
            )
            .is_err()
        );
    }
}
