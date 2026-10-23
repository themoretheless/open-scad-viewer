//! Original Bezier image correspondence through an authored nonlinear clock.
//! Distinct from same-parameter identity and projective-only correspondence.
use crate::{Result, check, curve::Curve, surface::Surface};
#[derive(Clone, Debug, PartialEq)]
pub struct CurveClockCorrespondence {
    sources: [Curve; 2],
    clock: Curve,
    orientation: i8,
    work: u64,
}
impl CurveClockCorrespondence {
    pub fn sources(&self) -> [&Curve; 2] {
        [&self.sources[0], &self.sources[1]]
    }
    pub fn clock(&self) -> &Curve {
        &self.clock
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self, max_work: u64) -> Result<()> {
        let (fresh, work) = prove(&self.sources[0], &self.sources[1], &self.clock, max_work)?;
        check(
            fresh == Some(self.orientation) && work == self.work,
            "Original curve clock correspondence changed or remains unproven",
        )
    }
}
fn bezier(c: &Curve) -> bool {
    !c.periodic
        && c.control_points.len() == c.degree + 1
        && c.knots[..=c.degree].iter().all(|&v| v == c.domain()[0])
        && c.knots[c.degree + 1..].iter().all(|&v| v == c.domain()[1])
}
fn prove(a: &Curve, b: &Curve, clock: &Curve, max_work: u64) -> Result<(Option<i8>, u64)> {
    a.validate()?;
    b.validate()?;
    clock.validate()?;
    check(
        a.control_points[0].len() == b.control_points[0].len()
            && clock.control_points[0].len() == 2
            && max_work <= cad_predicates::MAX_WORK,
        "Curve clock needs matching source dimensions, a scalar (u,0) clock and bounded work",
    )?;
    if !bezier(a)
        || !bezier(b)
        || !bezier(clock)
        || a.degree > 8
        || clock.degree > 8
        || clock.control_points.iter().any(|p| p[1] != 0.)
    {
        return Ok((None, 0));
    }
    let d = a.domain();
    let first = clock.control_points[0][0];
    let last = clock.control_points.last().unwrap()[0];
    let orientation = if first == d[0]
        && last == d[1]
        && clock.control_points.windows(2).all(|p| p[0][0] <= p[1][0])
    {
        1
    } else if first == d[1]
        && last == d[0]
        && clock.control_points.windows(2).all(|p| p[1][0] <= p[0][0])
    {
        -1
    } else {
        return Ok((None, 0));
    };
    // Positive rational Bezier basis and ordered scalar poles prove a strict
    // continuous clock onto the whole original interval, including endpoints.
    let xyz = |p: &Vec<f64>| {
        if p.len() == 2 {
            vec![p[0], p[1], 0.]
        } else {
            p.clone()
        }
    };
    let surface = Surface {
        degree_u: a.degree,
        degree_v: 1,
        knots_u: a.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: a
            .control_points
            .iter()
            .map(|p| vec![xyz(p), xyz(p)])
            .collect(),
        weights: a.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    // Duplicate V rows are structurally the SAME original curve A(u).
    // Appending zero or copying original scalars introduces no rounded net.
    let candidate = Curve {
        control_points: b.control_points.iter().map(xyz).collect(),
        ..b.clone()
    };
    let result =
        crate::curve_surface_agreement::verify_exact(&candidate, clock, &surface, false, max_work)?;
    let Some(result) = result else {
        return Ok((None, 0));
    };
    Ok((
        (result.outcome == cad_predicates::BezierIdentity::Equal).then_some(orientation),
        result.work_used,
    ))
}
pub(super) fn verify_with_work(
    a: &Curve,
    b: &Curve,
    clock: &Curve,
    max_work: u64,
) -> Result<(Option<CurveClockCorrespondence>, u64)> {
    let (orientation, work) = prove(a, b, clock, max_work)?;
    Ok((
        orientation.map(|orientation| CurveClockCorrespondence {
            sources: [a.clone(), b.clone()],
            clock: clock.clone(),
            orientation,
            work,
        }),
        work,
    ))
}
/// Prove B(s)=A(clock(s)) over complete NORMALIZED forward B/clock traversal.
/// Source A degree<=8 and clock degree<=8, actual positive Bezier definitions.
/// The clock may be nonlinear/nonprojective and may reverse orientation. This
/// owns original coefficient identity and full image coverage; no chart,
/// intersection census, isotopy, region or face ownership follows by itself.
pub fn verify_curve_clock_correspondence(
    a: &Curve,
    b: &Curve,
    clock: &Curve,
    max_work: u64,
) -> Result<Option<CurveClockCorrespondence>> {
    Ok(verify_with_work(a, b, clock, max_work)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_clock_receipt_rejects_source_parameter_and_work_damage() {
        let a = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let proof = verify_curve_clock_correspondence(&a, &b, &b, 1_000_000)
            .unwrap()
            .unwrap();
        proof.reverify(1_000_000).unwrap();
        let mut bad = proof.clone();
        bad.sources[1].control_points[1][0] = 0.01;
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof.clone();
        bad.clock.control_points[1][0] = 0.01;
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof.clone();
        bad.orientation = -1;
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof;
        bad.work -= 1;
        assert!(bad.reverify(1_000_000).is_err());
    }
}
