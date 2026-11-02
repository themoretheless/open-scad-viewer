use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
#[derive(Clone)]
struct Case {
    c: Vec<[f64; 4]>,
    p: Vec<[f64; 3]>,
    s: [[[f64; 4]; 2]; 2],
    d: [[f64; 2]; 2],
}
fn fixture() -> Case {
    Case {
        c: vec![
            [-0.5, 0.5, -0.25, 1.],
            [0., 0.5, 0., 3.],
            [0.5, 0.5, 0.25, 2.],
        ],
        p: vec![[3., 3., 1.], [4., 3., 0.25], [5., 3., 2.]],
        s: [
            [[-1., -1., 1., 1.], [-1., 1., -1., 1.]],
            [[1., -1., -1., 1.], [1., 1., 1., 1.]],
        ],
        d: [[2., 6.], [-3., 5.]],
    }
}
fn inspect(f: &Case, work: u64) -> LineImageDecision {
    let values =
        f.c.iter()
            .flatten()
            .chain(f.p.iter().flatten())
            .chain(f.s.iter().flatten().flatten())
            .chain(f.d.iter().flatten())
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
    let arena = SourceArena::authored("line-image-fixture", 1, values).unwrap();
    let c: Vec<_> = (0..f.c.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
        .collect();
    let o = 4 * f.c.len();
    let p: Vec<_> = (0..f.p.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(o + 3 * i + k).unwrap()))
        .collect();
    let o = o + 3 * f.p.len();
    let s = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(o + 4 * (2 * i + j) + k).unwrap())
        })
    });
    let d =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(o + 16 + 2 * i + j).unwrap()));
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
    rational_bezier_axis_line_image(&mut ctx, &c, &p, s, d).unwrap()
}
#[test]
fn different_positive_clocks_prove_the_same_oriented_image_on_a_nonplanar_patch() {
    let mut f = fixture();
    let r = inspect(&f, 1000000);
    assert_eq!(r.outcome, LineImageOutcome::SameOrientedSegment);
    assert!(r.work_used > 0);
    assert_eq!(
        inspect(&f, r.work_used).outcome,
        LineImageOutcome::SameOrientedSegment
    );
    for budget in [0, r.work_used - 1] {
        let short = inspect(&f, budget);
        assert!(matches!(short.outcome, LineImageOutcome::Unproved(_)));
        assert!(short.work_used <= budget);
    }
    f.c.reverse();
    f.p.reverse();
    assert_eq!(
        inspect(&f, 1000000).outcome,
        LineImageOutcome::SameOrientedSegment
    );
    for row in &mut f.c {
        row[3] *= 8.;
    }
    for row in &mut f.p {
        row[2] *= 0.5;
    }
    for row in f.s.iter_mut().flatten() {
        row[3] *= 4.;
    }
    assert_eq!(
        inspect(&f, 1000000).outcome,
        LineImageOutcome::SameOrientedSegment
    );
    // Genuinely nonuniform carrier weights, not merely a common gauge.
    let mut f = fixture();
    for row in &mut f.p {
        row[1] = 1.;
    }
    for row in &mut f.s {
        row[1][3] = 3.;
    }
    assert_eq!(
        inspect(&f, 1000000).outcome,
        LineImageOutcome::SameOrientedSegment
    );
    // Transpose both source parameter axes; the XYZ image stays unchanged.
    let net = f.s;
    f.s = std::array::from_fn(|i| std::array::from_fn(|j| net[j][i]));
    f.d.swap(0, 1);
    for row in &mut f.p {
        row.swap(0, 1);
    }
    assert_eq!(
        inspect(&f, 1000000).outcome,
        LineImageOutcome::SameOrientedSegment
    );
}
#[test]
fn endpoint_coincidence_cannot_hide_bowing_backtracking_wrong_orientation_or_a_pole() {
    let base = fixture();
    for change in 0..9 {
        let mut f = base.clone();
        match change {
            0 => f.c[1][2] = f64::from_bits(1),
            1 => f.c[1] = [0.75, 0.5, 0.375, 3.],
            2 => f.p[1][0] = 2.75,
            3 => f.p[1][1] = 2.5,
            4 => f.c.reverse(),
            5 => f.c[1][3] = 0.,
            6 => f.p[1][2] = -1.,
            7 => f.s[0][0][3] = 0.,
            _ => f.p[1][0] = 7.,
        }
        let r = inspect(&f, 1000000);
        assert!(
            matches!(r.outcome, LineImageOutcome::Unproved(_)),
            "case {change}: {:?}",
            r.outcome
        );
        assert!(r.work_used <= 1000000);
    }
}
