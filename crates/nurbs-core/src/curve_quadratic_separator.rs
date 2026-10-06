//! Sufficient contact ownership for two tangent-adjacent original UV curves.
use crate::{Result, check, curve::Curve, numeric_err};
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
pub struct Certificate {
    curves: [Curve; 2],
    ends: [usize; 2],
    frame_index: usize,
    beta: f64,
    sides: [Sign; 2],
}
impl Certificate {
    pub fn curves(&self) -> &[Curve; 2] {
        &self.curves
    }
    pub fn ends(&self) -> [usize; 2] {
        self.ends
    }
    pub fn frame_index(&self) -> usize {
        self.frame_index
    }
    pub fn beta(&self) -> f64 {
        self.beta
    }
    pub fn sides(&self) -> [Sign; 2] {
        self.sides
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub coefficients: Option<[Vec<Sign>; 2]>,
    pub exact_reason: Option<cad_predicates::Reason>,
}
/// Every interior point lies strictly on its own side of one quadratic;
/// both other endpoints must have strict signs, so only the original join
/// is shared. This does not prove either curve individually injective.
pub fn certify(
    curves: [&Curve; 2],
    ends: [usize; 2],
    frame_index: usize,
    beta: f64,
    max_work: u64,
) -> Result<Report> {
    check(
        ends.iter().all(|e| *e <= 1)
            && beta.is_finite()
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Choose bounded exact separator inputs",
    )?;
    for c in curves {
        c.validate()?;
        let n = c.control_points.len();
        let d = c.domain();
        check(
            !c.periodic
                && (1..=16).contains(&c.degree)
                && n == c.degree + 1
                && c.control_points.iter().all(|p| p.len() == 2)
                && c.weights.iter().all(|w| *w > 0.)
                && d[0] < d[1]
                && c.knots[..=c.degree].iter().all(|t| *t == d[0])
                && c.knots[n..].iter().all(|t| *t == d[1]),
            "Separator requires positive-weight clamped UV Bezier curves",
        )?;
    }
    check(
        frame_index < curves[0].control_points.len(),
        "Frame index outside original curve",
    )?;
    let indices = std::array::from_fn::<_, 2, _>(|s| ends[s] * curves[s].degree);
    let join = &curves[0].control_points[indices[0]];
    if join != &curves[1].control_points[indices[1]] {
        return Ok(Report {
            certificate: None,
            exact_work: 0,
            coefficients: None,
            exact_reason: None,
        });
    }
    let mut values = Vec::new();
    let mut offsets = Vec::new();
    for c in curves {
        offsets.push(values.len());
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend([p[0], p[1], *w]);
        }
    }
    let beta_index = values.len();
    values.push(beta);
    let arena = SourceArena::authored(
        "original-quadratic-curve-separator",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| numeric_err("Separator source admission failed"))?;
    let leaves = std::array::from_fn::<_, 2, _>(|s| {
        (0..curves[s].control_points.len())
            .map(|i| std::array::from_fn(|a| arena.leaf(offsets[s] + 3 * i + a).unwrap()))
            .collect::<Vec<_>>()
    });
    let frame = [
        std::array::from_fn(|a| leaves[0][indices[0]][a]),
        std::array::from_fn(|a| leaves[0][frame_index][a]),
    ];
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let d = cad_predicates::quadratic_curve_separator(
        &mut ctx,
        [&leaves[0], &leaves[1]],
        frame,
        arena.leaf(beta_index).unwrap(),
    )
    .map_err(|_| numeric_err("Separator predicate admission failed"))?;
    let sides = d.coefficients.as_ref().and_then(|c| {
        let a = c[0].iter().find(|s| **s != Sign::Zero).copied()?;
        let b = c[1].iter().find(|s| **s != Sign::Zero).copied()?;
        if a == b {
            return None;
        }
        let sides = [a, b];
        (0..2)
            .all(|s| {
                c[s].iter().all(|v| *v == Sign::Zero || *v == sides[s])
                    && c[s][2 * indices[s]] == Sign::Zero
                    && c[s][(1 - ends[s]) * 2 * curves[s].degree] == sides[s]
            })
            .then_some(sides)
    });
    // Polynomial degree doubles: the shared endpoint index is doubled too.
    let certificate = sides.map(|sides| Certificate {
        curves: [curves[0].clone(), curves[1].clone()],
        ends,
        frame_index,
        beta,
        sides,
    });
    Ok(Report {
        certificate,
        exact_work: d.work_used,
        coefficients: d.coefficients,
        exact_reason: d.reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve(height: f64, weight: f64) -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., height]],
            weights: vec![1., weight, 1.],
            periodic: false,
        }
    }
    #[test]
    fn tangent_curves_need_a_curved_separator_and_keep_originals() {
        let a = curve(1., 1.);
        let b = curve(2., 1.);
        let r = certify([&a, &b], [0, 0], 1, 3., 1_000_000).unwrap();
        let c = r.certificate.unwrap();
        assert_eq!(c.curves(), &[a.clone(), b.clone()]);
        assert_eq!(c.sides(), [Sign::Negative, Sign::Positive]);
        assert!(
            certify([&a, &b], [0, 0], 1, 1., 1_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        let ar = a.reverse().unwrap();
        let br = b.reverse().unwrap();
        assert!(
            certify([&ar, &br], [1, 1], 1, 3., 1_000_000)
                .unwrap()
                .certificate
                .is_some()
        );
    }
    #[test]
    fn rational_weights_and_wrong_geometry_or_budget_refuse() {
        let a = curve(1., 2.);
        let b = curve(10., 2.);
        assert!(
            certify([&a, &b], [0, 0], 1, 3., 1_000_000)
                .unwrap()
                .certificate
                .is_some()
        );
        assert!(
            certify([&a, &a], [0, 0], 1, 3., 1_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut bad = b.clone();
        bad.control_points[1][1] = -2.;
        assert!(
            certify([&a, &bad], [0, 0], 1, 3., 1_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        bad = b.clone();
        bad.control_points[0][1] = 1e-12;
        assert!(
            certify([&a, &bad], [0, 0], 1, 3., 1_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        let r = certify([&a, &b], [0, 0], 1, 3., 1).unwrap();
        assert!(r.certificate.is_none());
        assert!(r.exact_reason.is_some());
    }
}
