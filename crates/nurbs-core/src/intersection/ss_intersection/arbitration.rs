//! Interval arbitration for two nearly degenerate continuation branches.
//!
//! `CommonBranch` is a *local common-coverage* certificate: every parameter
//! interval is covered, both branches are independently re-traced on the
//! original surfaces, and outward-rounded original-surface enclosures agree in
//! the explicit geometric band. It is not a global uniqueness or branch-count
//! proof. Samples alone never authorize a merge.
use super::*;
use crate::Error;
use crate::foundation::guards::Budget;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NearDegenerateSsiLimits {
    /// Maximum number of original samples on either branch.
    pub max_samples: usize,
    /// Maximum number of interval nodes visited, including subdivisions.
    pub max_interval_work: usize,
    /// Maximum binary subdivision depth of one original sample interval.
    pub max_depth: usize,
}

impl Default for NearDegenerateSsiLimits {
    fn default() -> Self {
        Self {
            max_samples: MAX_CONTINUATION * 2 + 1,
            max_interval_work: 8192,
            max_depth: 16,
        }
    }
}

/// One half-open parameter coverage cell. The active domain end owns the final
/// endpoint; adjacent cells therefore have neither gaps nor duplicate interior.
#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedSsiInterval {
    pub parameter: [f64; 2],
    /// UV box enclosing the first branch on this interval.
    pub first_uv: [f64; 4],
    /// UV box enclosing the second branch on this interval.
    pub second_uv: [f64; 4],
    /// ST box enclosing the first branch on this interval.
    pub first_st: [f64; 4],
    /// ST box enclosing the second branch on this interval.
    pub second_st: [f64; 4],
    /// Union of outward-rounded original-surface and re-traced point enclosures.
    pub geometry_enclosure: [[f64; 2]; 3],
    pub max_residual_mm: f64,
}

pub enum NearDegenerateSsiArbitration {
    /// Whole-branch spatial enclosures are disjoint beyond the geometric band.
    DistinctByEnclosures,
    /// Certified local common coverage, not global branch uniqueness.
    CommonBranch {
        samples: Vec<ContinuationSample>,
        intervals: Vec<CertifiedSsiInterval>,
        max_residual_mm: f64,
        geometric_tolerance_mm: f64,
        interval_work: usize,
    },
}

#[derive(Clone, Copy)]
struct RetracedPoint {
    uv: [f64; 2],
    st: [f64; 2],
    point: [f64; 3],
    residual_mm: f64,
}

#[derive(Clone, Copy)]
struct RetracedPair {
    parameter: f64,
    seam_wrap: Option<[[i32; 2]; 2]>,
    a: RetracedPoint,
    b: RetracedPoint,
}

impl RetracedPair {
    fn residual_mm(self) -> f64 {
        self.a
            .residual_mm
            .max(self.b.residual_mm)
            .max(distance(&self.a.point, &self.b.point))
    }

    fn sample(self) -> ContinuationSample {
        ContinuationSample {
            point: std::array::from_fn(|i| (self.a.point[i] + self.b.point[i]) * 0.5),
            uv_first: std::array::from_fn(|i| (self.a.uv[i] + self.b.uv[i]) * 0.5),
            uv_second: std::array::from_fn(|i| (self.a.st[i] + self.b.st[i]) * 0.5),
            parameter: self.parameter,
            seam_wrap: self.seam_wrap,
        }
    }
}

