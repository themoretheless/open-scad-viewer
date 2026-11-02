//! Rounded polynomial lift proposals, admitted only by complete actual lift proof.
use crate::{
    Result, bernstein_interpolation as interpolation, check, curve::Curve,
    distance_bounds::Interval as I, surface::Surface, surface_offset,
};
#[derive(Clone, Copy)]
pub struct FitLimits {
    pub max_degree: usize,
    pub evaluations: usize,
    pub coefficient_work: usize,
    /// Shared across all attempted degrees, not reset after a failed proposal.
    pub proof_cells: usize,
    pub spans: usize,
}
impl Default for FitLimits {
    fn default() -> Self {
        Self {
            max_degree: 16,
            evaluations: 4096,
            coefficient_work: 100000,
            proof_cells: 100000,
            spans: 128,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitStop {
    Certified,
    DegreeLimit,
    EvaluationLimit,
    CoefficientLimit,
    ProofLimit,
    SourceUnresolved,
}
pub struct FitAttempt {
    pub degree: usize,
    pub proof: super::Report,
}
pub struct FitReport {
    pub curve: Option<Curve>,
    pub attempts: Vec<FitAttempt>,
    pub evaluations: usize,
    pub coefficient_work: usize,
    pub proof_cells: usize,
    pub stop: FitStop,
}
pub(crate) fn validate(l: FitLimits) -> Result<()> {
    check(
        (1..=16).contains(&l.max_degree)
            && (1..=100000).contains(&l.evaluations)
            && (1..=10000000).contains(&l.coefficient_work)
            && (1..=100000).contains(&l.proof_cells)
            && (1..=100000).contains(&l.spans),
        "Choose bounded actual lift fitting work",
    )
}
/// Fit a 3D lift of the ACTUAL supplied UV curve on an original normal offset.
/// Optional literal world endpoint poles are proposals only, used for exact
/// closure with existing qualified edges. Samples and interpolation never issue
/// lift authority; every full rounded candidate must pass direct interval proof.
pub fn fit(
    uv: &Curve,
    source: &Surface,
    distance: f64,
    tolerance_mm: f64,
    ends: Option<[[f64; 3]; 2]>,
    limits: FitLimits,
) -> Result<FitReport> {
    validate(limits)?;
    uv.validate()?;
    source.validate()?;
    check(
        uv.control_points[0].len() == 2
            && !uv.periodic
            && distance.is_finite()
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && ends.is_none_or(|e| e.iter().flatten().all(|x| x.is_finite())),
        "Choose a finite original UV lift and endpoint proposals",
    )?;
    let natural = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    check(
        uv.weights.iter().all(|w| *w > 0.)
            && uv
                .control_points
                .iter()
                .all(|p| (0..2).all(|k| p[k] >= natural[k][0] && p[k] <= natural[k][1])),
        "Whole lift UV control hull must stay in the original chart",
    )?;
    let mut out = FitReport {
        curve: None,
        attempts: vec![],
        evaluations: 0,
        coefficient_work: 0,
        proof_cells: 0,
        stop: FitStop::DegreeLimit,
    };
    let mut degrees = vec![1usize];
    while *degrees.last().unwrap() < limits.max_degree {
        degrees.push((2 * degrees.last().unwrap()).min(limits.max_degree));
    }
    let clock = uv.domain();
    for (attempt, &degree) in degrees.iter().enumerate() {
        let n = degree + 1;
        if out.proof_cells == limits.proof_cells {
            out.stop = FitStop::ProofLimit;
            break;
        }
        if n > limits.evaluations - out.evaluations {
            out.stop = FitStop::EvaluationLimit;
            break;
        }
        let required = n * degree * (degree + 3) / 2 + 3 * n * n;
        if required > limits.coefficient_work - out.coefficient_work {
            out.stop = FitStop::CoefficientLimit;
            break;
        }
        let sites: Vec<_> = (0..n)
            .map(|i| {
                if i == 0 {
                    clock[0]
                } else if i == degree {
                    clock[1]
                } else {
                    let f = (1. - (std::f64::consts::PI * i as f64 / degree as f64).cos()) * 0.5;
                    clock[0] * (1. - f) + clock[1] * f
                }
            })
            .collect();
        check(
            sites.windows(2).all(|r| r[0] < r[1]),
            "Lift proposal clock resolution exhausted",
        )?;
        let (Some(basis), work) = interpolation::basis_on(
            &sites,
            clock,
            limits.coefficient_work - out.coefficient_work,
        )?
        else {
            unreachable!("prechecked coefficient work");
        };
        out.coefficient_work += work;
        let mut samples = vec![];
        for &t in &sites {
            out.evaluations += 1;
            let p = uv.evaluate(t)?.point;
            // Sampling is a proposal. The positive control hull above admits
            // clamping evaluator roundoff to the ORIGINAL chart, never extension.
            let domain = std::array::from_fn(|k| [p[k].max(natural[k][0]).min(natural[k][1]); 2]);
            let Some(point) = surface_offset::bounds(source, domain, distance, limits.spans)?.image
            else {
                out.stop = FitStop::SourceUnresolved;
                return Ok(out);
            };
            samples.push(point.map(|r| r[0] * 0.5 + r[1] * 0.5));
        }
        let mut points = vec![vec![0.; 3]; n];
        for k in 0..3 {
            let data: Vec<_> = samples.iter().map(|p| I::point(p[k])).collect();
            let c = interpolation::reconstruct(&data, &basis)?;
            out.coefficient_work += n * n;
            for (i, c) in c.into_iter().enumerate() {
                points[i][k] = c.lo * 0.5 + c.hi * 0.5;
            }
        }
        if let Some(ends) = ends {
            points[0] = ends[0].to_vec();
            points[n - 1] = ends[1].to_vec();
        }
        let candidate = Curve {
            degree,
            knots: std::iter::repeat_n(clock[0], n)
                .chain(std::iter::repeat_n(clock[1], n))
                .collect(),
            control_points: points,
            weights: vec![1.; n],
            periodic: false,
        };
        let cells = ((limits.proof_cells - out.proof_cells) / (degrees.len() - attempt)).max(1);
        let proof = super::certify(
            &candidate,
            uv,
            source,
            distance,
            tolerance_mm,
            super::Limits {
                cells,
                spans: limits.spans,
            },
        )?;
        out.proof_cells += proof.visited;
        let valid = proof.lift_correspondence_proven;
        out.attempts.push(FitAttempt { degree, proof });
        if valid {
            out.curve = Some(candidate);
            out.stop = FitStop::Certified;
            break;
        }
    }
    Ok(out)
}
