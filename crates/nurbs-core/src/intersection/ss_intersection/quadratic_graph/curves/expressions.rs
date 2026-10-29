use super::super::*;
use super::arithmetic::{Arithmetic as B, P, Rational};
use crate::curve::Curve;
pub(super) fn normalized(
    owner: &QualifiedQuadraticGraphSS,
    edge: usize,
    b: &mut B,
) -> Result<Rational> {
    let geometry = &owner.edges()[edge].geometry;
    match geometry {
        OriginalGraphSSEdgeGeometry::Segment(p) => Ok(Rational {
            numerator: (0..2)
                .map(|a| vec![I::point(p[0][a]), I::point(p[1][a])])
                .collect(),
            denominator: vec![I::point(1.); 2],
        }),
        OriginalGraphSSEdgeGeometry::EllipseQuarter(q) => {
            let w = vec![I::point(1.), I::point(1.), I::point(2.)];
            let x = vec![I::point(1.), I::point(1.), I::point(0.)];
            let y = vec![I::point(0.), I::point(1.), I::point(2.)];
            let neg = |p: P| {
                p.into_iter()
                    .map(|v| I {
                        lo: -v.hi,
                        hi: -v.lo,
                    })
                    .collect::<P>()
            };
            let unit = match q {
                0 => [x, y],
                1 => [neg(y), x],
                2 => [neg(x), neg(y)],
                _ => [y, neg(x)],
            };
            let mut radii = [I::point(0.); 2];
            for a in 0..2 {
                let square = b.div(
                    I::point(-owner.data.diagonal[2]),
                    I::point(owner.data.diagonal[a]),
                )?;
                radii[a] = b.sqrt(square)?;
            }
            let mut numerator = Vec::new();
            for a in 0..2 {
                let mut p = b.scale(&w, I::point(owner.data.center[a]))?;
                for j in 0..2 {
                    let scale = b.mul(I::point(owner.data.basis[a][j]), radii[j])?;
                    let term = b.scale(&unit[j], scale)?;
                    p = b.sum(&p, &term)?;
                }
                numerator.push(p);
            }
            Ok(Rational {
                numerator,
                denominator: w,
            })
        }
    }
}
pub(super) fn native(
    owner: &QualifiedQuadraticGraphSS,
    uv: &Rational,
    side: usize,
    b: &mut B,
) -> Result<Rational> {
    let mut out = Vec::new();
    for a in 0..2 {
        let d = owner.data.domains[side][a];
        let span = b.sub(I::point(d[1]), I::point(d[0]))?;
        let base = b.scale(&uv.denominator, I::point(d[0]))?;
        let delta = b.scale(&uv.numerator[a], span)?;
        out.push(b.sum(&base, &delta)?);
    }
    Ok(Rational {
        numerator: out,
        denominator: uv.denominator.clone(),
    })
}
pub(super) fn actual(curve: &Curve, b: &mut B) -> Result<Rational> {
    curve.validate()?;
    check(
        !curve.periodic
            && curve.control_points.len() == curve.degree + 1
            && curve.knots[..curve.degree + 1].iter().all(|k| *k == 0.)
            && curve.knots[curve.degree + 1..].iter().all(|k| *k == 1.),
        "SS proxy must retain one complete clamped clock in0..1",
    )?;
    let mut numerator = Vec::new();
    for a in 0..curve.control_points[0].len() {
        numerator.push(
            curve
                .control_points
                .iter()
                .zip(&curve.weights)
                .map(|(p, w)| b.mul(I::point(p[a]), I::point(*w)))
                .collect::<Result<P>>()?,
        );
    }
    Ok(Rational {
        numerator,
        denominator: curve.weights.iter().copied().map(I::point).collect(),
    })
}
pub(super) fn pullback(
    owner: &QualifiedQuadraticGraphSS,
    curve: &Curve,
    side: usize,
    b: &mut B,
) -> Result<Rational> {
    let mut uv = actual(curve, b)?;
    for a in 0..2 {
        let d = owner.data.domains[side][a];
        if curve.control_points.iter().all(|p| p[a] == d[0]) {
            uv.numerator[a] = vec![I::point(0.); uv.denominator.len()];
            continue;
        }
        if curve.control_points.iter().all(|p| p[a] == d[1]) {
            uv.numerator[a] = uv.denominator.clone();
            continue;
        }
        let span = b.sub(I::point(d[1]), I::point(d[0]))?;
        check(
            span.lo > 0.,
            "Original native span conditioning is unresolved",
        )?;
        let offset = b.scale(&uv.denominator, I::point(d[0]))?;
        for i in 0..uv.denominator.len() {
            let delta = b.sub(uv.numerator[a][i], offset[i])?;
            uv.numerator[a][i] = b.div(delta, span)?;
        }
    }
    world(owner, &uv, side, b)
}
fn height_degree(owner: &QualifiedQuadraticGraphSS, side: usize) -> usize {
    let mut degree = 1;
    for i in 0..3 {
        for j in 0..3 {
            if owner.data.heights[side][i][j] != 0. {
                degree = degree.max(i + j);
            }
        }
    }
    degree
}
pub(super) fn preferred_height(owner: &QualifiedQuadraticGraphSS) -> usize {
    let rank = |side: usize| {
        (
            height_degree(owner, side),
            owner.data.heights[side]
                .iter()
                .flatten()
                .filter(|v| **v != 0.)
                .count(),
        )
    };
    usize::from(rank(1) < rank(0))
}
pub(super) fn constant_height(owner: &QualifiedQuadraticGraphSS) -> Option<f64> {
    let side = preferred_height(owner);
    ((0..3).all(|i| (0..3).all(|j| i + j == 0 || owner.data.heights[side][i][j] == 0.)))
        .then_some(owner.data.heights[side][0][0])
}
pub(super) fn world(
    owner: &QualifiedQuadraticGraphSS,
    uv: &Rational,
    side: usize,
    b: &mut B,
) -> Result<Rational> {
    let degree = height_degree(owner, side);
    let denominator = b.power(&uv.denominator, degree)?;
    let cube = b.power(&uv.denominator, degree - 1)?;
    let mut numerator = Vec::new();
    for a in 0..2 {
        let base = b.scale(&uv.denominator, I::point(owner.data.xy[2 * a]))?;
        let delta = b.scale(&uv.numerator[a], I::point(owner.data.xy[2 * a + 1]))?;
        let p = b.sum(&base, &delta)?;
        numerator.push(b.product(&p, &cube)?);
    }
    let mut height = vec![I::point(0.); degree * uv.degree() + 1];
    for i in 0..3 {
        for j in 0..3 {
            let coefficient = owner.data.heights[side][i][j];
            if coefficient == 0. {
                continue;
            }
            let u = b.power(&uv.numerator[0], i)?;
            let v = b.power(&uv.numerator[1], j)?;
            let w = b.power(&uv.denominator, degree - i - j)?;
            let p = b.product(&u, &v)?;
            let p = b.product(&p, &w)?;
            let p = b.scale(&p, I::point(coefficient))?;
            height = b.sum(&height, &p)?;
        }
    }
    numerator.push(height);
    Ok(Rational {
        numerator,
        denominator,
    })
}