fn enclosure_gap(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> f64 {
    (0..3)
        .map(|axis| {
            (a[axis][0] - b[axis][1])
                .max(b[axis][0] - a[axis][1])
                .max(0.)
                .powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

fn valid_enclosure(enclosure: [[f64; 2]; 3]) -> bool {
    enclosure
        .iter()
        .all(|axis| axis[0].is_finite() && axis[1].is_finite() && axis[0] <= axis[1])
}

fn union_enclosure(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        [
            next_down(a[axis][0].min(b[axis][0])),
            next_up(a[axis][1].max(b[axis][1])),
        ]
    })
}

fn unresolved(line: u32) -> Error {
    Error::new(
        "NURBS_SSI_NEAR_DEGENERATE_UNRESOLVED",
        format!(
            "Nearly degenerate SSI branches have neither separated enclosures nor complete interval common-branch coverage (gate {line})"
        ),
    )
}

fn budget(message: impl Into<String>) -> Error {
    Error::new("NURBS_SSI_ARBITRATION_BUDGET", message)
}

fn midpoint(a: f64, b: f64) -> f64 {
    a + (b - a) * 0.5
}

fn interpolate(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn padded_box(a: [f64; 2], b: [f64; 2], domain: [[f64; 2]; 2], padding: f64) -> [f64; 4] {
    [
        next_down(a[0].min(b[0]) - padding).max(domain[0][0]),
        next_up(a[0].max(b[0]) + padding).min(domain[0][1]),
        next_down(a[1].min(b[1]) - padding).max(domain[1][0]),
        next_up(a[1].max(b[1]) + padding).min(domain[1][1]),
    ]
}

fn box_center(b: [f64; 4]) -> [f64; 2] {
    [midpoint(b[0], b[1]), midpoint(b[2], b[3])]
}

fn surface_enclosure(surface: &Surface, b: [f64; 4]) -> Result<[[f64; 2]; 3]> {
    if !b.iter().all(|v| v.is_finite()) || b[0] >= b[1] || b[2] >= b[3] {
        return Err(unresolved(line!()));
    }
    let trimmed = surface.trim(b)?;
    let grid = homogeneous_grid(&trimmed);
    let enclosure = super::super::hull_ranges(&grid.iter().flatten().copied().collect::<Vec<_>>());
    if valid_enclosure(enclosure) {
        Ok(enclosure)
    } else {
        Err(unresolved(line!()))
    }
}

fn retrace_point(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    uv_box: [f64; 4],
    st_box: [f64; 4],
    geometric_tolerance_mm: f64,
) -> Result<RetracedPoint> {
    if box_center(uv_box)
        .iter()
        .chain(&box_center(st_box))
        .any(|v| !v.is_finite())
    {
        return Err(unresolved(line!()));
    }
    let Some((ruv, rst, point, contact)) = refine_seed(
        first,
        second,
        uv,
        st,
        uv_box,
        st_box,
        geometric_tolerance_mm,
    )?
    else {
        return Err(unresolved(line!()));
    };
    if contact != ContactClass::Transverse {
        return Err(Error::new(
            "NURBS_SSI_NEAR_DEGENERATE_UNRESOLVED",
            format!("Interval re-trace is not transverse: {contact:?}"),
        ));
    }
    let pa = point3(&first.evaluate(ruv[0], ruv[1])?.point)?;
    let pb = point3(&second.evaluate(rst[0], rst[1])?.point)?;
    let residual_mm = distance(&pa, &pb);
    if !residual_mm.is_finite() || residual_mm > geometric_tolerance_mm {
        return Err(unresolved(line!()));
    }
    Ok(RetracedPoint {
        uv: ruv,
        st: rst,
        point,
        residual_mm,
    })
}

fn paired_boxes(
    a: RetracedPoint,
    b: RetracedPoint,
    first_domain: [[f64; 2]; 2],
    second_domain: [[f64; 2]; 2],
    parameter_tolerance: f64,
) -> ([f64; 4], [f64; 4]) {
    (
        padded_box(a.uv, b.uv, first_domain, parameter_tolerance),
        padded_box(a.st, b.st, second_domain, parameter_tolerance),
    )
}

#[allow(clippy::too_many_arguments)]
fn retrace_branch_pair(
    first: &Surface,
    second: &Surface,
    a_left: &ContinuationSample,
    a_right: &ContinuationSample,
    b_left: &ContinuationSample,
    b_right: &ContinuationSample,
    parameter: f64,
    t: f64,
    first_domain: [[f64; 2]; 2],
    second_domain: [[f64; 2]; 2],
    geometric_tolerance_mm: f64,
    parameter_tolerance: f64,
) -> Result<RetracedPair> {
    if a_left.seam_wrap != a_right.seam_wrap
        || b_left.seam_wrap != b_right.seam_wrap
        || a_left.seam_wrap != b_left.seam_wrap
    {
        return Err(unresolved(line!()));
    }
    let a_uv = interpolate(a_left.uv_first, a_right.uv_first, t);
    let a_st = interpolate(a_left.uv_second, a_right.uv_second, t);
    let b_uv = interpolate(b_left.uv_first, b_right.uv_first, t);
    let b_st = interpolate(b_left.uv_second, b_right.uv_second, t);
    if (0..2).any(|axis| {
        (a_uv[axis] - b_uv[axis]).abs() > parameter_tolerance
            || (a_st[axis] - b_st[axis]).abs() > parameter_tolerance
    }) {
        return Err(unresolved(line!()));
    }
    let a_uv_box = padded_box(
        a_left.uv_first,
        a_right.uv_first,
        first_domain,
        parameter_tolerance,
    );
    let a_st_box = padded_box(
        a_left.uv_second,
        a_right.uv_second,
        second_domain,
        parameter_tolerance,
    );
    let b_uv_box = padded_box(
        b_left.uv_first,
        b_right.uv_first,
        first_domain,
        parameter_tolerance,
    );
    let b_st_box = padded_box(
        b_left.uv_second,
        b_right.uv_second,
        second_domain,
        parameter_tolerance,
    );
    let a = retrace_point(
        first,
        second,
        a_uv,
        a_st,
        a_uv_box,
        a_st_box,
        geometric_tolerance_mm,
    )?;
    let b = retrace_point(
        first,
        second,
        b_uv,
        b_st,
        b_uv_box,
        b_st_box,
        geometric_tolerance_mm,
    )?;
    if !distance(&a.point, &b.point).is_finite()
        || distance(&a.point, &b.point) > geometric_tolerance_mm
    {
        return Err(unresolved(line!()));
    }
    Ok(RetracedPair {
        parameter,
        seam_wrap: a_left.seam_wrap,
        a,
        b,
    })
}

/// Enforce interval common-branch coverage with an explicit work budget.
pub fn arbitrate_near_degenerate_branches_with_limits(
    first: &Surface,
    second: &Surface,
    a: &ContinuedBranch,
    b: &ContinuedBranch,
    geometric_tolerance_mm: f64,
    parameter_tolerance: f64,
    limits: NearDegenerateSsiLimits,
) -> Result<NearDegenerateSsiArbitration> {
    if !geometric_tolerance_mm.is_finite()
        || geometric_tolerance_mm <= 0.
        || !parameter_tolerance.is_finite()
        || parameter_tolerance <= 0.
    {
        return Err(Error::new(
            "NURBS_SSI_ARBITRATION_INPUT",
            "SSI arbitration needs finite positive geometric and parameter tolerances",
        ));
    }
    if limits.max_samples < 2 || limits.max_interval_work == 0 || limits.max_depth == 0 {
        return Err(Error::new(
            "NURBS_SSI_ARBITRATION_INPUT",
            "SSI arbitration limits need at least two samples and positive work/depth",
        ));
    }
    admit_surface(first)?;
    admit_surface(second)?;
    if !valid_enclosure(a.geometry_enclosure) || !valid_enclosure(b.geometry_enclosure) {
        return Err(unresolved(line!()));
    }
    let separation = enclosure_gap(a.geometry_enclosure, b.geometry_enclosure);
    if !separation.is_finite() {
        return Err(unresolved(line!()));
    }
    if separation > geometric_tolerance_mm {
        return Ok(NearDegenerateSsiArbitration::DistinctByEnclosures);
    }
    if a.samples.len() < 2
        || a.samples.len() != b.samples.len()
        || a.contact != ContactClass::Transverse
        || b.contact != ContactClass::Transverse
        || a.junction
        || b.junction
        || a.closed != b.closed
    {
        return Err(unresolved(line!()));
    }
    if a.samples.len() > limits.max_samples {
        return Err(budget("SSI arbitration sample budget exceeded"));
    }
    let first_domain = surface_domain(first);
    let second_domain = surface_domain(second);
    let mut nodes = Vec::with_capacity(a.samples.len());
    for (left, right) in a.samples.iter().zip(&b.samples) {
        if left.parameter != right.parameter || !left.parameter.is_finite() {
            return Err(unresolved(line!()));
        }
        if nodes
            .last()
            .is_some_and(|previous: &RetracedPair| left.parameter <= previous.parameter)
        {
            return Err(unresolved(line!()));
        }
        nodes.push(retrace_branch_pair(
            first,
            second,
            left,
            left,
            right,
            right,
            left.parameter,
            0.,
            first_domain,
            second_domain,
            geometric_tolerance_mm,
            parameter_tolerance,
        )?);
    }

    let mut guard = Budget::with_iterations(limits.max_interval_work)?
        .guard("ssi_near_degenerate_interval_retrace");
    let mut interval_work = 0_usize;
    let mut intervals = Vec::new();
    let mut samples = vec![nodes[0].sample()];
    let mut max_residual_mm = nodes
        .iter()
        .map(|node| node.residual_mm())
        .fold(0_f64, f64::max);

    for (segment_index, segment) in nodes.windows(2).enumerate() {
        let [left, right] = segment else {
            return Err(unresolved(line!()));
        };
        let mut pending = VecDeque::from([(*left, *right, 0_usize)]);
        while let Some((lo, hi, depth)) = pending.pop_front() {
            guard
                .tick()
                .map_err(|_| budget("SSI interval re-trace work budget exceeded"))?;
            interval_work += 1;
            let parameter = [lo.parameter, hi.parameter];
            if !(parameter[0].is_finite()
                && parameter[1].is_finite()
                && parameter[0] < parameter[1])
            {
                return Err(unresolved(line!()));
            }
            let mid_parameter = midpoint(parameter[0], parameter[1]);
            if mid_parameter <= parameter[0] || mid_parameter >= parameter[1] {
                return Err(unresolved(line!()));
            }
            let original = [&a.samples[segment_index], &a.samples[segment_index + 1]];
            let original_b = [&b.samples[segment_index], &b.samples[segment_index + 1]];
            let t = (mid_parameter - original[0].parameter)
                / (original[1].parameter - original[0].parameter);
            let mid = retrace_branch_pair(
                first,
                second,
                original[0],
                original[1],
                original_b[0],
                original_b[1],
                mid_parameter,
                t,
                first_domain,
                second_domain,
                geometric_tolerance_mm,
                parameter_tolerance,
            )?;
            let (a_uv, a_st) =
                paired_boxes(lo.a, hi.a, first_domain, second_domain, parameter_tolerance);
            let (b_uv, b_st) =
                paired_boxes(lo.b, hi.b, first_domain, second_domain, parameter_tolerance);
            let enclosures = [
                surface_enclosure(first, a_uv)?,
                surface_enclosure(second, a_st)?,
                surface_enclosure(first, b_uv)?,
                surface_enclosure(second, b_st)?,
            ];
            let separated = enclosure_gap(enclosures[0], enclosures[1]) > geometric_tolerance_mm
                || enclosure_gap(enclosures[2], enclosures[3]) > geometric_tolerance_mm
                || enclosure_gap(enclosures[0], enclosures[2]) > geometric_tolerance_mm
                || enclosure_gap(enclosures[1], enclosures[3]) > geometric_tolerance_mm;
            if separated {
                if depth >= limits.max_depth {
                    return Err(unresolved(line!()));
                }
                pending.push_back((lo, mid, depth + 1));
                pending.push_back((mid, hi, depth + 1));
                continue;
            }
            let mut enclosure = enclosures.into_iter().reduce(union_enclosure).unwrap();
            for point in [lo, mid, hi] {
                enclosure = union_enclosure(
                    enclosure,
                    union_enclosure(
                        enclosure_of(point.a.point, point.a.residual_mm),
                        enclosure_of(point.b.point, point.b.residual_mm),
                    ),
                );
            }
            let residual = lo
                .residual_mm()
                .max(mid.residual_mm())
                .max(hi.residual_mm());
            max_residual_mm = max_residual_mm.max(residual);
            intervals.push(CertifiedSsiInterval {
                parameter,
                first_uv: a_uv,
                second_uv: b_uv,
                first_st: a_st,
                second_st: b_st,
                geometry_enclosure: enclosure,
                max_residual_mm: residual,
            });
            samples.push(mid.sample());
            samples.push(hi.sample());
        }
    }
    intervals.sort_by(|x, y| x.parameter[0].total_cmp(&y.parameter[0]));
    samples.sort_by(|x, y| x.parameter.total_cmp(&y.parameter));
    samples.dedup_by(|x, y| x.parameter == y.parameter);
    if intervals.len() + 1 < nodes.len()
        || intervals.first().map(|x| x.parameter[0]) != nodes.first().map(|x| x.parameter)
        || intervals.last().map(|x| x.parameter[1]) != nodes.last().map(|x| x.parameter)
        || intervals
            .windows(2)
            .any(|pair| pair[0].parameter[1] != pair[1].parameter[0])
    {
        return Err(unresolved(line!()));
    }
    Ok(NearDegenerateSsiArbitration::CommonBranch {
        samples,
        intervals,
        max_residual_mm,
        geometric_tolerance_mm,
        interval_work,
    })
}

pub fn arbitrate_near_degenerate_branches(
    first: &Surface,
    second: &Surface,
    a: &ContinuedBranch,
    b: &ContinuedBranch,
    geometric_tolerance_mm: f64,
    parameter_tolerance: f64,
) -> Result<NearDegenerateSsiArbitration> {
    arbitrate_near_degenerate_branches_with_limits(
        first,
        second,
        a,
        b,
        geometric_tolerance_mm,
        parameter_tolerance,
        NearDegenerateSsiLimits::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(vertical: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![
                    if vertical {
                        vec![0., 0., -1.]
                    } else {
                        vec![0., 0., 0.]
                    },
                    if vertical {
                        vec![0., 0., 1.]
                    } else {
                        vec![0., -1., 0.]
                    },
                ],
                vec![
                    if vertical {
                        vec![1., 0., -1.]
                    } else {
                        vec![1., 0., 0.]
                    },
                    if vertical {
                        vec![1., 0., 1.]
                    } else {
                        vec![1., -1., 0.]
                    },
                ],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn branch(offset: f64) -> ContinuedBranch {
        let samples = [0.25, 0.75]
            .into_iter()
            .map(|u| ContinuationSample {
                point: [u, 0., offset],
                uv_first: [u, 0.0],
                uv_second: [u, 0.5],
                parameter: u,
                seam_wrap: None,
            })
            .collect::<Vec<_>>();
        ContinuedBranch {
            closed: false,
            junction: false,
            contact: ContactClass::Transverse,
            samples,
            first_uv: [0.25, 0.5],
            last_uv: [0.75, 0.5],
            first_st: [0.25, 0.5],
            last_st: [0.75, 0.5],
            location: EndpointLocation::Boundary,
            geometry_enclosure: [[0.25, 0.75], [-1e-8, 1e-8], [offset - 1e-8, offset + 1e-8]],
            coedge_trim: coedge_trim([0.25, 0.75], [0.25, 0.75], [[0, 0], [0, 0]]),
        }
    }

    #[test]
    fn certified_common_branch_covers_every_interval_and_retraces_both_sides() {
        let result = arbitrate_near_degenerate_branches(
            &plane(false),
            &plane(true),
            &branch(0.),
            &branch(1e-10),
            1e-5,
            1e-6,
        )
        .unwrap();
        let NearDegenerateSsiArbitration::CommonBranch {
            intervals,
            samples,
            geometric_tolerance_mm,
            interval_work,
            ..
        } = result
        else {
            panic!("identical transverse branches were not certified")
        };
        assert_eq!(geometric_tolerance_mm, 1e-5);
        assert_eq!(interval_work, intervals.len());
        assert_eq!(intervals[0].parameter, [0.25, 0.75]);
        assert!(
            intervals
                .windows(2)
                .all(|pair| pair[0].parameter[1] == pair[1].parameter[0])
        );
        assert!(
            samples
                .windows(2)
                .all(|pair| pair[0].parameter < pair[1].parameter)
        );
        assert!(
            intervals
                .iter()
                .all(|interval| interval.max_residual_mm <= 1e-5)
        );
    }

    #[test]
    fn separated_enclosures_are_a_proof_not_a_sample_verdict() {
        let separated = arbitrate_near_degenerate_branches(
            &plane(false),
            &plane(true),
            &branch(0.),
            &branch(1.),
            1e-7,
            1e-8,
        )
        .unwrap();
        assert!(matches!(
            separated,
            NearDegenerateSsiArbitration::DistinctByEnclosures
        ));
    }

    #[test]
    fn near_coincident_tangent_and_multibranch_inputs_refuse() {
        for mut candidate in [branch(0.), branch(1e-10), branch(0.)] {
            candidate.contact = ContactClass::EvenTangency;
            let Err(error) = arbitrate_near_degenerate_branches(
                &plane(false),
                &plane(false),
                &branch(0.),
                &candidate,
                1e-5,
                1e-6,
            ) else {
                panic!("ambiguous branch acquired a certificate")
            };
            assert_eq!(error.code, "NURBS_SSI_NEAR_DEGENERATE_UNRESOLVED");
        }
        let mut junction = branch(0.);
        junction.junction = true;
        assert!(
            arbitrate_near_degenerate_branches(
                &plane(false),
                &plane(true),
                &junction,
                &branch(0.),
                1e-5,
                1e-6,
            )
            .is_err()
        );
    }

    #[test]
    fn order_gaps_and_seam_mismatch_cannot_be_accepted_by_samples() {
        let mut reversed = branch(0.);
        reversed.samples.reverse();
        assert!(
            arbitrate_near_degenerate_branches(
                &plane(false),
                &plane(true),
                &branch(0.),
                &reversed,
                1e-5,
                1e-6,
            )
            .is_err()
        );
        let mut gap = branch(0.);
        gap.samples[1].parameter = 0.9;
        assert!(
            arbitrate_near_degenerate_branches(
                &plane(false),
                &plane(true),
                &branch(0.),
                &gap,
                1e-5,
                1e-6,
            )
            .is_err()
        );
        let mut seam = branch(0.);
        seam.samples[0].seam_wrap = Some([[1, 0], [0, 0]]);
        assert!(
            arbitrate_near_degenerate_branches(
                &plane(false),
                &plane(true),
                &branch(0.),
                &seam,
                1e-5,
                1e-6,
            )
            .is_err()
        );
    }

    #[test]
    fn work_budget_and_determinism_hold_for_nearby_family() {
        for i in 0..16 {
            let offset = i as f64 * 1e-12;
            let run = || {
                arbitrate_near_degenerate_branches(
                    &plane(false),
                    &plane(true),
                    &branch(0.),
                    &branch(offset),
                    1e-5,
                    1e-6,
                )
            };
            let first = run().unwrap();
            let second = run().unwrap();
            let (
                NearDegenerateSsiArbitration::CommonBranch {
                    intervals: x,
                    samples: xs,
                    ..
                },
                NearDegenerateSsiArbitration::CommonBranch {
                    intervals: y,
                    samples: ys,
                    ..
                },
            ) = (first, second)
            else {
                panic!("transverse family lost certification")
            };
            assert_eq!(x, y);
            assert_eq!(xs.len(), ys.len());
            for (x, y) in xs.iter().zip(&ys) {
                assert_eq!(x.point.map(f64::to_bits), y.point.map(f64::to_bits));
                assert_eq!(x.uv_first.map(f64::to_bits), y.uv_first.map(f64::to_bits));
                assert_eq!(x.uv_second.map(f64::to_bits), y.uv_second.map(f64::to_bits));
                assert_eq!(x.seam_wrap, y.seam_wrap);
            }
        }
        let mut many = branch(0.);
        many.samples.push(ContinuationSample {
            point: [0.9, 0., 0.],
            uv_first: [0.9, 0.0],
            uv_second: [0.9, 0.5],
            parameter: 0.9,
            seam_wrap: None,
        });
        let Err(error) = arbitrate_near_degenerate_branches_with_limits(
            &plane(false),
            &plane(true),
            &many,
            &many,
            1e-5,
            1e-6,
            NearDegenerateSsiLimits {
                max_samples: 2,
                ..Default::default()
            },
        ) else {
            panic!("sample budget was not enforced")
        };
        assert_eq!(error.code, "NURBS_SSI_ARBITRATION_BUDGET");
    }
}
