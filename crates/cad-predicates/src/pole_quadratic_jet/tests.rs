use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn source(perturb: bool, weighted: bool) -> Vec<Vec<[f64; 4]>> {
    let q = if perturb { 4 } else { 2 };
    (0..3)
        .map(|i| {
            (0..=q)
                .map(|j| {
                    let x = 3. * i as f64 / 2.;
                    let y = 6. * (i * j) as f64 / (2 * q) as f64;
                    let second = if perturb {
                        [1., 2., 1., 0., 1.][j]
                    } else {
                        [1., 1.25, 3.5][j]
                    };
                    let z = 0.25 * x - 0.5 * y + if i == 2 { second } else { 0. };
                    [
                        3. + x,
                        -7. + y,
                        5. + z,
                        if weighted {
                            2f64.powi(i as i32 - j as i32)
                        } else {
                            1.
                        },
                    ]
                })
                .collect()
        })
        .collect()
}
fn arena(s: &[Vec<[f64; 4]>]) -> (SourceArena, Vec<Vec<[LeafRef; 4]>>) {
    let a = SourceArena::authored(
        "pole-quadratic-jet-tests",
        1,
        s.iter()
            .flatten()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let refs = (0..s.len())
        .map(|i| {
            (0..s[0].len())
                .map(|j| std::array::from_fn(|k| a.leaf((i * s[0].len() + j) * 4 + k).unwrap()))
                .collect()
        })
        .collect();
    (a, refs)
}
#[test]
fn exact_nonbinary_hessian_survives_mobius_parameter_weights() {
    let mut jets = Vec::new();
    for weighted in [false, true] {
        let s = source(false, weighted);
        let (a, refs) = arena(&s);
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
        let r = rational_surface_pole_quadratic_jet(&mut c, &refs, 2).unwrap();
        assert!(
            matches!(r.outcome, BezierIdentity::Equal),
            "{:?} work {}",
            r.outcome,
            r.work_used
        );
        let jet = r.jet.unwrap();
        let b = jet.coefficient_bounds();
        for (v, b) in [0.25, -0.5, 2. / 9., 0.5 / 18., 4. / 36.]
            .into_iter()
            .zip(b)
        {
            assert!(b[0] <= v && v <= b[1], "{b:?} vs {v}");
        }
        let work = r.work_used;
        let mut c = PredicateContext::new(
            &a,
            &t,
            Limits {
                max_work: work - 1,
                ..Limits::default()
            },
            None,
        );
        assert!(
            rational_surface_pole_quadratic_jet(&mut c, &refs, 2)
                .unwrap()
                .jet
                .is_none()
        );
        jets.push(jet);
    }
    let (a, _) = arena(&source(false, false));
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
    assert!(compare_pole_quadratic_jets(&mut c, &jets[0], &jets[1]).unwrap());
}
#[test]
fn agreement_at_three_meridians_does_not_admit_a_quadratic_jet() {
    // Q(v)=1+4v(1-v)(1-2v), equal to 1 at v=0,1/2,1.
    let s = source(true, false);
    let (a, refs) = arena(&s);
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
    let r = rational_surface_pole_quadratic_jet(&mut c, &refs, 2).unwrap();
    assert!(
        matches!(r.outcome, BezierIdentity::Different),
        "{:?}",
        r.outcome
    );
    assert!(r.jet.is_none());
}
#[test]
fn budgets_cancellation_and_foreign_source_refs_never_admit() {
    let (a, refs) = arena(&source(false, false));
    let (b, _) = arena(&source(false, false));
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(
        &a,
        &t,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    assert!(
        rational_surface_pole_quadratic_jet(&mut c, &refs, 2)
            .unwrap()
            .jet
            .is_none()
    );
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut c = PredicateContext::new(&a, &t, Limits::default(), Some(&cancel));
    assert!(
        rational_surface_pole_quadratic_jet(&mut c, &refs, 2)
            .unwrap()
            .jet
            .is_none()
    );
    let mut c = PredicateContext::new(&b, &t, Limits::default(), None);
    assert!(rational_surface_pole_quadratic_jet(&mut c, &refs, 2).is_err());
}

#[test]
fn exact_comparison_rejects_changed_curvature_and_different_world_graphs() {
    let mut jets = Vec::new();
    for axis in [2, 2, 0] {
        let mut s = source(false, false);
        if jets.len() == 1 {
            for p in s[2].iter_mut() {
                p[2] += 1.;
            }
        }
        if axis == 0 {
            for p in s.iter_mut().flatten() {
                let [x, y, z, w] = *p;
                *p = [z, x, y, w];
            }
        }
        let (a, refs) = arena(&s);
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
        let r = rational_surface_pole_quadratic_jet(&mut c, &refs, axis).unwrap();
        assert!(matches!(r.outcome, BezierIdentity::Equal));
        jets.push(r.jet.unwrap());
    }
    let (a, _) = arena(&source(false, false));
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
    assert!(!compare_pole_quadratic_jets(&mut c, &jets[0], &jets[1]).unwrap());
    assert!(compare_pole_quadratic_jets(&mut c, &jets[0], &jets[2]).is_err());
}
