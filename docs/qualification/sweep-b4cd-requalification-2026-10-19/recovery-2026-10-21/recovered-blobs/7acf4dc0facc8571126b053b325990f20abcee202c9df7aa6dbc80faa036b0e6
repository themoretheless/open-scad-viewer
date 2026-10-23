//! Non-isoparametric rational proposals with unchanged original-source proof.
use super::{IntersectionCurve, invalid, unproven};
use nurbs_core::{Result, curve::Curve, curve_surface_agreement as agreement, surface::Surface};

/// Construct a nonlinear UV trace on a ruled Bezier chart against
/// a proposed planar carrier. A sign-definite homogeneous section denominator permits
/// a rational proposal; saved coefficients still need whole-support proof.
/// No sampled section, full SS coverage or trimmed ownership is assumed.
pub fn generate_ruled_plane_cut(
    surfaces: [&Surface; 2],
    endpoints: Option<[[f64; 3]; 2]>,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<IntersectionCurve> {
    construct(surfaces, None, endpoints, tolerance_mm, max_cells)
}
/// Restrict the original rational trace between two already owned chart ends.
/// Edits to the trimmed pcurve and spatial endpoints precede full original-
/// support qualification; root ownership is supplied by the B-rep transaction.
pub fn generate_ruled_plane_cut_between(
    surfaces: [&Surface; 2],
    uv_ends: [[f64; 2]; 2],
    endpoints: [[f64; 3]; 2],
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<IntersectionCurve> {
    construct(
        surfaces,
        Some(uv_ends),
        Some(endpoints),
        tolerance_mm,
        max_cells,
    )
}
fn construct(
    surfaces: [&Surface; 2],
    uv_ends: Option<[[f64; 2]; 2]>,
    endpoints: Option<[[f64; 3]; 2]>,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<IntersectionCurve> {
    for s in surfaces {
        s.validate()?;
    }
    if !(4..=100000).contains(&max_cells)
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || surfaces
            .iter()
            .any(|s| s.periodic_u || s.periodic_v || s.control_points[0][0].len() != 3
                || s.control_points.len().checked_mul(s.control_points[0].len())
                    .is_none_or(|n| n > 4096))
    {
        return Err(invalid("Need bounded nonperiodic 3D section inputs"));
    }
    // Transpose coefficients exactly for proposal algebra only. The returned
    // pcurve is mapped back BEFORE lifting against the original surface.
    let swapped = longitudinal_axis(surfaces[0], surfaces[1])? == 1;
    let transposed;
    let s = if swapped {
        let original = surfaces[0];
        transposed = Surface {
            degree_u: original.degree_v,
            degree_v: original.degree_u,
            knots_u: original.knots_v.clone(),
            knots_v: original.knots_u.clone(),
            control_points: (0..original.control_points[0].len())
                .map(|j| original.control_points.iter().map(|row| row[j].clone()).collect()).collect(),
            weights: (0..original.weights[0].len())
                .map(|j| original.weights.iter().map(|row| row[j]).collect()).collect(),
            periodic_u: original.periodic_v,
            periodic_v: original.periodic_u,
        };
        &transposed
    } else { surfaces[0] };
    let uv_ends = uv_ends.map(|ends| if swapped { ends.map(|p| [p[1], p[0]]) } else { ends });
    let plane = surfaces[1];
    let n = s.control_points.len();
    if !(1..=3).contains(&s.degree_u)
        || s.degree_v != 1
        || n != s.degree_u + 1
        || s.control_points[0].len() != 2
    {
        return Err(unproven(
            "Section needs a degree 1..3 by 1 rational Bezier chart",
        ));
    }
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[n]],
        [s.knots_v[1], s.knots_v[2]],
    ];
    if s.knots_u[..n].iter().any(|&k| k != d[0][0])
        || s.knots_u[n..].iter().any(|&k| k != d[0][1])
        || s.knots_v != vec![d[1][0], d[1][0], d[1][1], d[1][1]]
    {
        return Err(unproven("Section needs one original clamped ruled chart"));
    }
    let origin = &plane.control_points[0][0];
    let normal = carrier_normal(plane)?;
    let signed = |p: &Vec<f64>| (0..3).map(|k| normal[k] * (p[k] - origin[k])).sum::<f64>();
    let delta = signed(&s.control_points[0][1]) - signed(&s.control_points[0][0]);
    let constant_slope = delta != 0.
        && delta.is_finite()
        && s.control_points
            .iter()
            .all(|row| signed(&row[1]) - signed(&row[0]) == delta);
    let polynomial = s.weights.iter().flatten().all(|w| *w == s.weights[0][0]);
    let mut pc = if polynomial && constant_slope {
        Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            weights: vec![1.; n],
            periodic: false,
            control_points: s
                .control_points
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    vec![
                        d[0][0] + (d[0][1] - d[0][0]) * i as f64 / s.degree_u as f64,
                        d[1][0] - (d[1][1] - d[1][0]) * signed(&row[0]) / delta,
                    ]
                })
                .collect(),
        }
    } else {
        // Signed homogeneous boundary polynomials A and B satisfy
        // (1-v)*A+v*B=0, hence normalized v=A/(A-B).
        // Elevate denominator and numerator once to store t*(A-B).
        // Rounded homogeneous coefficients are only proposals; fresh original
        // composition bounds below remain the admission authority.
        let degree = s.degree_u + 1;
        let gauge = s.weights.iter().flatten().copied().fold(0., f64::max);
        let signed = s
            .control_points
            .iter()
            .zip(&s.weights)
            .map(|(row, weights)| {
                [
                    weights[0] / gauge * signed(&row[0]),
                    weights[1] / gauge * signed(&row[1]),
                ]
            })
            .collect::<Vec<_>>();
        let mut w = signed.iter().map(|p| p[0] - p[1]).collect::<Vec<_>>();
        let sign = if w.iter().all(|&x| x > 0.) {
            1.
        } else if w.iter().all(|&x| x < 0.) {
            -1.
        } else {
            return Err(unproven(
                "Section denominator sign is not established by its original controls",
            ));
        };
        for weight in &mut w {
            *weight *= sign;
        }
        let v = signed
            .iter()
            .zip(&w)
            .map(|(p, &weight)| d[1][0] * weight + (d[1][1] - d[1][0]) * sign * p[0])
            .collect::<Vec<_>>();
        let mut weights = Vec::with_capacity(degree + 1);
        let mut points = Vec::with_capacity(degree + 1);
        for k in 0..=degree {
            let alpha = k as f64 / degree as f64;
            let left = if k == 0 { 0. } else { alpha * w[k - 1] };
            let right = if k == degree { 0. } else { (1. - alpha) * w[k] };
            let weight = left + right;
            let numerator = if k == 0 { 0. } else { alpha * v[k - 1] }
                + if k == degree { 0. } else { (1. - alpha) * v[k] };
            if !weight.is_finite() || weight <= 0. {
                return Err(unproven("Rational section weights are not representable"));
            }
            weights.push(weight);
            points.push(vec![
                d[0][0] + (d[0][1] - d[0][0]) * left / weight,
                numerator / weight,
            ]);
        }
        Curve {
            degree,
            knots: [vec![d[0][0]; degree + 1], vec![d[0][1]; degree + 1]].concat(),
            control_points: points,
            weights,
            periodic: false,
        }
    };
    if let Some(ends) = uv_ends {
        if ends
            .iter()
            .any(|p| (0..2).any(|k| !p[k].is_finite() || p[k] < d[k][0] || p[k] > d[k][1]))
            || ends[0][0] >= ends[1][0]
        {
            return Err(invalid("Need increasing owned original-chart section ends"));
        }
        pc = pc.trim(ends[0][0], ends[1][0])?;
        pc.control_points[0] = ends[0].to_vec();
        *pc.control_points.last_mut().unwrap() = ends[1].to_vec();
    }
    if pc
        .control_points
        .iter()
        .any(|p| p.iter().any(|x| !x.is_finite()) || p[1] < d[1][0] || p[1] > d[1][1])
    {
        return Err(unproven("Proposed full section leaves the original chart"));
    }
    if swapped {
        for p in &mut pc.control_points { p.swap(0, 1); }
    }
    let lift = agreement::lift(&pc, surfaces[0], endpoints, tolerance_mm * 0.5, (max_cells - 1) / 2)?;
    let mut cells = 1 + lift.cells;
    let curve = lift
        .curve
        .ok_or_else(|| unproven("Nonlinear original-chart section lift is unresolved"))?;
    let first = lift
        .evidence
        .ok_or_else(|| unproven("Missing nonlinear section evidence"))?;
    if cells == max_cells {
        return Err(unproven("Shared section work exhausted"));
    }
    let projected = nurbs_core::curve_pullback::project(
        &curve,
        plane,
        nurbs_core::curve_pullback::Limits {
            tolerance: tolerance_mm * 0.5,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: max_cells - cells,
        },
    )?;
    cells += projected.agreement_cells;
    let second = projected
        .agreement
        .ok_or_else(|| unproven("Nonlinear section misses its second support"))?;
    let other = projected
        .pcurve
        .ok_or_else(|| unproven("Missing saved second-support section trace"))?;
    if !projected.within_tolerance
        || first.status != agreement::Status::WithinTolerance
        || second.status != agreement::Status::WithinTolerance
    {
        return Err(unproven(
            "Saved nonlinear section is not confirmed on both original supports",
        ));
    }
    Ok(IntersectionCurve {
        curve,
        pcurves: [pc, other],
        agreements: [first, second],
        cells,
    })
}

