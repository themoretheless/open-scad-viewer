//! Fresh inverse chart construction for a globally bijective quadratic shear.
//! Exact equations and exact candidate coordinates are both required.
//! This certificate alone does not authorize face contacts, shells or Bodies.
use cad_predicates::{
    AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
};
use nurbs_core::{Error, Result, surface::Surface};
pub struct Certificate {
    source: Surface,
    inverse: Surface,
    axes: [usize; 2],
    coefficient: f64,
}
impl Certificate {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn inverse(&self) -> &Surface {
        &self.inverse
    }
    pub fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn coefficient(&self) -> f64 {
        self.coefficient
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
pub fn qualify(
    source: &Surface,
    axes: [usize; 2],
    coefficient: f64,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    if axes.iter().any(|&a| a > 2)
        || axes[0] == axes[1]
        || !coefficient.is_finite()
        || !(1..=100_000_000).contains(&max_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Choose finite shear and bounded exact work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-inverse-shear-layout-unproven",
    };
    if source.degree_u != 2
        || source.degree_v != 2
        || source.periodic_u
        || source.periodic_v
        || source.control_points.len() != 3
        || source.control_points[0].len() != 3
        || source.control_points[0][0].len() != 3
    {
        return Ok(out);
    }
    for knots in [&source.knots_u, &source.knots_v] {
        if knots.len() != 6
            || !knots[..3].iter().all(|&k| k == knots[0])
            || !knots[3..].iter().all(|&k| k == knots[3])
        {
            return Ok(out);
        }
    }
    let weight = source.weights[0][0];
    if weight <= 0. || source.weights.iter().flatten().any(|&w| w != weight) {
        return Ok(out);
    }
    let corners = [(0, 0), (0, 2), (2, 0), (2, 2)];
    let inverse = corners.map(|(i, j)| {
        let mut p = source.control_points[i][j].clone();
        p[axes[1]] -= coefficient * p[axes[0]] * p[axes[0]];
        p
    });
    if inverse.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(out);
    }
    let mut values = source
        .control_points
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    values.push(coefficient);
    values.extend(inverse.iter().flatten().copied());
    let arena = SourceArena::authored(
        "source-inverse-shear",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid original source leaves",
        )
    })?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let refs = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(9 * i + 3 * j + k).unwrap()))
    });
    let identity = cad_predicates::quadratic_shear_chart_identity(
        &mut ctx,
        refs,
        arena.leaf(27).unwrap(),
        axes[0],
        axes[1],
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid chart identity request",
        )
    })?;
    out.exact_work = ctx.work_used();
    out.reason = "source-inverse-shear-equations-unproven";
    if identity.outcome != ParameterIdentity::Equal {
        return Ok(out);
    }
    let original =
        corners.map(|(i, j)| std::array::from_fn(|k| arena.leaf(9 * i + 3 * j + k).unwrap()));
    let proposed =
        std::array::from_fn(|i| std::array::from_fn(|k| arena.leaf(28 + 3 * i + k).unwrap()));
    let coordinates = cad_predicates::quadratic_shear_inverse_corners(
        &mut ctx,
        original,
        proposed,
        arena.leaf(27).unwrap(),
        axes[0],
        axes[1],
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid inverse coordinate request",
        )
    })?;
    out.exact_work = ctx.work_used();
    out.reason = "source-inverse-shear-coordinates-unproven";
    if coordinates.outcome != ParameterIdentity::Equal {
        return Ok(out);
    }
    let inverse = Surface {
        degree_u: 1,
        degree_v: 1,
        periodic_u: false,
        periodic_v: false,
        knots_u: vec![
            source.knots_u[0],
            source.knots_u[0],
            source.knots_u[3],
            source.knots_u[3],
        ],
        knots_v: vec![
            source.knots_v[0],
            source.knots_v[0],
            source.knots_v[3],
            source.knots_v[3],
        ],
        control_points: vec![
            vec![inverse[0].clone(), inverse[1].clone()],
            vec![inverse[2].clone(), inverse[3].clone()],
        ],
        weights: vec![vec![1.; 2]; 2],
    };
    inverse.validate()?;
    out.certificate = Some(Certificate {
        source: source.clone(),
        inverse,
        axes,
        coefficient,
    });
    out.reason = "source-inverse-shear-chart-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_chart_requires_exact_original_equations_and_coordinates() {
        let source = Surface {
            degree_u: 2,
            degree_v: 2,
            periodic_u: false,
            periodic_v: false,
            knots_u: vec![2., 2., 2., 4., 4., 4.],
            knots_v: vec![-1., -1., -1., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., if i == 2 { 0.25 } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![2.; 3]; 3],
        };
        let report = qualify(&source, [0, 2], 0.25, 100000).unwrap();
        let certificate = report.certificate.expect(report.reason);
        assert_eq!(certificate.source(), &source);
        assert_eq!(certificate.axes(), [0, 2]);
        assert_eq!(certificate.coefficient(), 0.25);
        assert_eq!(
            certificate.inverse().control_points,
            vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]]
            ]
        );
        assert_eq!(certificate.inverse().knots_u, vec![2., 2., 4., 4.]);
        let mut damaged = source.clone();
        damaged.control_points[1][1][2] += 1e-12;
        assert!(
            qualify(&damaged, [0, 2], 0.25, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut weighted = source.clone();
        weighted.weights[1][1] = 3.;
        assert!(
            qualify(&weighted, [0, 2], 0.25, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            qualify(&source, [0, 2], 0.25, 1)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            qualify(&source, [0, 2], 0.5, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
}
