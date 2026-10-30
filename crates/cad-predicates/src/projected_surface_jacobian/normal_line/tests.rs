use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn input(bend: bool) -> (SourceArena, Vec<Vec<[LeafRef; 4]>>) {
    let values = (0..3)
        .flat_map(|u| {
            (0..3).flat_map(move |v| {
                [
                    u as f64 * 0.5,
                    if u == 2 { v as f64 * 0.5 } else { 0. },
                    f64::from(bend && u == 2 && v == 2),
                    1.,
                ]
            })
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let a = SourceArena::authored("normal-line-test", 1, values).unwrap();
    let refs = (0..3)
        .map(|u| {
            (0..3)
                .map(|v| std::array::from_fn(|k| a.leaf((u * 3 + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect();
    (a, refs)
}
#[test]
fn one_coefficient_cannot_grant_the_whole_original_normal_row() {
    let (a, refs) = input(true);
    let t = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&a, &t, Limits::default(), None);
    assert_eq!(
        rational_surface_normal_row_line(&mut ctx, &refs, 2, None)
            .unwrap()
            .parallel,
        Some(false)
    );
    let mut outcomes = Vec::new();
    for column in 0..6 {
        let mut ctx = PredicateContext::new(&a, &t, Limits::default(), None);
        outcomes.push(
            rational_surface_normal_row_line(&mut ctx, &refs, 2, Some(column))
                .unwrap()
                .parallel,
        );
    }
    assert_eq!(outcomes[0], Some(true));
    assert!(outcomes.contains(&Some(false)));
    let (a, refs) = input(false);
    let mut ctx = PredicateContext::new(&a, &t, Limits::default(), None);
    assert_eq!(
        rational_surface_normal_row_line(&mut ctx, &refs, 2, None)
            .unwrap()
            .parallel,
        Some(true)
    );
}
#[test]
fn source_provenance_and_exact_work_cannot_be_replaced_by_flags() {
    let (a, refs) = input(false);
    let (_, foreign) = input(false);
    let t = ToleranceContext::default_valid();
    let mut short = PredicateContext::new(
        &a,
        &t,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    let r = rational_surface_normal_row_line(&mut short, &refs, 2, None).unwrap();
    assert!(r.parallel.is_none());
    assert!(r.work_used <= 1);
    let mut ctx = PredicateContext::new(&a, &t, Limits::default(), None);
    assert!(rational_surface_normal_row_line(&mut ctx, &foreign, 2, None).is_err());
    assert!(rational_surface_normal_row_line(&mut ctx, &refs, 6, None).is_err());
}
