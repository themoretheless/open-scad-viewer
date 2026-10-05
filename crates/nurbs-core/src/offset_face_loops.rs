//! Replace an ordered source-boundary arc with an interior rational contact path.
//! Kept curves and holes retain their definitions. Jordan-region containment
//! requires complete original/new audits and no return to the removed boundary.
//! This builds UV contours, not world end transitions or a sewn B-rep solid.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance, curve_surface_agreement as bounds,
    distance_bounds::Interval as I,
    trim_domain::{Location, TrimDomain},
    trim_region_audit as audit,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    StartConnector,
    Contact,
    EndConnector,
    Kept {
        loop_index: usize,
        curve_index: usize,
    },
}
pub struct Report {
    pub loops: Option<Vec<Vec<Curve>>>,
    pub origins: Vec<Origin>,
    pub original: audit::Report,
    pub replacement: Option<audit::Report>,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    pub removed_pair_checks: usize,
    pub exact_retained_geometry: bool,
    pub region_subset_proven: bool,
    pub reason: &'static str,
}
fn endpoint(c: &Curve, end: bool) -> Option<Vec<f64>> {
    if c.periodic {
        return None;
    }
    let n = c.control_points.len();
    let domain = c.domain();
    if end {
        c.knots[n..]
            .iter()
            .all(|k| *k == domain[1])
            .then(|| c.control_points[n - 1].clone())
    } else {
        c.knots[..=c.degree]
            .iter()
            .all(|k| *k == domain[0])
            .then(|| c.control_points[0].clone())
    }
}
fn projection(p: &[f64], q: &[f64], direction: [f64; 2]) -> Result<I> {
    let mut sum = I::point(0.);
    for k in 0..2 {
        sum = sum.add(
            I::point(p[k])
                .sub(I::point(q[k]))?
                .mul(I::point(direction[k]))?,
        )?;
    }
    Ok(sum)
}
// This is used only after complete original/new injectivity audits. All
// controls except the one common endpoint must lie on a strict half-plane;
// a whole span supported only at that endpoint would violate injectivity.
fn single_endpoint_separation(a: &Curve, b: &Curve, join: &[f64]) -> Result<bool> {
    let mut directions = vec![[1., 0.], [0., 1.]];
    let neighbor = |c: &Curve| -> Option<[f64; 2]> {
        let p = if c.control_points[0].as_slice() == join {
            c.control_points.iter().find(|p| p.as_slice() != join)
        } else {
            c.control_points.iter().rev().find(|p| p.as_slice() != join)
        }?;
        let v = [p[0] - join[0], p[1] - join[1]];
        let length = v[0].hypot(v[1]);
        (length.is_finite() && length > 0.).then(|| [v[0] / length, v[1] / length])
    };
    if let (Some(u), Some(v)) = (neighbor(a), neighbor(b)) {
        directions.push([v[0] - u[0], v[1] - u[1]]);
    }
    for d in directions {
        for sign in [1., -1.] {
            let d = d.map(|x| x * sign);
            let strict = |c: &Curve| -> Result<bool> {
                let mut any = false;
                for p in &c.control_points {
                    if p.as_slice() == join {
                        continue;
                    }
                    any = true;
                    if projection(p, join, d)?.lo <= 0. {
                        return Ok(false);
                    }
                }
                Ok(any)
            };
            let closed = |c: &Curve| -> Result<bool> {
                for p in &c.control_points {
                    if p.as_slice() != join && projection(p, join, d)?.hi > 0. {
                        return Ok(false);
                    }
                }
                Ok(true)
            };
            if (strict(a)? && closed(b)?) || (strict(b)? && closed(a)?) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn connector(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1.; 2],
        periodic: false,
    }
}
/// Contact traversal is supplied explicitly; this function never reverses,
/// trims, resamples or snaps a previously qualified contact curve.
/// The removed arc consists of whole, prequalified source boundary curves.
/// Partial-edge partitioning and world transition curves remain separate.
pub fn splice(
    loops: &[Vec<Curve>],
    loop_index: usize,
    arc_start: usize,
    arc_count: usize,
    contact: &Curve,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    check(
        loop_index < loops.len()
            && arc_start < loops[loop_index].len()
            && arc_count > 0
            && arc_count < loops[loop_index].len(),
        "Choose a nonempty source arc with at least one retained boundary curve",
    )?;
    check(
        (1..=100000).contains(&limits.pairs)
            && (1..=100000).contains(&limits.cells)
            && (1..=1000000).contains(&limits.domain_cells),
        "Source face loop replacement needs bounded positive work",
    )?;
    for c in loops.iter().flatten().chain(std::iter::once(contact)) {
        c.validate()?;
        check(
            c.control_points[0].len() == 2,
            "Source face contours must be two-dimensional",
        )?;
    }
    let original = audit::inspect(
        loops,
        tolerance_uv,
        limits.pairs,
        limits.cells,
        limits.domain_cells,
    )?;
    let mut out = Report {
        loops: None,
        origins: vec![],
        pairs: original.pairs,
        cells: original.cells,
        domain_cells: original.domain_cells,
        original,
        replacement: None,
        removed_pair_checks: 0,
        exact_retained_geometry: false,
        region_subset_proven: false,
        reason: "original-region-unqualified",
    };
    if out.original.valid != Some(true) {
        return Ok(out);
    }
    let old = &loops[loop_index];
    let ordered = (0..old.len())
        .map(|i| (arc_start + i) % old.len())
        .collect::<Vec<_>>();
    let Some(start) = endpoint(&old[ordered[0]], false) else {
        out.reason = "arc-endpoint-unproven";
        return Ok(out);
    };
    let Some(end) = endpoint(&old[ordered[arc_count - 1]], true) else {
        out.reason = "arc-endpoint-unproven";
        return Ok(out);
    };
    let Some(inner_start) = endpoint(contact, false) else {
        out.reason = "contact-endpoint-unproven";
        return Ok(out);
    };
    let Some(inner_end) = endpoint(contact, true) else {
        out.reason = "contact-endpoint-unproven";
        return Ok(out);
    };
    if start == inner_start || end == inner_end {
        out.reason = "boundary-contact-endpoint-unqualified";
        return Ok(out);
    }
    // A full rational image enclosure certifies the contact curve, including
    // both endpoints, inside the original material region (holes excluded).
    if out.domain_cells == limits.domain_cells {
        out.reason = "loop-work-limit";
        return Ok(out);
    }
    let domain = TrimDomain::new(loops, tolerance_uv)?;
    let mut pending = vec![(contact.domain(), 0usize)];
    while let Some((interval, depth)) = pending.pop() {
        // Count every visited parent, even when classification stops early.
        if limits.domain_cells - out.domain_cells < 2 {
            out.reason = "loop-work-limit";
            return Ok(out);
        }
        out.domain_cells += 1;
        let image = bounds::curve_bounds(contact, I::new(interval[0], interval[1])?)?;
        let membership = domain.classify(
            [[image[0].lo, image[0].hi], [image[1].lo, image[1].hi]],
            (limits.domain_cells - out.domain_cells).min(100000),
        )?;
        out.domain_cells += membership.cells;
        match membership.location {
            Location::Inside => continue,
            Location::Outside => {
                out.reason = "contact-not-inside-original-region";
                return Ok(out);
            }
            Location::Unresolved => {}
        }
        let mid = interval[0] * 0.5 + interval[1] * 0.5;
        if depth == 32 || mid <= interval[0] || mid >= interval[1] {
            out.reason = "contact-region-membership-unproven";
            return Ok(out);
        }
        pending.push(([mid, interval[1]], depth + 1));
        pending.push(([interval[0], mid], depth + 1));
    }
    let path = vec![
        connector(start.clone(), inner_start),
        contact.clone(),
        connector(inner_end, end.clone()),
    ];
    let mut replacement = path.clone();
    out.origins = vec![
        Origin::StartConnector,
        Origin::Contact,
        Origin::EndConnector,
    ];
    for &i in &ordered[arc_count..] {
        replacement.push(old[i].clone());
        out.origins.push(Origin::Kept {
            loop_index,
            curve_index: i,
        });
    }
    let mut new_loops = loops.to_vec();
    new_loops[loop_index] = replacement;
    out.exact_retained_geometry = true;
    out.loops = Some(new_loops);
    if out.pairs == limits.pairs
        || out.cells == limits.cells
        || out.domain_cells == limits.domain_cells
    {
        out.reason = "loop-work-limit";
        return Ok(out);
    }
    let r = audit::inspect(
        out.loops.as_ref().unwrap(),
        tolerance_uv,
        limits.pairs - out.pairs,
        limits.cells - out.cells,
        limits.domain_cells - out.domain_cells,
    )?;
    out.pairs += r.pairs;
    out.cells += r.cells;
    out.domain_cells += r.domain_cells;
    let valid = r.valid;
    let winding = r.winding[loop_index];
    out.replacement = Some(r);
    if valid != Some(true) {
        out.reason = "replacement-region-unqualified";
        return Ok(out);
    }
    if winding != out.original.winding[loop_index] {
        out.reason = "replacement-orientation-changed";
        return Ok(out);
    }
    // New-loop simplicity excludes crossings with the retained original arc.
    // These checks exclude crossings with every removed original curve.
    for (pi, p) in path.iter().enumerate() {
        for (ai, &old_index) in ordered[..arc_count].iter().enumerate() {
            if out.pairs == limits.pairs || out.cells == limits.cells {
                out.reason = "loop-work-limit";
                return Ok(out);
            }
            out.pairs += 1;
            out.removed_pair_checks += 1;
            let join = if pi == 0 && ai == 0 {
                Some(&start)
            } else if pi == 2 && ai == arc_count - 1 {
                Some(&end)
            } else {
                None
            };
            if let Some(join) = join {
                if !single_endpoint_separation(p, &old[old_index], join)? {
                    out.reason = "removed-boundary-endpoint-separation-unproven";
                    return Ok(out);
                }
            } else {
                let r = curve_distance::prove_separation(
                    p,
                    &old[old_index],
                    tolerance_uv,
                    limits.cells - out.cells,
                )?;
                out.cells += r.cells;
                if r.distance_interval_mm[0] <= 0. {
                    out.reason = "removed-boundary-separation-unproven";
                    return Ok(out);
                }
            }
        }
    }
    // The contact is inside and each endpoint connector has an interior end.
    // No connector can leave that component without a proven-excluded crossing.
    // Equal winding at retained curves selects the original material side.
    out.region_subset_proven = true;
    out.reason = "replacement-face-contours-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Vec<Curve> {
        let p = [vec![0., 1.], vec![1., 1.], vec![1., 0.], vec![0., 0.]];
        (0..4)
            .map(|i| connector(p[i].clone(), p[(i + 1) % 4].clone()))
            .collect()
    }
    fn limits() -> Limits {
        Limits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 100000,
        }
    }
    fn contact() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.2, 0.7], vec![0.5, 0.5], vec![0.8, 0.7]],
            weights: vec![1., 0.8, 1.],
            periodic: false,
        }
    }
    #[test]
    fn rational_contact_replaces_source_arc_with_exact_retained_definitions() {
        let loops = vec![square()];
        let before = loops.clone();
        let p = contact();
        let r = splice(&loops, 0, 0, 1, &p, 1e-8, limits()).unwrap();
        assert!(r.region_subset_proven, "{}", r.reason);
        assert!(r.exact_retained_geometry);
        let new = &r.loops.as_ref().unwrap()[0];
        assert_eq!(new.len(), 6);
        assert_eq!(new[1], p);
        for i in 1..4 {
            assert_eq!(new[i + 2], loops[0][i]);
        }
        assert_eq!(loops, before);
        assert_eq!(r.removed_pair_checks, 3);
        assert_eq!(r.replacement.as_ref().unwrap().winding, r.original.winding);
    }
    #[test]
    fn holes_wrong_contact_and_work_stop_do_not_gain_replacement_regions() {
        let loops = vec![square()];
        let mut p = contact();
        for q in &mut p.control_points {
            q[1] += 1.;
        }
        let r = splice(&loops, 0, 0, 1, &p, 1e-8, limits()).unwrap();
        assert!(!r.region_subset_proven);
        assert_eq!(r.reason, "contact-not-inside-original-region");
        let mut hole = square();
        for c in &mut hole {
            for p in &mut c.control_points {
                p[0] = 0.4 + 0.2 * p[0];
                p[1] = 0.55 + 0.2 * p[1];
            }
        }
        hole = hole
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        let loops = vec![square(), hole];
        let r = splice(&loops, 0, 0, 1, &contact(), 1e-8, limits()).unwrap();
        assert!(!r.region_subset_proven);
        let work = Limits {
            pairs: 1,
            cells: 1,
            domain_cells: 1,
        };
        let r = splice(&[square()], 0, 0, 1, &contact(), 1e-8, work).unwrap();
        assert!(!r.region_subset_proven);
    }
    #[test]
    fn multiple_selected_curves_and_unaffected_holes_keep_their_ownership() {
        let mut outer = square();
        outer[0] = connector(vec![0., 1.], vec![0.5, 1.]);
        outer.insert(1, connector(vec![0.5, 1.], vec![1., 1.]));
        let mut hole = square();
        for c in &mut hole {
            for p in &mut c.control_points {
                p[0] = 0.4 + 0.2 * p[0];
                p[1] = 0.1 + 0.2 * p[1];
            }
        }
        hole = hole
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        let loops = vec![outer, hole];
        let r = splice(&loops, 0, 0, 2, &contact(), 1e-8, limits()).unwrap();
        assert!(r.region_subset_proven, "{}", r.reason);
        assert_eq!(r.loops.as_ref().unwrap()[1], loops[1]);
        assert_eq!(r.removed_pair_checks, 6);
        assert_eq!(
            r.origins[3],
            Origin::Kept {
                loop_index: 0,
                curve_index: 2
            }
        );
    }
    #[test]
    fn wrapped_arc_in_rotated_chart_keeps_original_curve_addresses() {
        let mut outer = square();
        outer[0] = connector(vec![0., 1.], vec![0.5, 1.]);
        outer.insert(1, connector(vec![0.5, 1.], vec![1., 1.]));
        outer.rotate_left(1);
        let mut p = contact();
        for c in outer.iter_mut().chain(std::iter::once(&mut p)) {
            for q in &mut c.control_points {
                let [x, y] = [q[0], q[1]];
                q[0] = x - y;
                q[1] = x + y;
            }
        }
        let r = splice(&[outer.clone()], 0, 4, 2, &p, 1e-8, limits()).unwrap();
        assert!(r.region_subset_proven, "{}", r.reason);
        let new = &r.loops.as_ref().unwrap()[0];
        assert_eq!(new[1], p);
        for i in 1..4 {
            assert_eq!(new[i + 2], outer[i]);
            assert_eq!(
                r.origins[i + 2],
                Origin::Kept {
                    loop_index: 0,
                    curve_index: i
                }
            );
        }
        assert_eq!(r.removed_pair_checks, 6);
    }

    #[test]
    fn malformed_contact_is_not_hidden_by_exhausted_audit_budget() {
        let mut p = contact();
        p.weights[1] = -1.;
        assert!(
            splice(
                &[square()],
                0,
                0,
                1,
                &p,
                1e-8,
                Limits {
                    pairs: 1,
                    cells: 1,
                    domain_cells: 1
                }
            )
            .is_err()
        );
    }
}
