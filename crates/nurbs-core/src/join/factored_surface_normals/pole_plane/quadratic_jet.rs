//! Exact original quadratic graph jets. Graph regularity and G2 transfer are
//! separate owner obligations; a successful algebraic jet alone is not G2.
use super::*;
use cad_predicates::{BezierIdentity, PoleQuadraticJet as ExactJet};
pub struct QuadraticJet<'a> {
    source: &'a Surface,
    boundary: Boundary,
    axis: usize,
    point: [f64; 3],
    exact: ExactJet,
}
impl<'a> QuadraticJet<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.source
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    /// gx,gy,Hxx,Hxy,Hyy in cyclic world projection coordinates, relative to
    /// the exact original pole. These enclosures never authorize equality.
    pub fn coefficient_bounds(&self) -> [[f64; 2]; 5] {
        self.exact.coefficient_bounds()
    }
}
pub struct QuadraticJetReport<'a> {
    pub certificate: Option<QuadraticJet<'a>>,
    pub work: u64,
    pub outcome: BezierIdentity,
    pub reason: &'static str,
}
pub struct QuadraticPairReport {
    pub same: Option<bool>,
    pub work: u64,
}
pub fn prepare_quadratic_jet(
    s: &Surface,
    boundary: Boundary,
    axis: usize,
    max_work: u64,
) -> Result<QuadraticJetReport<'_>> {
    s.validate()?;
    check(
        axis < 3 && (1..=100000000).contains(&max_work),
        "Choose a world graph and bounded exact jet work",
    )?;
    let clamped = |p: usize, k: &[f64], n: usize| {
        (1..=8).contains(&p)
            && n == p + 1
            && k.len() == 2 * (p + 1)
            && k[..=p].iter().all(|x| *x == k[p])
            && k[p + 1..].iter().all(|x| *x == k[p + 1])
    };
    check(
        !s.periodic_u
            && !s.periodic_v
            && s.control_points[0][0].len() == 3
            && clamped(s.degree_u, &s.knots_u, s.control_points.len())
            && clamped(s.degree_v, &s.knots_v, s.control_points[0].len()),
        "Use an original 3D single Bezier pole chart",
    )?;
    let view = mirrored(s, boundary);
    let point = std::array::from_fn(|k| view.control_points[0][0][k]);
    let (arena, leaves) = arena(&[&view]);
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
    let r =
        cad_predicates::rational_surface_pole_quadratic_jet(&mut ctx, &leaves[0], axis).unwrap();
    let reason = match r.outcome {
        BezierIdentity::Equal => "original-pole-quadratic-jet-identity",
        BezierIdentity::Different => "original-pole-quadratic-jet-inconsistent",
        _ => "original-pole-quadratic-jet-indeterminate",
    };
    Ok(QuadraticJetReport {
        certificate: r.jet.map(|exact| QuadraticJet {
            source: s,
            boundary,
            axis,
            point,
            exact,
        }),
        work: r.work_used,
        outcome: r.outcome,
        reason,
    })
}
pub fn compare_quadratic_jets(
    a: &QuadraticJet<'_>,
    b: &QuadraticJet<'_>,
    max_work: u64,
) -> Result<QuadraticPairReport> {
    check(
        a.axis == b.axis && a.point == b.point && (1..=100000000).contains(&max_work),
        "Compare exact jets at one original pole in one world graph",
    )?;
    let (arena, _) = arena(&[a.source, b.source]);
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
    let same = cad_predicates::compare_pole_quadratic_jets(&mut ctx, &a.exact, &b.exact).ok();
    Ok(QuadraticPairReport {
        same,
        work: ctx.work_used(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_rational_jets_cover_nonunit_axes_endpoints_and_exact_comparisons() {
        let mut jets = Vec::new();
        let mut sources = Vec::new();
        for weighted in [false, true] {
            for b in [
                Boundary::UMin,
                Boundary::UMax,
                Boundary::VMin,
                Boundary::VMax,
            ] {
                let mut s =
                    super::super::tests::oriented(super::super::tests::source(1, weighted), b);
                s.knots_u = if s.degree_u == 2 {
                    vec![2., 2., 2., 5., 5., 5.]
                } else {
                    vec![2., 2., 5., 5.]
                };
                s.knots_v = if s.degree_v == 2 {
                    vec![-4., -4., -4., 8., 8., 8.]
                } else {
                    vec![-4., -4., 8., 8.]
                };
                sources.push((s, b));
            }
        }
        for (s, b) in &sources {
            let before = s.clone();
            let r = prepare_quadratic_jet(s, *b, 2, 1000000).unwrap();
            let jet = r.certificate.expect(r.reason);
            assert!(std::ptr::eq(jet.surface(), s));
            assert_eq!(*s, before);
            for (target, bounds) in [0., 0., 0.5, 0., 0.]
                .into_iter()
                .zip(jet.coefficient_bounds())
            {
                assert!(bounds[0] <= target && target <= bounds[1]);
            }
            assert!(
                prepare_quadratic_jet(s, *b, 2, r.work - 1)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            jets.push(jet);
        }
        for jet in &jets[1..] {
            assert_eq!(
                compare_quadratic_jets(&jets[0], jet, 1000000).unwrap().same,
                Some(true)
            );
        }
        assert!(
            compare_quadratic_jets(&jets[0], &jets[1], 1)
                .unwrap()
                .same
                .is_none()
        );
    }
}
