//! Global nonintersection proof or original-source interior contact witness.
use crate::{Result, surface::Surface, surface_injectivity};
use num_rational::BigRational;
use num_traits::{FromPrimitive, ToPrimitive};
pub(super) mod linear_crossing;
mod nonlinear_crossing;
mod spatial_crossing;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingProof {
    PlanarCurve,
    CurveSurface,
}
#[derive(Clone, Debug)]
pub struct CertifiedCrossing {
    pub proof: CrossingProof,
    /// Two original normalized UV boxes, each enclosing a distinct preimage.
    pub parameter_boxes: [[[f64; 2]; 2]; 2],
    pub position_bounds: [[f64; 2]; 3],
    pub cells: usize,
}
#[derive(Clone, Debug)]
pub struct ProbeAttempt {
    pub kind: &'static str,
    pub axis: usize,
    pub parameter: f64,
    pub cells: usize,
    pub iso_coverage_complete: bool,
    pub refusal: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct ProbeSearch {
    pub attempts: Vec<ProbeAttempt>,
    /// Charged search units, including admissions and interval solver cells.
    pub cells: usize,
    pub exhausted: bool,
}
mod local_fold;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Absent,
    Present,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub injectivity: surface_injectivity::Report,
    /// Distinct interior normalized parameters, interpreted by the exact affine
    /// active-domain map. Populated only from an exact original-source proof.
    pub witness: Option<[[f64; 2]; 2]>,
    /// Authoritative exact normalized UV pair, including non-dyadic values.
    /// `witness` is populated only when every scalar is exactly binary64.
    pub exact_witness: Option<[[BigRational; 2]; 2]>,
    /// Existence proof with interval UV boxes; no rounded point is authoritative.
    pub certified_crossing: Option<CertifiedCrossing>,
    /// Finite search proposals, never evidence of whole-surface coverage.
    pub probe_search: ProbeSearch,
    /// Reflection axis for fold proofs; None for proper span crossings.
    pub reflected_axis: Option<usize>,
}
fn reflected_basis(k: &[f64], p: usize, n: usize) -> bool {
    let (a, b) = (k[p], k[n]);
    if !k[..=p].iter().all(|x| *x == a) || !k[n..].iter().all(|x| *x == b) {
        return false;
    }
    // C0 continuity makes the identity valid even when the witness lands on
    // an internal knot: opposite one-sided endpoint conventions then agree.
    let mut multiplicity = 0;
    let mut previous = a;
    for &t in &k[p + 1..n] {
        multiplicity = if t == previous { multiplicity + 1 } else { 1 };
        previous = t;
        if t <= a || t >= b || multiplicity > p {
            return false;
        }
    }
    let q = |x| BigRational::from_f64(x).unwrap();
    let sum = q(a) + q(b);
    // Rounded binary64 sums can hide asymmetric knots by one ULP. Compare
    // the exact dyadic values of the original input instead.
    k.iter()
        .zip(k.iter().rev())
        .all(|(&x, &y)| q(x) + q(y) == sum)
}
/// Absence follows a single global contraction bound across every source span.
/// Presence uses exact rational reflection or proper crossings of original
/// degree-one iso-curve spans. The latter checks all three world coordinates
/// and original rational parameter clocks. Up to 15 interior dyadic proposals
/// per axis steer planar iso-curve Krawczyk proofs on original curve/surface
/// bounds. Four further central iso/surface windows can prove nonplanar
/// contacts with different transverse parameters. That route requires an
/// exactly binary64-representable source iso and uses original 3D bounds.
/// Probe admissions and solver cells share max_spans search units.
/// Probes select candidates only; finite exhaustion never proves absence.
/// Local reflection spans and linear span pairs each have max_spans work caps.
/// All other cases (including unsupported periodic charts and insufficient
/// budgets) stay Unresolved. Isolated local regularity never proves absence.
pub fn inspect(s: &Surface, max_spans: usize) -> Result<Report> {
    let injectivity = surface_injectivity::certify(s, max_spans)?;
    let mut out = Report {
        status: if injectivity.proven {
            Status::Absent
        } else {
            Status::Unresolved
        },
        injectivity,
        witness: None,
        exact_witness: None,
        certified_crossing: None,
        probe_search: ProbeSearch::default(),
        reflected_axis: None,
    };
    if out.status == Status::Absent {
        return Ok(out);
    }
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if s.periodic_u || s.periodic_v {
        return Ok(out);
    }
    for axis in 0..2 {
        let eligible = if axis == 0 {
            reflected_basis(&s.knots_u, s.degree_u, nu)
        } else {
            reflected_basis(&s.knots_v, s.degree_v, nv)
        };
        if !eligible {
            continue;
        }
        let reflected = (0..nu).all(|i| {
            (0..nv).all(|j| {
                let (a, b) = if axis == 0 {
                    (nu - 1 - i, j)
                } else {
                    (i, nv - 1 - j)
                };
                s.control_points[i][j] == s.control_points[a][b]
                    && s.weights[i][j] == s.weights[a][b]
            })
        });
        if reflected {
            let witness = if axis == 0 {
                [[0.25, 0.5], [0.75, 0.5]]
            } else {
                [[0.5, 0.25], [0.5, 0.75]]
            };
            record(
                &mut out,
                Some(axis),
                witness.map(|uv| uv.map(|x| BigRational::from_f64(x).unwrap())),
            );
            return Ok(out);
        }
    }
    if let Some((axis, witness)) = local_fold::find(s, max_spans) {
        record(&mut out, Some(axis), witness);
    } else if let Some(witness) = linear_crossing::find(s, max_spans) {
        record(&mut out, None, witness);
    } else {
        let (mut crossing, mut search) = nonlinear_crossing::find(s, max_spans)?;
        if crossing.is_none() && search.cells < max_spans {
            crossing = spatial_crossing::find(s, max_spans, &mut search)?;
        }
        out.probe_search = search;
        if let Some(crossing) = crossing {
            out.status = Status::Present;
            out.certified_crossing = Some(crossing);
        }
    }
    Ok(out)
}

fn record(out: &mut Report, axis: Option<usize>, exact: [[BigRational; 2]; 2]) {
    let mut floating = [[0.; 2]; 2];
    let mut representable = true;
    for i in 0..2 {
        for j in 0..2 {
            match exact[i][j].to_f64() {
                Some(x) if BigRational::from_f64(x).as_ref() == Some(&exact[i][j]) => {
                    floating[i][j] = x
                }
                _ => representable = false,
            }
        }
    }
    out.status = Status::Present;
    out.reflected_axis = axis;
    out.witness = representable.then_some(floating);
    out.exact_witness = Some(exact);
}
