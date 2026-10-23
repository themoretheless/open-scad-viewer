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

mod beam_kernel;
use beam_kernel::*;


mod assembly;
use assembly::*;
pub use assembly::{diagnose};


mod spectral;
use spectral::*;
pub use spectral::{FrameBucklingMode, FrameBucklingResponse, buckling, FrameModalMode, FrameModalResponse, modal};


/// Budget of hinge insertions in one collapse analysis.
pub const MAX_PLASTIC_HINGES: usize = 256;

mod plasticity;
use plasticity::*;
pub use plasticity::{PlasticHinge, CollapseStatus, FrameCollapseResponse, collapse};


/// Maximum load positions in one influence line request.
pub const MAX_INFLUENCE_POSITIONS: usize = 1024;

mod influence;
use influence::*;
pub use influence::{Resultant, InfluenceTarget, InfluenceResponse, influence};


mod solution;
use solution::*;
pub use solution::{solve, solve_envelopes};


#[cfg(test)]
#[path = "tests/frame.rs"]
mod tests;