fn carrier_normal(plane: &Surface) -> Result<[f64; 3]> {
    let axes = (0..3)
        .filter(|&k| {
            plane
                .control_points
                .iter()
                .flatten()
                .all(|p| p[k] == plane.control_points[0][0][k])
        })
        .collect::<Vec<_>>();
    let origin = &plane.control_points[0][0];
    let normal = if axes.len() == 1 {
        let mut n = [0.; 3];
        n[axes[0]] = 1.;
        n
    } else {
        // A rounded carrier only proposes the trace. The complete curve must
        // still lift onto the actual second support, even if it is not planar.
        let a = &plane.control_points[plane.control_points.len() - 1][0];
        let b = plane.control_points[0].last().unwrap();
        let u: [f64; 3] = std::array::from_fn(|k| a[k] - origin[k]);
        let v: [f64; 3] = std::array::from_fn(|k| b[k] - origin[k]);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = n.iter().map(|x| x * x).sum::<f64>().sqrt();
        if !length.is_finite() || length == 0. {
            return Err(unproven("Second support plane carrier is singular"));
        }
        n.map(|x| x / length)
    };
    Ok(normal)
}
/// Choose a proposal chart direction from actual homogeneous carrier slopes.
/// Rounded signs guide construction only; both saved original lifts are required.
pub(super) fn longitudinal_axis(s: &Surface, plane: &Surface) -> Result<usize> {
    let normal = carrier_normal(plane)?;
    let origin = &plane.control_points[0][0];
    let signed = |p: &[f64]| (0..3).map(|k| normal[k]*(p[k]-origin[k])).sum::<f64>();
    for axis in 0..2 {
        let cross_degree = if axis == 0 { s.degree_v } else { s.degree_u };
        let cross_count = if axis == 0 { s.control_points[0].len() } else { s.control_points.len() };
        if cross_degree != 1 || cross_count != 2 { continue; }
        let count = if axis == 0 { s.control_points.len() } else { s.control_points[0].len() };
        let differences = (0..count).map(|i| {
            let (a,b) = if axis == 0 { ((i,0),(i,1)) } else { ((0,i),(1,i)) };
            s.weights[a.0][a.1]*signed(&s.control_points[a.0][a.1])
                -s.weights[b.0][b.1]*signed(&s.control_points[b.0][b.1])
        }).collect::<Vec<_>>();
        if differences.iter().all(|v| v.is_finite() && *v>0.)
            || differences.iter().all(|v| v.is_finite() && *v<0.) { return Ok(axis); }
    }
    Err(unproven("No ruled carrier direction has a bounded sign-definite denominator"))
}
