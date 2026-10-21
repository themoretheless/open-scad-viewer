//! Complete finite-segment clearance against a closed original carrier interval.
//! A zero signed crossing count does not imply clearance: tangencies and pairs
//! of crossings must both block a proposed material connectivity witness.
use super::*;
use branch_neighbourhood::ParameterValue;
use std::cmp::Ordering;

pub(in crate::profile_certificate) fn clear(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    bounds: &[ParameterValue; 2],
    endpoints: &[[Q; 2]; 2],
    refinements: usize,
    matrix: usize,
) -> Result<bool> {
    // Reuse the complete native-domain validation and exact closed endpoint
    // contact test. Budget failures propagate; they are never called clear.
    for point in endpoints {
        if selected_winding::contribution_status(net, domain, bounds, point, refinements, matrix)?
            .is_none()
        {
            return Ok(false);
        }
    }
    if endpoints[0] == endpoints[1] {
        return Ok(true);
    }
    let d = [
        &endpoints[1][0] - &endpoints[0][0],
        &endpoints[1][1] - &endpoints[0][1],
    ];
    let project = |p: &[Q; 2]| [&d[0] * &p[0] + &d[1] * &p[1], &d[0] * &p[1] - &d[1] * &p[0]];
    let a = project(&endpoints[0]);
    let b = project(&endpoints[1]);
    let transformed = net
        .iter()
        .map(|p| {
            [
                &d[0] * &p[0] + &d[1] * &p[1],
                &d[0] * &p[1] - &d[1] * &p[0],
                p[2].clone(),
            ]
        })
        .collect::<Vec<_>>();
    let width = Q::new(1.into(), 1024.into());
    let roots = shore_events::produce(&transformed, domain, 1, &a[1], &width, refinements)?;
    let within = |root: &shore_events::ShoreRoot| -> Result<bool> {
        Ok(selected_winding::compare(
            &root.primitive,
            &root.root,
            domain,
            &bounds[0],
            refinements,
            matrix,
        )? != Ordering::Less
            && selected_winding::compare(
                &root.primitive,
                &root.root,
                domain,
                &bounds[1],
                refinements,
                matrix,
            )? != Ordering::Greater)
    };
    if let Some(roots) = roots {
        let x = power(&transformed, 0);
        let w = power(&transformed, 2);
        for root in roots {
            if !within(&root)? {
                continue;
            }
            let sign = |value: &Q| {
                event_coordinates::sign_at(
                    &sub(&x, &w.iter().map(|q| q * value).collect::<Poly>()),
                    &root.primitive,
                    &root.root,
                    refinements,
                )
            };
            // b.x-a.x = |d|^2 > 0. Positive original weights preserve signs.
            if sign(&a[0])? >= 0 && sign(&b[0])? <= 0 {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    // The entire supporting cell lies on the path line. Endpoint fibres were
    // already excluded above. By continuity on the connected selected interval,
    // one exact interior point decides whether its image is inside the segment.
    let mut cuts = bounds.clone();
    for step in 0..=refinements {
        let lo = parameter_order::range(&cuts[0].value, &cuts[0].interval)[1].clone();
        let hi = parameter_order::range(&cuts[1].value, &cuts[1].interval)[0].clone();
        if lo < hi {
            let t =
                ((&lo + &hi) / Q::from_integer(2.into()) - &domain[0]) / (&domain[1] - &domain[0]);
            let x = at(&power(&transformed, 0), &t) / at(&power(&transformed, 2), &t);
            return Ok(x < a[0] || x > b[0]);
        }
        if step < refinements {
            for cut in &mut cuts {
                parameter_order::refine(&cut.primitive, &mut cut.interval)?;
            }
        }
    }
    Err(refused(
        "Selected clearance interior remains unproven within allowance",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(n: i64) -> Q {
        Q::from_integer(n.into())
    }
    fn bound(n: i64) -> ParameterValue {
        ParameterValue {
            primitive: vec![q(-n), q(1)],
            value: vec![q(0), q(1)],
            interval: [q(n), q(n)],
        }
    }
    #[test]
    fn zero_winding_and_even_tangencies_are_not_clear_paths() {
        let net = [[q(0), q(1), q(1)], [q(1), q(-1), q(1)], [q(2), q(1), q(1)]];
        let cuts = [bound(0), bound(1)];
        let domain = [q(0), q(1)];
        let tangent = [[q(-1), q(0)], [q(3), q(0)]];
        assert_eq!(
            selected_winding::segment(&net, &domain, &cuts, &tangent, 192, 64).unwrap(),
            Some(0)
        );
        assert!(!clear(&net, &domain, &cuts, &tangent, 192, 64).unwrap());
        let crossing = [
            [q(-1), Q::new(1.into(), 2.into())],
            [q(3), Q::new(1.into(), 2.into())],
        ];
        assert_eq!(
            selected_winding::segment(&net, &domain, &cuts, &crossing, 192, 64).unwrap(),
            Some(0)
        );
        assert!(!clear(&net, &domain, &cuts, &crossing, 192, 64).unwrap());
        assert!(
            clear(
                &net,
                &domain,
                &cuts,
                &[[q(-1), q(-1)], [q(3), q(-1)]],
                192,
                64
            )
            .unwrap()
        );
    }
    #[test]
    fn collinear_containment_native_clipping_and_endpoint_contacts() {
        let net = [[q(0), q(0), q(1)], [q(4), q(0), q(1)]];
        let domain = [q(2), q(6)];
        let cuts = [bound(4), bound(6)];
        assert!(
            clear(
                &net,
                &domain,
                &cuts,
                &[[q(-1), q(0)], [q(1), q(0)]],
                192,
                64
            )
            .unwrap()
        );
        assert!(!clear(&net, &domain, &cuts, &[[q(1), q(0)], [q(5), q(0)]], 192, 64).unwrap());
        assert!(!clear(&net, &domain, &cuts, &[[q(1), q(0)], [q(2), q(0)]], 192, 64).unwrap());
        assert!(
            !clear(
                &net,
                &domain,
                &cuts,
                &[[q(3), q(-1)], [q(3), q(1)]],
                192,
                64
            )
            .unwrap()
        );
        assert!(
            clear(
                &net,
                &domain,
                &cuts,
                &[[q(1), q(-1)], [q(1), q(1)]],
                192,
                64
            )
            .unwrap()
        );
        assert!(
            clear(
                &net,
                &domain,
                &[bound(6), bound(4)],
                &[[q(0), q(1)], [q(1), q(1)]],
                192,
                64
            )
            .is_err()
        );
    }
    #[test]
    fn algebraic_native_cuts_exclude_continuations_but_keep_selected_contacts() {
        let net = [[q(0), q(1), q(4)], [q(2), q(-1), q(4)], [q(4), q(1), q(4)]];
        let root = ParameterValue {
            primitive: vec![q(-1), q(0), q(2)],
            value: vec![q(0), q(1)],
            interval: [Q::new(7.into(), 10.into()), Q::new(8.into(), 10.into())],
        };
        let cuts = [root, bound(1)];
        let domain = [q(0), q(1)];
        // x=t,y=(t-1/2)^2; the tangent at t=1/2 is OUTSIDE the selected interval.
        assert!(
            clear(
                &net,
                &domain,
                &cuts,
                &[[q(-1), q(0)], [q(2), q(0)]],
                192,
                64
            )
            .unwrap()
        );
        let y = Q::new(1.into(), 16.into());
        assert!(
            !clear(
                &net,
                &domain,
                &cuts,
                &[[q(-1), y.clone()], [q(2), y]],
                192,
                64
            )
            .unwrap()
        );
        let point = [q(0), q(-1)];
        assert!(clear(&net, &domain, &cuts, &[point.clone(), point], 192, 64).unwrap());
        assert!(
            clear(
                &net,
                &domain,
                &cuts,
                &[[q(-1), q(0)], [q(2), q(0)]],
                513,
                64
            )
            .is_err()
        );
    }
}
