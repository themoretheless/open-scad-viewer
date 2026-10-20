//! Source-complete affine clipping and original/stored convex UV isotopy.
use super::*;
use crate::{distance_bounds::Interval as I, exact_products::sum_sign};
use std::cmp::Ordering as O;
#[derive(Clone, Debug)]
pub struct TrimCoverageCertificate {
    pub source_patches: usize,
    pub retained_patches: usize,
    pub excluded_patches: usize,
    pub max_affine_z_error_upper: f64,
    pub max_uv_error_upper: f64,
    pub axial_error_band_upper: f64,
    pub exact_affine_partition: bool,
    pub stored_uv_isotopy_certified: bool,
}
pub(super) fn affine_deviation(s: &Surface, coefficients: [f64; 3]) -> Result<f64> {
    s.validate()?;
    check(
        s.degree_u == 1
            && s.control_points.len() == 2
            && s.knots_u == [0., 0., 1., 1.]
            && s.weights.iter().flatten().all(|&w| w == s.weights[0][0]),
        "Affine thread bound requires constant-weight original U basis",
    )?;
    let [gamma, alpha, beta] = coefficients;
    check(
        coefficients.iter().all(|x| x.is_finite()),
        "Affine clipping coefficients must be finite",
    )?;
    let mut error = 0_f64;
    for row in 0..2 {
        for j in 0..s.control_points[row].len() {
            let mut sum = I::point(0.);
            for &k in &s.knots_v[j + 1..=j + s.degree_v] {
                sum = sum.add(I::point(k))?;
            }
            let greville = sum.div(I::point(s.degree_v as f64))?;
            let expected = I::point(gamma)
                .add(I::point(alpha).mul(I::point(row as f64))?)?
                .add(I::point(beta).mul(greville)?)?;
            let d = I::point(s.control_points[row][j][2]).sub(expected)?;
            error = error.max(d.lo.abs().max(d.hi.abs()));
        }
    }
    Ok(error)
}
fn corner_sign(corner: usize, coeff: [f64; 3], level: f64) -> O {
    let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let p = corners[corner];
    sum_sign(&[
        (coeff[0], 1.),
        (coeff[1], p[0]),
        (coeff[2], p[1]),
        (-level, 1.),
    ])
}
/// Independent rectangle-perimeter enumeration, rather than the constructor's
/// sequential polygon clipping. Parallel planes have no interior crossings.
fn exact_polygon(coeff: [f64; 3], limits: [f64; 2]) -> Vec<ExactUvVertex> {
    let mut out = Vec::new();
    for edge in 0..4 {
        if corner_sign(edge, coeff, limits[0]) != O::Less
            && corner_sign(edge, coeff, limits[1]) != O::Greater
        {
            out.push(ExactUvVertex::Corner { corner: edge });
        }
        let order = if edge < 2 {
            limits
        } else {
            [limits[1], limits[0]]
        };
        for level in order {
            let a = corner_sign(edge, coeff, level);
            let b = corner_sign((edge + 1) % 4, coeff, level);
            if (a == O::Less && b == O::Greater) || (a == O::Greater && b == O::Less) {
                out.push(ExactUvVertex::EdgePlane {
                    edge,
                    z_limit: level,
                });
            }
        }
    }
    out
}
fn cyclic_equal(a: &[ExactUvVertex], b: &[ExactUvVertex]) -> bool {
    a.len() == b.len()
        && (0..a.len()).any(|offset| (0..a.len()).all(|i| a[i] == b[(i + offset) % b.len()]))
}
fn perimeter(v: &ExactUvVertex, p: &[f64]) -> (f64, f64) {
    match v {
        ExactUvVertex::Corner { corner } => (*corner as f64, 0.),
        ExactUvVertex::EdgePlane { edge, .. } => match edge {
            0 => (0., p[0]),
            1 => (1., p[1]),
            2 => (3., -p[0]),
            3 => (4., -p[1]),
            _ => unreachable!(),
        },
    }
}
fn on_plane(v: &ExactUvVertex, coeff: [f64; 3], level: f64) -> bool {
    match v {
        ExactUvVertex::Corner { corner } => corner_sign(*corner, coeff, level) == O::Equal,
        ExactUvVertex::EdgePlane { z_limit, .. } => *z_limit == level,
    }
}
fn stored_isotopy(
    c: &TrimCandidate,
    pcurve: &PcurveApproximation,
    coeff: [f64; 3],
    limits: [f64; 2],
) -> Result<()> {
    let vertices = &c.exact_uv_vertices;
    let points = &pcurve.curve.control_points[..vertices.len()];
    let mut wraps = 0;
    let mut positive = false;
    for i in 0..vertices.len() {
        let next = (i + 1) % vertices.len();
        let a = &points[i];
        let b = &points[next];
        let (oa, ta) = perimeter(&vertices[i], a);
        let (ob, tb) = perimeter(&vertices[next], b);
        match sum_sign(&[(ob, 1.), (tb, 1.), (-oa, 1.), (-ta, 1.)]) {
            O::Less => wraps += 1,
            O::Equal => {
                return Err(crate::input(
                    "Stored thread vertices merge in rectangle perimeter order",
                ));
            }
            O::Greater => {}
        }
        let d = &points[(i + 2) % vertices.len()];
        let orientation = sum_sign(&[
            (a[0], b[1]),
            (b[0], d[1]),
            (d[0], a[1]),
            (-a[1], b[0]),
            (-b[1], d[0]),
            (-d[1], a[0]),
        ]);
        check(
            orientation != O::Less,
            "Stored thread polygon lost convexity",
        )?;
        positive |= orientation == O::Greater;
        for (level, lower) in [(limits[0], true), (limits[1], false)] {
            if on_plane(&vertices[i], coeff, level) && on_plane(&vertices[next], coeff, level) {
                // Exact sign of the left normal dot the affine gradient.
                let sign = sum_sign(&[
                    (-b[1], coeff[1]),
                    (a[1], coeff[1]),
                    (b[0], coeff[2]),
                    (-a[0], coeff[2]),
                ]);
                check(
                    sign == if lower { O::Greater } else { O::Less },
                    "Stored thread cut orientation is unresolved",
                )?;
            }
        }
    }
    check(
        wraps == 1 && positive,
        "Stored thread polygon must preserve strict CCW perimeter order and positive area",
    )
}
/// Recheck complete original patch identities, exact affine polygon partitions,
/// source disjointness and convex stored UV isotopy. Source Z differs from its
/// affine field by an outward original-coefficient bound; rounded clipping adds
/// a separate UV-derived axial band. No global B-rep/solid authority is issued.
pub fn certify_trim_candidates(
    spec: Spec,
    trims: &TrimCandidates,
    max_uv_error: f64,
    axial_tolerance: f64,
    max_patches: usize,
) -> Result<TrimCoverageCertificate> {
    let p = parameters(spec)?;
    let original = patches(spec)?;
    certify_patch_trim_coverage(
        &original.patches,
        spec.starts,
        spec.turns,
        p.lead,
        trims,
        max_uv_error,
        axial_tolerance,
        max_patches,
    )
}
/// Source-complete independent slab coverage for an original five-segment,
/// four-quarter screw inventory. Nominal geometry qualification is separate.
pub fn certify_patch_trim_coverage(
    original: &[Patch],
    starts: usize,
    turns: usize,
    lead: f64,
    trims: &TrimCandidates,
    max_uv_error: f64,
    axial_tolerance: f64,
    max_patches: usize,
) -> Result<TrimCoverageCertificate> {
    check(
        (1..=8).contains(&starts) && (1..=16).contains(&turns) && lead.is_finite() && lead > 0.,
        "Invalid screw source census or lead",
    )?;
    let count = 20 * starts * turns;
    let limits = trims.requested_z_bounds;
    check(
        (1..=2560).contains(&max_patches)
            && count <= max_patches
            && trims.unresolved.is_empty()
            && trims.candidates.len() + trims.excluded.len() == count
            && limits.iter().all(|x| x.is_finite())
            && limits[0] < limits[1]
            && max_uv_error.is_finite()
            && max_uv_error > 0.
            && axial_tolerance.is_finite()
            && axial_tolerance > 0.,
        "Thread trim proof needs complete bounded resolved source census and finite positive tolerances",
    )?;
    check(
        original.len() == count,
        "Incomplete original screw source inventory",
    )?;
    for (i, patch) in original.iter().enumerate() {
        patch.surface.validate()?;
        check(
            (
                patch.start,
                patch.turn,
                patch.quarter,
                patch.profile_segment,
            ) == (i / (20 * turns), (i / 20) % turns, (i / 5) % 4, i % 5),
            "Original screw source roles are not canonical",
        )?;
    }
    let mut seen = vec![false; count];
    let mut bind = |patch: &Patch| -> Result<()> {
        check(
            patch.start < starts
                && patch.turn < turns
                && patch.quarter < 4
                && patch.profile_segment < 5,
            "Invalid original thread patch role",
        )?;
        let index =
            (((patch.start * turns + patch.turn) * 4 + patch.quarter) * 5) + patch.profile_segment;
        check(
            !seen[index] && patch.surface == original[index].surface,
            "Missing, duplicated or changed original trim patch",
        )?;
        seen[index] = true;
        Ok(())
    };
    let mut report = TrimCoverageCertificate {
        source_patches: count,
        retained_patches: trims.candidates.len(),
        excluded_patches: trims.excluded.len(),
        max_affine_z_error_upper: 0.,
        max_uv_error_upper: 0.,
        axial_error_band_upper: 0.,
        exact_affine_partition: true,
        stored_uv_isotopy_certified: true,
    };
    for c in &trims.candidates {
        bind(&c.patch)?;
        let s = &c.patch.surface;
        let gamma = s.control_points[0][0][2];
        let coeff = [gamma, s.control_points[1][0][2] - gamma, lead / 4.];
        check(
            coeff[1] > 0. && coeff[2] > 0. && c.affine_z_coefficients == coeff,
            "Unqualified original clipping coefficients",
        )?;
        let expected = exact_polygon(coeff, limits);
        check(
            expected.len() >= 3 && cyclic_equal(&expected, &c.exact_uv_vertices),
            "Exact affine clipping polygon is incomplete or misordered",
        )?;
        let stored = c.pcurve(max_uv_error)?;
        stored_isotopy(c, &stored, coeff, limits)?;
        let error = affine_deviation(s, coeff)?;
        let gradient = I::point(coeff[1]).add(I::point(coeff[2]))?;
        let band = I::point(error)
            .add(gradient.mul(I::point(stored.max_uv_deviation_upper_bound))?)?
            .hi;
        check(
            band <= axial_tolerance,
            "Stored thread clipping exceeds its axial error band",
        )?;
        report.max_affine_z_error_upper = report.max_affine_z_error_upper.max(error);
        report.max_uv_error_upper = report
            .max_uv_error_upper
            .max(stored.max_uv_deviation_upper_bound);
        report.axial_error_band_upper = report.axial_error_band_upper.max(band);
    }
    for c in &trims.excluded {
        bind(&c.patch)?;
        let lo = c
            .patch
            .surface
            .control_points
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        let hi = c
            .patch
            .surface
            .control_points
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max);
        check(
            (hi < limits[0] || lo > limits[1]) && c.source_z_bounds == [lo, hi],
            "Excluded original patch is not proved disjoint",
        )?;
    }
    check(
        seen.iter().all(|v| *v),
        "Original thread patch coverage is incomplete",
    )?;
    Ok(report)
}
