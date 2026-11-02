//! Exact planar profiles under transverse nonnegative homothety. The quotient
//! identifies periodic shores and any explicitly declared collapsed end.
use super::*;
mod frame;
#[derive(Clone, Debug)]
pub struct PeriodicExtrusionReport {
    pub proven: bool,
    pub cells: usize,
    pub pairs: usize,
    /// Present when the exact source-plane height is one world coordinate.
    pub axial_coordinate: Option<usize>,
    pub projection_axes: Option<[usize; 2]>,
    /// Literal source controls P,A,B,Q; height normal is (A-P) cross (B-P)
    /// in exact real arithmetic; Q anchors the opposite homothetic row.
    pub source_frame: Option<[[f64; 3]; 4]>,
    /// Outward scale interval for the opposite row relative to `base_end`.
    pub profile_scale: Option<[f64; 2]>,
    pub base_end: Option<usize>,
    /// Endpoint 0/1 of the NONPERIODIC axis identified as one point.
    pub collapsed_end: Option<usize>,
    /// Regular on every profile span away from the declared collapsed shore.
    pub regular_interior: bool,
    pub reason: &'static str,
}
pub fn certify_periodic_extrusion(
    s: &Surface,
    periodic_axis: usize,
    max_pairs: usize,
    max_cells: usize,
) -> Result<PeriodicExtrusionReport> {
    s.validate()?;
    check(
        periodic_axis < 2 && (1..=100000).contains(&max_pairs) && (1..=100000).contains(&max_cells),
        "Periodic extrusion needs bounded axis/pair/cell work",
    )?;
    let mut out = PeriodicExtrusionReport {
        proven: false,
        cells: 0,
        pairs: 0,
        axial_coordinate: None,
        projection_axes: None,
        source_frame: None,
        profile_scale: None,
        base_end: None,
        collapsed_end: None,
        regular_interior: false,
        reason: "unsupported-extrusion-structure",
    };
    let flags = [s.periodic_u, s.periodic_v];
    if !flags[periodic_axis] || flags[1 - periodic_axis] {
        return Ok(out);
    }
    let (p, knots, n, other_degree, other_knots, other_n) = if periodic_axis == 0 {
        (
            s.degree_u,
            &s.knots_u,
            s.control_points.len(),
            s.degree_v,
            &s.knots_v,
            s.control_points[0].len(),
        )
    } else {
        (
            s.degree_v,
            &s.knots_v,
            s.control_points[0].len(),
            s.degree_u,
            &s.knots_u,
            s.control_points.len(),
        )
    };
    if other_degree != 1
        || other_n != 2
        || other_knots.len() != 4
        || other_knots[0] != other_knots[1]
        || other_knots[2] != other_knots[3]
    {
        return Ok(out);
    }
    let read = n * 12 + knots.len();
    if read >= max_cells {
        out.reason = "work-limit";
        return Ok(out);
    }
    out.cells = read;
    if crate::periodic_trim::verify_surface_seam_identity(s, periodic_axis)?.is_none() {
        out.reason = "original-seam-unproven";
        return Ok(out);
    }
    let pole = |i: usize, j: usize| {
        if periodic_axis == 0 {
            &s.control_points[i][j]
        } else {
            &s.control_points[j][i]
        }
    };
    let weight = |i: usize, j: usize| {
        if periodic_axis == 0 {
            s.weights[i][j]
        } else {
            s.weights[j][i]
        }
    };
    // A constant positive row gauge gives the same translated family under
    // the strictly increasing clock r*t/(1-t+r*t), independently of profile U.
    if (0..n).any(|i| {
        crate::exact_products::sum_sign(&[
            (weight(i, 1), weight(0, 0)),
            (-weight(i, 0), weight(0, 1)),
        ]) != std::cmp::Ordering::Equal
    }) {
        return Ok(out);
    }
    let frame = match frame::derive(s, periodic_axis, n) {
        Ok(f) => f,
        Err(reason) => {
            out.reason = reason;
            return Ok(out);
        }
    };
    out.axial_coordinate = frame.axial_coordinate;
    out.source_frame = Some(frame.source);
    out.profile_scale = Some(frame.scale);
    out.base_end = Some(frame.base_end);
    out.collapsed_end = frame.collapsed_end;
    for axes in frame.axes {
        if out.pairs >= max_pairs || out.cells >= max_cells {
            out.reason = "work-limit";
            return Ok(out);
        }
        let profile = crate::curve::Curve {
            degree: p,
            knots: knots.clone(),
            control_points: (0..n)
                .map(|i| {
                    vec![
                        pole(i, frame.base_end)[axes[0]],
                        pole(i, frame.base_end)[axes[1]],
                    ]
                })
                .collect(),
            weights: (0..n).map(|i| weight(i, frame.base_end)).collect(),
            periodic: true,
        };
        if !crate::curve_partition_agreement::verify_source_closure(&profile)? {
            out.reason = "profile-closure-unproven";
            return Ok(out);
        }
        let Some(parts) = crate::curve_partition_agreement::exact_bezier_span_partition(&profile)?
        else {
            out.reason = "exact-profile-decomposition-unproven";
            return Ok(out);
        };
        if parts.len() < 2 {
            out.reason = "profile-jordan-unproven";
            return Ok(out);
        }
        let r = crate::trim_simplicity::inspect(
            &parts,
            1e-10,
            max_pairs - out.pairs,
            max_cells - out.cells,
        )?;
        out.cells += r.cells;
        out.pairs += r.pairs.len();
        if r.proven_simple {
            if out.cells < max_cells {
                let regularity = crate::curve_regularity::inspect(&profile, max_cells - out.cells)?;
                out.cells += regularity.cells;
                out.regular_interior = regularity.spanwise_regular;
            }
            out.proven = true;
            out.projection_axes = Some(axes);
            out.reason = "simple-original-planar-profile-times-transverse-homothety";
            return Ok(out);
        }
    }
    out.reason = "profile-jordan-unproven";
    Ok(out)
}
