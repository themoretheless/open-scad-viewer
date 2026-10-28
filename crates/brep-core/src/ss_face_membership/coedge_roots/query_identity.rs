//! Exact equality of original clamped UV Bezier maps, including reversed clocks.
//! This does not equate merely coincident images with different parameter speeds.
use super::*;
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits as ExactLimits, PredicateContext, SourceArena,
    ToleranceContext, rational_bezier_identity,
};
use nurbs_core::curve::Curve;

/// Some(false/true) proves identical/opposite normalized clocks. Native parameter
/// domains may differ. Inputs are never reconstructed, elevated or rounded.
pub(super) fn relation(a: &Curve, b: &Curve, used: &mut usize, max: usize) -> Result<Option<bool>> {
    spend(used, max, 1)?;
    a.validate()?;
    b.validate()?;
    if a == b {
        return Ok(Some(false));
    }
    for c in [a, b] {
        let n = c.control_points.len();
        let d = c.domain();
        if c.periodic
            || !(1..=32).contains(&c.degree)
            || n != c.degree + 1
            || c.control_points[0].len() != 2
            || !c.knots[..n].iter().all(|&t| t == d[0])
            || !c.knots[n..].iter().all(|&t| t == d[1])
        {
            return Ok(None);
        }
    }
    let ac = [&a.control_points[0], &a.control_points[a.degree]];
    let bc = [&b.control_points[0], &b.control_points[b.degree]];
    // Positive, clamped rational endpoints are their literal Euclidean poles.
    // Unequal endpoints disprove equality without consuming exact algebra work.
    for reverse in [false, true] {
        if ac[0] != bc[usize::from(reverse)] || ac[1] != bc[usize::from(!reverse)] {
            continue;
        }
        let raw = [a, b]
            .into_iter()
            .flat_map(|c| {
                c.control_points
                    .iter()
                    .zip(&c.weights)
                    .flat_map(|(p, &w)| [p[0], p[1], 0., w])
            })
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        let source = SourceArena::authored("original-SS-UV-query-identity", 1, raw)
            .map_err(|_| invalid("Invalid original query identity leaves"))?;
        let mut cursor = 0;
        let mut poles: Vec<Vec<_>> = [a, b]
            .iter()
            .map(|c| {
                (0..c.control_points.len())
                    .map(|_| {
                        std::array::from_fn(|_| {
                            let r = source.leaf(cursor).unwrap();
                            cursor += 1;
                            r
                        })
                    })
                    .collect()
            })
            .collect();
        if reverse {
            poles[1].reverse();
        }
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &source,
            &tolerance,
            ExactLimits {
                max_work: (max - *used) as u64,
                ..ExactLimits::default()
            },
            None,
        );
        let proof = rational_bezier_identity(&mut ctx, &poles[0], &poles[1])
            .map_err(|_| invalid("Invalid original query identity request"))?;
        spend(used, max, proof.work_used as usize)?;
        match proof.outcome {
            BezierIdentity::Equal => return Ok(Some(reverse)),
            BezierIdentity::Different => {}
            BezierIdentity::Indeterminate(_) => {
                return Err(unproven("Original query identity proof unresolved"));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Curve; 2] {
        // Exact degree elevation of a genuinely rational line. All authored
        // Euclidean poles remain dyadic; the middle weight is (1+3)/2.
        let a = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.]],
            weights: vec![1., 3.],
            periodic: false,
        };
        let b = Curve {
            degree: 2,
            knots: vec![10., 10., 10., 18., 18., 18.],
            control_points: vec![vec![0., 0.], vec![0.75, 0.75], vec![1., 1.]],
            weights: vec![2., 4., 6.],
            periodic: false,
        };
        [a, b]
    }
    #[test]
    fn original_rational_maps_keep_degree_weights_domains_and_opposite_clocks() {
        let [a, mut b] = pair();
        let before = b.clone();
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, 1000000).unwrap(), Some(false));
        assert!(used > 1);
        assert_eq!(b, before);
        let exact = used;
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, exact).unwrap(), Some(false));
        let mut used = 0;
        assert!(relation(&a, &b, &mut used, exact - 1).is_err());
        assert!(used <= exact - 1);
        b.control_points.reverse();
        b.weights.reverse();
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, 1000000).unwrap(), Some(true));
    }
    #[test]
    fn a_nearby_image_or_different_speed_never_becomes_root_identity() {
        let [a, mut b] = pair();
        b.control_points[1][1] = b.control_points[1][1].next_up();
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, 1000000).unwrap(), None);
        let [a, mut b] = pair();
        // Same straight-line image and endpoints, different rational speed.
        b.weights[1] = 5.;
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, 1000000).unwrap(), None);
        let [a, mut b] = pair();
        b.control_points.reverse();
        b.weights.reverse();
        b.control_points[1][0] = b.control_points[1][0].next_up();
        let mut used = 0;
        assert_eq!(relation(&a, &b, &mut used, 1000000).unwrap(), None);
    }
}
