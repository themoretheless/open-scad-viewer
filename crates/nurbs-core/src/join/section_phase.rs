//! Closed-section phase alignment to explicit corresponding landmark points.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance::{self, NearestPoint},
    periodic_seam::{self, Verification},
};
pub(crate) fn closed(c: &Curve) -> bool {
    if c.periodic {
        return true;
    }
    let [a, b] = c.domain();
    let p = c.degree;
    c.knots.iter().filter(|&&k| k == a).count() == p + 1
        && c.knots.iter().filter(|&&k| k == b).count() == p + 1
        && c.control_points.first() == c.control_points.last()
}
/// Cut a closed clamped curve at a parameter and join the two rational pieces
/// in reversed order. All arithmetic perturbations must be verified separately.
pub fn curve_candidate(source: &Curve, parameter: f64) -> Result<Curve> {
    source.validate()?;
    check(
        closed(source),
        "Phase alignment needs periodic storage or exactly closed clamped endpoints",
    )?;
    let [a, b] = source.domain();
    check(
        parameter.is_finite() && a <= parameter && parameter < b,
        "Closed seam must lie in the half-open domain",
    )?;
    if source.periodic {
        return periodic_seam::curve_candidate(source, parameter);
    }
    if parameter == a {
        return Ok(source.clone());
    }
    let [left, right] = source.split(parameter)?;
    let p = source.degree;
    let period = b - a;
    let end = parameter + period;
    let scale = right.weights[right.weights.len() - 1] / left.weights[0];
    let mut knots = right.knots[..right.knots.len() - 1].to_vec();
    for &k in &left.knots[p + 1..] {
        knots.push(if k == parameter { end } else { k + period });
    }
    let mut points = right.control_points.clone();
    points.extend(left.control_points[1..].iter().cloned());
    let mut weights = right.weights.clone();
    weights.extend(left.weights[1..].iter().map(|w| w * scale));
    // Two independent clamping operations may perturb the seam endpoints by
    // different amounts. Make stored closure exact, then bound that perturbation.
    let last = points.len() - 1;
    points[last] = points[0].clone();
    let result = Curve {
        degree: p,
        knots,
        control_points: points,
        weights,
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Aligned,
    SearchUnresolved,
    ShapeUnresolved,
}
#[derive(Clone, Debug)]
pub struct Phase {
    pub status: Status,
    pub parameter: f64,
    /// The witness need not be a unique nearest point.
    pub nearest: NearestPoint,
    pub shape: Option<Verification>,
    /// Distance from the landmark to the actual accepted curve's new start.
    pub final_distance_interval: Option<[f64; 2]>,
    /// Upper bound on excess over the accepted curve's globally minimal
    /// landmark distance. Includes shape perturbation, not just search error.
    pub final_minimum_gap_upper: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Atomic all-section result: absent if any search or shape bound is unresolved.
    pub curves: Option<Vec<Curve>>,
    pub phases: Vec<Phase>,
}
/// Select a globally near-minimal point to each section's supplied landmark,
/// then shift its seam with a continuous shape tolerance. This does not infer
/// unique phase, orientation, feature correspondence or globally minimal twist.
/// Search tolerance bounds distance uncertainty, not angular/parameter error.
/// max_cells applies separately to each search and each shape verification.
pub fn align(
    curves: &[Curve],
    anchors: &[Vec<f64>],
    search_tolerance: f64,
    shape_tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    check(
        (2..=32).contains(&curves.len()) && anchors.len() == curves.len(),
        "Phase alignment needs 2..32 matching sections and landmarks",
    )?;
    check(
        search_tolerance.is_finite()
            && search_tolerance > 0.
            && shape_tolerance.is_finite()
            && shape_tolerance > 0.,
        "Phase tolerances must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Phase alignment needs 1..100000 cells per stage",
    )?;
    for (c, anchor) in curves.iter().zip(anchors) {
        c.validate()?;
        check(closed(c), "Phase sections must be closed")?;
        check(
            c.control_points[0].len() == curves[0].control_points[0].len()
                && anchor.len() == c.control_points[0].len()
                && anchor.iter().all(|x| x.is_finite() && x.abs() <= 1e9),
            "Landmarks and sections must have matching bounded finite dimensions",
        )?;
    }
    let mut output = Vec::new();
    let mut phases = Vec::new();
    let mut accepted = true;
    for (c, anchor) in curves.iter().zip(anchors) {
        let mut nearest = curve_distance::nearest_point(c, anchor, search_tolerance, max_cells)?;
        let [a, b] = c.domain();
        if nearest.parameter == b {
            // Periodic validation allows a small storage-seam residual. Do not
            // transfer the end witness's distance proof to the start unchecked.
            use crate::distance_bounds::{Interval as I, box_distance};
            let span = (c.degree..c.control_points.len())
                .find(|&i| c.knots[i] <= a && a < c.knots[i + 1])
                .ok_or_else(|| crate::input("No span owns the canonical seam"))?;
            let image = curve_distance::enclosure(c, span, I::point(a))?;
            let point: Vec<_> = anchor.iter().map(|&x| I::point(x)).collect();
            let upper = box_distance(&image, &point)?.1;
            nearest.parameter = a;
            nearest.point = c.evaluate(a)?.point;
            nearest.point_enclosure = image.iter().map(|x| [x.lo, x.hi]).collect();
            nearest.distance_interval[1] = upper;
            nearest.converged = upper - nearest.distance_interval[0] <= search_tolerance;
            nearest.reason = if nearest.converged {
                crate::DistanceStopReason::Tolerance
            } else {
                crate::DistanceStopReason::PrecisionLimit
            };
        }
        let parameter = if nearest.parameter == b {
            a
        } else {
            nearest.parameter
        };
        if !nearest.converged {
            accepted = false;
            phases.push(Phase {
                status: Status::SearchUnresolved,
                parameter,
                nearest,
                shape: None,
                final_distance_interval: None,
                final_minimum_gap_upper: None,
            });
            continue;
        }
        let candidate = curve_candidate(c, parameter)?;
        let proof = periodic_seam::verify_curve(c, &candidate, shape_tolerance, max_cells)?;
        let (final_distance_interval, final_minimum_gap_upper) = if proof.accepted {
            use crate::distance_bounds::{Interval as I, box_distance};
            let start = candidate.domain()[0];
            let span = (candidate.degree..candidate.control_points.len())
                .find(|&i| candidate.knots[i] <= start && start < candidate.knots[i + 1])
                .ok_or_else(|| crate::input("No span owns the accepted seam"))?;
            let image = curve_distance::enclosure(&candidate, span, I::point(start))?;
            let landmark: Vec<_> = anchor.iter().map(|&x| I::point(x)).collect();
            let (lower, upper) = box_distance(&image, &landmark)?;
            // Hausdorff <= shape.error_upper implies the accepted curve's
            // global minimum is at least original_min_lower - shape_error.
            let gap = I::point(upper)
                .sub(I::point(nearest.distance_interval[0]))?
                .add(I::point(proof.error_upper))?
                .hi
                .max(0.);
            (Some([lower, upper]), Some(gap))
        } else {
            (None, None)
        };
        let status = if proof.accepted {
            output.push(candidate);
            Status::Aligned
        } else {
            accepted = false;
            Status::ShapeUnresolved
        };
        phases.push(Phase {
            status,
            parameter,
            nearest,
            shape: Some(proof),
            final_distance_interval,
            final_minimum_gap_upper,
        });
    }
    Ok(Report {
        curves: accepted.then_some(output),
        phases,
    })
}
