//! Whole original-root world error using correlated composition and the actual
//! verified contraction operator. This is not physical branch/trim topology.
use super::*;
use crate::{curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Copy, Debug)]
pub struct FiberWorldTraceLimits {
    pub tolerance_world: f64,
    pub max_rectangles: usize,
    pub max_agreement_cells: usize,
}
impl Default for FiberWorldTraceLimits {
    fn default() -> Self {
        Self {
            tolerance_world: 1e-7,
            max_rectangles: 100000,
            max_agreement_cells: 4096,
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedFiberWorldTrace {
    source: QualifiedFiber,
    pcurves: [Curve; 2],
    curve: Curve,
    limits: FiberWorldTraceLimits,
    errors: [f64; 2],
    scaled_parameter_error: f64,
    world_error: f64,
    orientation: TransitionOrientation,
    rectangles: usize,
    agreement_cells: usize,
}
impl QualifiedFiberWorldTrace {
    pub fn source(&self) -> &QualifiedFiber {
        &self.source
    }
    pub fn pcurves(&self) -> &[Curve; 2] {
        &self.pcurves
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn composition_errors(&self) -> [f64; 2] {
        self.errors
    }
    pub fn scaled_parameter_error(&self) -> f64 {
        self.scaled_parameter_error
    }
    pub fn world_error_upper(&self) -> f64 {
        self.world_error
    }
    pub fn orientation(&self) -> TransitionOrientation {
        self.orientation
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn agreement_cells(&self) -> usize {
        self.agreement_cells
    }
    pub fn reverify(&self) -> Result<()> {
        let r = qualify_fiber_world_trace(&self.source, &self.pcurves, &self.curve, self.limits)?;
        let fresh = r
            .trace
            .ok_or_else(|| crate::input("Original root world trace remains unproved"))?;
        check(
            fresh.errors == self.errors
                && fresh.scaled_parameter_error == self.scaled_parameter_error
                && fresh.world_error == self.world_error
                && fresh.orientation == self.orientation
                && fresh.rectangles == self.rectangles
                && fresh.agreement_cells == self.agreement_cells,
            "Original world trace source error, clock or proof work changed",
        )
    }
}
pub struct FiberWorldTraceResult {
    trace: Option<QualifiedFiberWorldTrace>,
    reason: &'static str,
    rectangles: usize,
    agreement_cells: usize,
}
impl FiberWorldTraceResult {
    pub fn trace(&self) -> Option<&QualifiedFiberWorldTrace> {
        self.trace.as_ref()
    }
    pub fn into_trace(self) -> Option<QualifiedFiberWorldTrace> {
        self.trace
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn agreement_cells(&self) -> usize {
        self.agreement_cells
    }
}
fn clamped(c: &Curve) -> bool {
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&t| t == c.domain()[0])
        && c.knots[c.control_points.len()..]
            .iter()
            .all(|&t| t == c.domain()[1])
}
// Positive rational B-spline scalar poles preserve their order. Strict
// progress on every active knot block excludes flat spans; interior
// multiplicity <= degree preserves continuity and full endpoint coverage.
fn monotone_clock(c: &Curve, axis: usize, increasing: bool) -> bool {
    let n = c.control_points.len();
    let d = c.domain();
    if c.control_points.windows(2).any(|p| {
        if increasing {
            p[0][axis] > p[1][axis]
        } else {
            p[0][axis] < p[1][axis]
        }
    }) {
        return false;
    }
    for &k in &c.knots {
        if d[0] < k && k < d[1] && c.knots.iter().filter(|&&v| v == k).count() > c.degree {
            return false;
        }
    }
    (c.degree..n)
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .all(|i| {
            if increasing {
                c.control_points[i - c.degree][axis] < c.control_points[i][axis]
            } else {
                c.control_points[i - c.degree][axis] > c.control_points[i][axis]
            }
        })
}
pub(super) fn root_error(source: &QualifiedFiber, errors: [f64; 2]) -> Result<(f64, f64)> {
    let free = source.free_axis().index();
    let (Some(op), Some(q)) = (
        source.report().operator.as_ref(),
        source.report().contraction_upper,
    ) else {
        return Err(crate::input("Missing original root operator"));
    };
    let residual = I::point(errors[0]).add(I::point(errors[1]))?;
    let mut gain = 0_f64;
    for i in 0..3 {
        let mut row = I::point(0.);
        for r in op.preconditioner[i] {
            row = row.add(I::point(r.abs()))?;
        }
        gain = gain.max(row.div(I::point(op.scales[i]))?.hi);
    }
    let delta = I::point(gain)
        .mul(residual)?
        .div(I::point(1.).sub(I::point(q))?)?;
    let unknown = (0..4).filter(|&a| a != free).collect::<Vec<_>>();
    let mut world = I::point(errors[0]);
    // L1 bounds the Euclidean world norm; every derivative is from the
    // same complete original first-surface box used by the actual operator.
    for k in 0..3 {
        for a in 0..2 {
            if a != free {
                let index = unknown.iter().position(|&n| n == a).unwrap();
                let j = op.first_jacobian[k][a];
                let magnitude = j[0].abs().max(j[1].abs());
                world = world.add(
                    I::point(magnitude)
                        .mul(I::point(op.scales[index]))?
                        .mul(delta)?,
                )?;
            }
        }
    }
    Ok((delta.hi, world.hi))
}
/// Qualify the ACTUAL candidate coefficients against one specific original
/// root fiber. All curves share normalized forward traversal, with an explicit
/// monotone source-free-coordinate clock covering the whole source interval.
/// Positive rational hulls bind both pcurves inside the original box; the free
/// coordinate clock is a continuously monotone positive B-spline.
/// Whole original C-S1(P1) and C-S2(P2) composition bounds give F(candidate).
/// The actual verified R/scales/q give distance to the unique original root:
/// ||root-candidate||scaled <= ||D^-1 R F(candidate)||inf/(1-q).
/// Original whole S1 Jacobian bounds then give continuous world error + lift
/// error. Candidate positions, fit samples and proximity to both supports are
/// never by themselves source-root authority. Whole original agreement cells
/// share one allowance; unresolved source/knot cells never authorize a trace.
/// Neither nonzero world speed, physical embedding nor trim topology is claimed.
pub fn qualify_fiber_world_trace(
    source: &QualifiedFiber,
    pcurves: &[Curve; 2],
    curve: &Curve,
    limits: FiberWorldTraceLimits,
) -> Result<FiberWorldTraceResult> {
    check(
        limits.tolerance_world.is_finite()
            && (1e-12..=1e-2).contains(&limits.tolerance_world)
            && (1..=100000).contains(&limits.max_rectangles)
            && limits.max_agreement_cells <= 100000,
        "Invalid original world trace tolerance or work allowances",
    )?;
    for p in pcurves {
        p.validate()?;
        check(
            p.control_points[0].len() == 2,
            "Root trace requires actual two-dimensional pcurves",
        )?;
    }
    curve.validate()?;
    check(
        curve.control_points[0].len() == 3,
        "Root world trace requires actual three-dimensional coefficients",
    )?;
    let mut out = FiberWorldTraceResult {
        trace: None,
        reason: "original-fiber-replay-unproven",
        rectangles: 0,
        agreement_cells: 0,
    };
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    if replay_fiber(source, &mut work).is_err() {
        out.rectangles = work.used;
        return Ok(out);
    }
    out.rectangles = work.used;
    out.reason = "candidate-clock-or-box-unproven";
    if !clamped(curve) || pcurves.iter().any(|p| !clamped(p)) {
        return Ok(out);
    }
    let box4 = source.domain();
    let free = source.free_axis().index();
    for (i, p) in pcurves.iter().enumerate() {
        if p.control_points.iter().any(|point| {
            (0..2).any(|a| point[a] < box4[2 * i + a][0] || point[a] > box4[2 * i + a][1])
        }) {
            return Ok(out);
        }
    }
    let clock = &pcurves[free / 2];
    if clock.control_points.len() > 4096 {
        return Ok(out);
    }
    let axis = free % 2;
    let first = clock.control_points[0][axis];
    let last = clock.control_points.last().unwrap()[axis];
    let orientation = if first == box4[free][0]
        && last == box4[free][1]
        && monotone_clock(clock, axis, true)
    {
        TransitionOrientation::Forward
    } else if first == box4[free][1] && last == box4[free][0] && monotone_clock(clock, axis, false)
    {
        TransitionOrientation::Reverse
    } else {
        return Ok(out);
    };
    if first == last {
        return Ok(out);
    }
    // Ordered positive rational B-spline scalar poles with strict progress
    // on every active local block make this continuous clock strictly
    // monotone and onto, including both literal original interval endpoints.
    let epsilon = (|| -> Result<f64> {
        let (_, factor) = root_error(source, [1., 1.])?;
        Ok(I::point(limits.tolerance_world)
            .div(I::point(2.).mul(I::point(factor))?)?
            .lo)
    })();
    let Ok(epsilon) = epsilon else {
        out.reason = "original-root-error-unproven";
        return Ok(out);
    };
    if epsilon <= 0. {
        out.reason = "original-root-error-unproven";
        return Ok(out);
    }
    let mut errors = [0.; 2];
    out.reason = "original-agreement-unproven";
    for i in 0..2 {
        if out.agreement_cells == limits.max_agreement_cells {
            out.reason = "agreement-work-unproven";
            return Ok(out);
        }
        let s = if i == 0 {
            source.source().first()
        } else {
            source.source().second()
        };
        let r = crate::curve_surface_agreement::verify(
            curve,
            &pcurves[i],
            s,
            false,
            epsilon,
            limits.max_agreement_cells - out.agreement_cells,
        );
        let Ok(report) = r else {
            // A failed native verifier cannot expose its partial cell count;
            // reserve the remaining allowance instead of granting it again.
            out.agreement_cells = limits.max_agreement_cells;
            return Ok(out);
        };
        out.agreement_cells += report.cells;
        if report.status != crate::curve_surface_agreement::Status::WithinTolerance {
            return Ok(out);
        }
        errors[i] = epsilon;
    }
    let bound = root_error(source, errors);
    let Ok((scaled_parameter_error, world_error)) = bound else {
        out.reason = "original-root-error-unproven";
        return Ok(out);
    };
    out.reason = "world-tolerance-unproven";
    if world_error > limits.tolerance_world {
        return Ok(out);
    }
    out.reason = "qualified-original-root-world-trace";
    out.trace = Some(QualifiedFiberWorldTrace {
        source: source.clone(),
        pcurves: pcurves.clone(),
        curve: curve.clone(),
        limits,
        errors,
        scaled_parameter_error,
        world_error,
        orientation,
        rectangles: out.rectangles,
        agreement_cells: out.agreement_cells,
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_world_root_error_and_actual_coefficients_require_source_replay() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            control_points: vec![
                vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
                vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
            ],
            ..a.clone()
        };
        let sources = FiberSources::new(&a, &b).unwrap();
        let f = sources
            .certify(
                [[0., 1.], [0.25, 0.75], [0.125, 0.625], [0.25, 0.75]],
                FiberAxis::FirstU,
                FiberLimits::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let p = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0.5], vec![1., 0.5]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let q = Curve {
            control_points: vec![vec![0.25, 0.5], vec![0.5, 0.5]],
            ..p.clone()
        };
        let c = Curve {
            control_points: vec![vec![0.25, 0.5, 0.], vec![0.5, 0.5, 0.]],
            ..p.clone()
        };
        let owner = qualify_fiber_world_trace(&f, &[p, q], &c, FiberWorldTraceLimits::default())
            .unwrap()
            .into_trace()
            .unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.world_error = 0.;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.scaled_parameter_error = 0.;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.errors = [0.; 2];
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.rectangles -= 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.agreement_cells = 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.curve.control_points[1][2] = 0.01;
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        bad.source = f
            .at(0.5, FiberLimits::default())
            .unwrap()
            .into_fiber()
            .unwrap();
        assert!(bad.reverify().is_err());
    }
}
