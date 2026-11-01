use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
#[derive(Clone)]
struct Case {
    c: Vec<[f64; 4]>,
    p: Vec<[f64; 3]>,
    s: Vec<Vec<[f64; 4]>>,
    d: [[f64; 2]; 2],
}
fn fixture() -> Case {
    Case {
        // Exact degree elevation: C=[3t, .75+1.5t, .75t+1.5t^2].
        c: vec![
            [0., 0.75, 0., 1.],
            [1., 1.25, 0.25, 1.],
            [2., 1.75, 1., 1.],
            [3., 2.25, 2.25, 1.],
        ],
        p: vec![[0., 0.25, 1.], [1., 0.75, 2.]],
        s: vec![
            vec![[0., 0., 0., 1.], [0., 3., 0., 1.]],
            vec![[3., 0., 0., 1.], [3., 3., 3., 1.]],
        ],
        d: [[0., 1.], [0., 1.]],
    }
}
fn inspect(f: &Case, work: u64) -> ProjectiveImageDecision {
    let values =
        f.c.iter()
            .flatten()
            .chain(f.p.iter().flatten())
            .chain(f.s.iter().flatten().flatten())
            .chain(f.d.iter().flatten())
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
    let arena = SourceArena::authored("projective-image-test", 1, values).unwrap();
    let c: Vec<_> = (0..f.c.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
        .collect();
    let o = 4 * f.c.len();
    let p: Vec<_> = (0..f.p.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(o + 3 * i + k).unwrap()))
        .collect();
    let o = o + 3 * f.p.len();
    let nv = f.s[0].len();
    let s: Vec<Vec<_>> = (0..f.s.len())
        .map(|i| {
            (0..nv)
                .map(|j| std::array::from_fn(|k| arena.leaf(o + 4 * (i * nv + j) + k).unwrap()))
                .collect()
        })
        .collect();
    let o = o + 4 * f.s.len() * nv;
    let d = std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(o + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: work,
            ..Limits::default()
        },
        None,
    );
    rational_bezier_projective_image(&mut ctx, &c, &p, &s, d).unwrap()
}
#[test]
fn nonplanar_curved_trace_uses_complete_identity_after_deriving_its_clock() {
    let mut f = fixture();
    let r = inspect(&f, 1000000);
    assert_eq!(
        r.outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
    assert!(r.work_used > 0);
    assert_eq!(
        inspect(&f, r.work_used).outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
    for budget in [0, r.work_used - 1] {
        let s = inspect(&f, budget);
        assert!(matches!(s.outcome, ProjectiveImageOutcome::Unproved(_)));
        assert!(s.work_used <= budget);
    }
    f.c.reverse();
    f.p.reverse();
    assert_eq!(
        inspect(&f, 1000000).outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
    for row in &mut f.c {
        row[3] *= 4.;
    }
    for row in &mut f.p {
        row[2] *= 0.5;
    }
    assert_eq!(
        inspect(&f, 1000000).outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
    // Rational carrier, rational world curve and a nontrivial source clock.
    let f = Case {
        c: vec![[0., 0., 0., 1.], [2., 1., 0., 1.5], [3., 3., 3., 2.]],
        p: vec![[0., 0., 1.], [1., 1., 2.]],
        s: vec![
            vec![[0., 0., 0., 1.], [0., 3., 0., 1.]],
            vec![[3., 0., 0., 2.], [3., 3., 3., 2.]],
        ],
        d: [[0., 1.], [0., 1.]],
    };
    assert_eq!(
        inspect(&f, 1000000).outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
    // Curved conic; its first X jet is zero, so derive from a nonzero component.
    let f = Case {
        c: vec![[1., 0., 0., 1.], [1., 1., 0., 1.], [0., 1., 0., 4.]],
        p: vec![[1., 0., 1.], [1., 1., 0.5], [0., 1., 1.]],
        s: vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 1.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 1.]],
        ],
        d: [[0., 1.], [0., 1.]],
    };
    assert_eq!(
        inspect(&f, 1000000).outcome,
        ProjectiveImageOutcome::SameOrientedProjectiveImage
    );
}
#[test]
fn matching_endpoints_and_initial_tangent_do_not_hide_interior_damage() {
    let base = fixture();
    for change in 0..6 {
        let mut f = base.clone();
        match change {
            0 => f.c[2][2] += 0.0625,
            1 => f.s[1][1][2] += 0.0625,
            2 => f.c.reverse(),
            3 => f.p[1][2] = -1.,
            4 => f.p[0][0] = -0.125,
            _ => f.c[1] = f.c[0],
        };
        assert!(
            matches!(
                inspect(&f, 1000000).outcome,
                ProjectiveImageOutcome::Unproved(_)
            ),
            "case {change}"
        );
    }
}
