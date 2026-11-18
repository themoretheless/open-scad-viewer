//! Finite graph membership of roots already owned by the exact carrier proof.
use super::super::patch_exact_membership::{bound, point};
use super::*;
use num_traits::Zero;
pub(super) enum Decision {
    Outside,
    Unresolved([bool; 4]),
    Contact(OriginalQuadricGraphContact),
}
fn contradiction() -> crate::Error {
    crate::Error::new(
        "NURBS_ROOT_PROOF_CONTRADICTION",
        "Original quadric root contradicts graph coordinate bounds",
    )
}
pub(super) fn contact(
    query: QuadricQuery<'_>,
    root: &OriginalQuadricCarrierRoot,
    signs: &[Option<cad_predicates::Sign>; 4],
) -> Result<Decision> {
    let QuadricQuery {
        c,
        s,
        chart,
        matrix,
        products,
    } = query;
    let span = root.source_spans[0];
    let normalized = root.normalized_original_span_parameters[0];
    let mut parameter = root.parameter;
    let mut position = root.position_bounds;
    let mut ranges = if normalized[0] == normalized[1] {
        let t = q(c.knots[span]) + (q(c.knots[span + 1]) - q(c.knots[span])) * q(normalized[0]);
        let p = point(c, span, &t)?;
        let h = [p[0].clone(), p[1].clone(), p[2].clone(), q(1.)];
        let mut residual = products.iter().map(|v| q(v[0]) * q(v[1])).sum::<Q>();
        for i in 0..4 {
            for j in 0..4 {
                residual += &h[i] * q(matrix[i][j]) * &h[j];
            }
        }
        if !residual.is_zero() {
            return Err(contradiction());
        }
        parameter = bound(&t)?;
        for k in 0..3 {
            position[k] = bound(&p[k])?;
        }
        chart.fractions(&p).map(|x| [x.clone(), x])
    } else {
        // An affine functional of a positive rational spline is inside the
        // exact original active-control hull. Intersect it with the original
        // world box; all affine corner evaluations are rational, not rounded.
        let controls: Vec<_> = c.control_points[span - c.degree..=span]
            .iter()
            .map(|p| chart.fractions(&std::array::from_fn(|k| q(p[k]))))
            .collect();
        let mut corners = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut p = std::array::from_fn(|k| q(position[k][0]));
                p[chart.axes[0]] = q(position[chart.axes[0]][i]);
                p[chart.axes[1]] = q(position[chart.axes[1]][j]);
                corners.push(chart.fractions(&p));
            }
        }
        std::array::from_fn(|axis| {
            let lo = controls
                .iter()
                .map(|p| p[axis].clone())
                .min()
                .unwrap()
                .max(corners.iter().map(|p| p[axis].clone()).min().unwrap());
            let hi = controls
                .iter()
                .map(|p| p[axis].clone())
                .max()
                .unwrap()
                .min(corners.iter().map(|p| p[axis].clone()).max().unwrap());
            [lo, hi]
        })
    };
    for (edge, sign) in signs.iter().enumerate() {
        use cad_predicates::Sign;
        let r = &mut ranges[edge / 2];
        match sign {
            Some(Sign::Zero) => {
                let x = q((edge % 2) as f64);
                *r = [x.clone(), x];
            }
            Some(sign) => {
                if (edge % 2 == 0 && *sign == Sign::Negative)
                    || (edge % 2 == 1 && *sign == Sign::Positive)
                {
                    return Ok(Decision::Outside);
                }
                if edge % 2 == 0 {
                    r[0] = r[0].clone().max(q(0.));
                } else {
                    r[1] = r[1].clone().min(q(1.));
                }
            }
            None => {}
        }
    }
    if ranges.iter().any(|r| r[0] > r[1]) {
        return Err(contradiction());
    }
    if ranges.iter().any(|r| r[1] < Q::zero() || r[0] > q(1.)) {
        return Ok(Decision::Outside);
    }
    if ranges.iter().any(|r| r[0] < Q::zero() || r[1] > q(1.)) {
        return Ok(Decision::Unresolved(std::array::from_fn(|edge| {
            let r = &ranges[edge / 2];
            if edge % 2 == 0 {
                r[0] < q(0.) && r[1] >= q(0.)
            } else {
                r[0] <= q(1.) && r[1] > q(1.)
            }
        })));
    }
    let inverse = chart.inverse_clocks(&ranges)?;
    let natural = |axis: usize| -> Result<[f64; 2]> {
        let (knots, p, n) = if axis == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        let width = q(knots[n]) - q(knots[p]);
        Ok([
            bound(&(q(knots[p]) + &width * &inverse[axis][0]))?[0],
            bound(&(q(knots[p]) + width * &inverse[axis][1]))?[1],
        ])
    };
    Ok(Decision::Contact(OriginalQuadricGraphContact {
        parameter_box: [parameter, natural(0)?, natural(1)?],
        uv_proof: if signs.iter().any(Option::is_some) {
            OriginalQuadricGraphUvProof::QuadricCoordinateSign
        } else if normalized[0] == normalized[1] {
            OriginalQuadricGraphUvProof::ExactSingleton
        } else {
            OriginalQuadricGraphUvProof::OriginalAffineControlBounds
        },
        position_bounds: position,
        source_spans: root.source_spans.clone(),
        multiplicities: root.multiplicities.clone(),
        normalized_original_span_parameters: root.normalized_original_span_parameters.clone(),
    }))
}
