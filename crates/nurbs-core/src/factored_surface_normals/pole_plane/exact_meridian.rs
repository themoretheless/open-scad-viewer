//! Exact graph first/second jets on complete original natural meridians.
use super::*;
use crate::curve::Curve;
use cad_predicates::{BezierIdentity, LeafRef};
pub struct ExactMeridianJets<'a> {
    surfaces: [&'a Surface; 2],
    curves: [&'a Curve; 2],
    axis: usize,
    forward: [[f64; 2]; 2],
    reversed: [bool; 2],
}
impl<'a> ExactMeridianJets<'a> {
    pub fn forward_weights(&self) -> [[f64; 2]; 2] {
        self.forward
    }
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
    pub fn surfaces(&self) -> [&'a Surface; 2] {
        self.surfaces
    }
    pub fn curves(&self) -> [&'a Curve; 2] {
        self.curves
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
}
pub struct ExactMeridianReport<'a> {
    pub certificate: Option<ExactMeridianJets<'a>>,
    pub outcome: BezierIdentity,
    pub work: u64,
    pub reason: &'static str,
}
fn oriented(s: &Surface, p: &Curve, reverse: bool) -> Option<Surface> {
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || p.periodic
        || p.degree != 1
        || p.control_points.len() != 2
        || p.control_points[0].len() != 2
        || (0..2).any(|a| {
            !(1..=8).contains(&degrees[a])
                || counts[a] != degrees[a] + 1
                || knots[a].len() != 2 * counts[a]
                || knots[a][..counts[a]]
                    .iter()
                    .any(|x| *x != knots[a][degrees[a]])
                || knots[a][counts[a]..]
                    .iter()
                    .any(|x| *x != knots[a][counts[a]])
        })
    {
        return None;
    }
    let domain = [
        [knots[0][degrees[0]], knots[0][counts[0]]],
        [knots[1][degrees[1]], knots[1][counts[1]]],
    ];
    let pd = p.domain();
    if p.knots.len() != 4
        || p.knots[..2].iter().any(|x| *x != pd[0])
        || p.knots[2..].iter().any(|x| *x != pd[1])
    {
        return None;
    }
    let fixed = (0..2).find(|&a| {
        p.control_points[0][a] == p.control_points[1][a]
            && domain[a].contains(&p.control_points[0][a])
    })?;
    let varying = 1 - fixed;
    let raw_reverse = if p.control_points[0][varying] == domain[varying][0]
        && p.control_points[1][varying] == domain[varying][1]
    {
        false
    } else if p.control_points[0][varying] == domain[varying][1]
        && p.control_points[1][varying] == domain[varying][0]
    {
        true
    } else {
        return None;
    };
    let mut out = if varying == 0 {
        s.clone()
    } else {
        view(s, Boundary::VMin)
    };
    if p.control_points[0][fixed] == domain[fixed][1] {
        for row in &mut out.control_points {
            row.reverse();
        }
        for row in &mut out.weights {
            row.reverse();
        }
    }
    if raw_reverse ^ reverse {
        out.control_points.reverse();
        out.weights.reverse();
    }
    Some(out)
}
/// No caller derivative candidates or knot insertion. Original indices alone
/// are permuted/reversed, and ORIGINAL surface/map/UV-line weight bits enter
/// the exact arena. This algebra supplies neither C0 ownership nor regularity.
pub fn prepare_exact_meridian_jets<'a>(
    surfaces: [&'a Surface; 2],
    curves: [&'a Curve; 2],
    reverse: [bool; 2],
    forward: [[f64; 2]; 2],
    axis: usize,
    max_work: u64,
) -> Result<ExactMeridianReport<'a>> {
    check(
        axis < 3
            && (1..=100000000).contains(&max_work)
            && forward.iter().flatten().all(|x| x.is_finite() && *x > 0.),
        "Choose a world projection, original positive maps and bounded exact work",
    )?;
    for s in surfaces {
        s.validate()?;
        check(
            s.control_points[0][0].len() == 3,
            "Use original XYZ meridian surfaces",
        )?;
    }
    for p in curves {
        p.validate()?;
    }
    let mut out = ExactMeridianReport {
        certificate: None,
        outcome: BezierIdentity::Indeterminate(cad_predicates::Reason::MissingProof),
        work: 0,
        reason: "exact-meridian-layout-unsupported",
    };
    let (Some(a), Some(b)) = (
        oriented(surfaces[0], curves[0], reverse[0]),
        oriented(surfaces[1], curves[1], reverse[1]),
    ) else {
        return Ok(out);
    };
    let mut data = Vec::new();
    for s in [&a, &b] {
        for (u, row) in s.control_points.iter().enumerate() {
            for (v, p) in row.iter().enumerate() {
                data.extend([p[0], p[1], p[2], s.weights[u][v]]);
            }
        }
    }
    let at = data.len();
    for side in 0..2 {
        data.extend([
            forward[side][0],
            forward[side][1],
            curves[side].weights[0],
            curves[side].weights[1],
        ]);
    }
    let arena = SourceArena::authored(
        "original-natural-meridian-g2",
        1,
        data.into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let mut index = 0;
    let leaves = [&a, &b].map(|s| {
        s.control_points
            .iter()
            .map(|row| {
                row.iter()
                    .map(|_| {
                        let p = std::array::from_fn(|k| arena.leaf(index + k).unwrap());
                        index += 4;
                        p
                    })
                    .collect()
            })
            .collect::<Vec<Vec<[LeafRef; 4]>>>()
    });
    let maps =
        std::array::from_fn(|s| std::array::from_fn(|k| arena.leaf(at + 4 * s + k).unwrap()));
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
    let r = cad_predicates::rational_natural_meridian_g2(
        &mut ctx,
        [&leaves[0], &leaves[1]],
        maps,
        reverse,
        axis,
    )
    .unwrap();
    out.work = r.work_used;
    out.outcome = r.outcome;
    out.reason = match out.outcome {
        BezierIdentity::Equal => "exact-original-meridian-graph-jets-qualified",
        BezierIdentity::Different => "exact-original-meridian-graph-jets-different",
        _ => "exact-original-meridian-graph-jets-unproven",
    };
    if matches!(out.outcome, BezierIdentity::Equal) {
        out.certificate = Some(ExactMeridianJets {
            surfaces,
            curves,
            axis,
            forward,
            reversed: reverse,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
