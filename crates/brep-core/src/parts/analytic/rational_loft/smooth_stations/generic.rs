//! Compatible retained Bezier profiles with exact shared adaptive-degree station jets.
//! This only constructs a bounded candidate; regularity/material stay separate.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
mod walls;

fn zero_difference(
    a: [usize; 3],
    b: [usize; 3],
    coefficients: [usize; 3],
    source: &SourceArena,
    ctx: &mut PredicateContext<'_>,
) -> bool {
    let refs = |indices: [usize; 3]| indices.map(|i| source.leaf(i).unwrap());
    cad_predicates::direction_dot3d(ctx, refs(a), refs(b), refs([6, 6, 6]), refs(coefficients))
        .is_ok_and(|proof| proof.outcome == Outcome::Sign(Sign::Zero))
}
fn shared_jets(left: &Surface, right: &Surface, work: &mut u64, max_work: u64) -> bool {
    if left.weights != right.weights || left.control_points.len() != right.control_points.len() {
        return false;
    }
    for (a, b) in left.control_points.iter().zip(&right.control_points) {
        let last = a.len() - 1;
        if a[last] != b[0] {
            return false;
        }
        for k in 0..3 {
            let values = [
                a[last - 2][k],
                a[last - 1][k],
                a[last][k],
                b[0][k],
                b[1][k],
                b[2][k],
                0.,
                1.,
                -1.,
                -2.,
            ]
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .to_vec();
            let Ok(source) = SourceArena::authored("general-station-shared-jets", 1, values) else {
                return false;
            };
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &source,
                &tolerance,
                Limits {
                    max_work: max_work - *work,
                    ..Limits::default()
                },
                None,
            );
            // Equal first and second differences, including nonzero shared
            // curvature. Positive V-constant weights give equal homogeneous
            // jets; equal degree/domain factors cancel on the two sides.
            let valid = zero_difference([1, 3, 6], [2, 4, 6], [7, 8, 6], &source, &mut ctx)
                && zero_difference([0, 1, 6], [5, 4, 6], [7, 9, 6], &source, &mut ctx);
            *work += ctx.work_used();
            if !valid {
                return false;
            }
        }
    }
    true
}

pub(super) fn construct(
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<SmoothStationWalls> {
    walls::build(sections, sharp, closed, quantum, tolerance, max_work)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_nonzero_second_jet_is_exact_and_curvature_mutation_is_refused() {
        // Authored binary64 controls at the observed exponent-boundary case.
        // The shared second difference is nonzero; G2 does not require zero curvature.
        let l2=f64::from_bits(0x3ff1f3b645a1c28c);
        let l1=f64::from_bits(0x3ff84189374bc28e);
        let station=f64::from_bits(0x3ffe8f5c28f5c28f);
        let r1=f64::from_bits(0x40026e978d4fe148);
        let r2=f64::from_bits(0x400595810624e148);
        let surface=|y:[f64;6]| Surface {
            degree_u:1,degree_v:5,knots_u:vec![0.,0.,1.,1.],
            knots_v:[vec![0.;6],vec![1.;6]].concat(),
            control_points:[0.,1.].into_iter().map(|x|y.iter().map(|&v|vec![x,v,0.]).collect()).collect(),
            weights:vec![vec![1.;6];2],periodic_u:false,periodic_v:false,
        };
        let left=surface([0.3,0.7,1.,l2,l1,station]);
        let right=surface([station,r1,r2,3.1,3.5,3.9]);
        let mut work=0;
        assert!(shared_jets(&left,&right,&mut work,10000));
        assert!(work>0&&work<=10000);
        let mut changed=right.clone();
        changed.control_points[0][2][1]=changed.control_points[0][2][1].next_up();
        let mut changed_work=0;
        assert!(!shared_jets(&left,&changed,&mut changed_work,10000),
            "Equal position/first jet must not conceal a changed second jet");
        let mut zero_work=0;
        assert!(!shared_jets(&left,&right,&mut zero_work,0));
        assert_eq!(zero_work,0);
    }
}
