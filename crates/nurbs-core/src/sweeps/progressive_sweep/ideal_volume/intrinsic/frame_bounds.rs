//! Fresh original orthonormal-frame bounds, never caller-supplied reports.
use super::*;
pub(super) fn tangent_continuity(
    path: &Curve,
    closed: bool,
    out: &mut IdealVolumeReport,
    max: u64,
) -> Result<bool> {
    for endpoints in [false, true].into_iter().filter(|e| !*e || closed) {
        let position = super::super::super::endpoint_jets::certify_position(
            path,
            endpoints,
            max - out.exact_work,
        )?;
        out.exact_work += position.exact_work;
        if !position.certified {
            return Ok(false);
        }
        let direction = super::super::super::endpoint_jets::certify_direction(
            path,
            0,
            endpoints,
            true,
            max - out.exact_work,
        )?;
        out.exact_work += direction.exact_work;
        if !direction.certified {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn jets(
    c: &cell::Context<'_, '_>,
    station: [f64; 2],
    phase: frame::PathPhase,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<frame::Report>> {
    let stage = max.min(out.cells.saturating_add(100000));
    if out.cells == stage {
        return Ok(None);
    }
    out.cells += 1;
    let guide = if c.s.orientation_guide.is_some() {
        if let Some(map) = &c.guide_arc {
            let Some(p) = map.phase(station, &mut out.cells, stage)? else {
                return Ok(None);
            };
            Some(p)
        } else {
            Some(phase)
        }
    } else {
        None
    };
    let result = arc_frames::arc_frame(
        c.s,
        station,
        phase,
        guide,
        c.offset.as_ref(),
        c.transport.as_ref(),
        c.shared_guide,
        &c.zero,
        stage - out.cells,
    )?;
    let Some(result) = result else {
        return Ok(None);
    };
    out.cells += result.cells;
    if result.status != Status::Certified {
        return Ok(None);
    }
    Ok(Some(result))
}

/// R is an exact orthonormal source frame, hence sum |R_i'|²=2|Omega|².
/// All three original first-jet enclosures use the same station units.
pub(super) fn angular(f: &frame::Report) -> Result<f64> {
    let mut sum = I::point(0.);
    for col in [&f.transverse, &f.binormal, &f.longitudinal] {
        let n = I::point(iv::norm(decode(col.as_ref().unwrap().first)?)?.hi);
        sum = sum.add(n.mul(n)?)?;
    }
    Ok(sum.div(I::point(2.))?.hi.sqrt().next_up())
}

/// For A0=[N0,B0,T0], with exact orthonormal [N0,B0,L0],
/// sigma_min(A0)^2=1-sqrt(1-|T0 dot L0|²).
/// Every rounded interval below encloses the complete original anchor field.
/// A lower bound on the absolute dot gives a conservative spectral floor;
/// no inverse proposal or positive pointwise determinant supplies credit.
pub(super) fn singular_floor(tangent: V3, longitudinal: V3) -> Result<I> {
    let dot = iv::dot_tight(tangent, longitudinal)?;
    let lambda = if dot.lo > 0. {
        dot.lo
    } else if dot.hi < 0. {
        -dot.hi
    } else {
        return Ok(I::point(0.));
    };
    let lambda = I::point(lambda.min(1.));
    let squared = lambda.mul(lambda)?;
    let remainder = I::point(1.).sub(squared)?;
    let root = I::point(remainder.hi.max(0.).sqrt().next_up());
    let floor = squared.div(I::point(1.).add(root)?)?;
    Ok(I::point(floor.lo.max(0.)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> IdealVolumeReport {
        IdealVolumeReport {
            embedding_certified: false,
            regularity_certified: false,
            parameter_topology_preserved: false,
            cells: 0,
            exact_work: 0,
            boxes: 0,
            monotonicity_lower: None,
            projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            reason: None,
        }
    }
    #[test]
    fn tangent_rate_integration_requires_original_joins_even_when_the_authored_frame_is_smooth() {
        let mut path = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0.; 3], vec![1., 0., 0.], vec![3., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut complete = report();
        assert!(tangent_continuity(&path, false, &mut complete, 1000000).unwrap());
        // Unequal raw parameter speeds do not invalidate an arc tangent join.
        assert!(complete.exact_work > 0);
        for max in [0, complete.exact_work - 1] {
            let mut short = report();
            assert!(!tangent_continuity(&path, false, &mut short, max).unwrap());
            assert!(short.exact_work <= max);
        }
        // Both one-sided accelerations are zero. Their finite tangent-rate
        // bounds cannot integrate through a missing original tangent identity.
        path.control_points[2][1] = 1.;
        assert!(!tangent_continuity(&path, false, &mut report(), 1000000).unwrap());
        path.control_points[2][1] = f64::from_bits(1);
        assert!(!tangent_continuity(&path, false, &mut report(), 1000000).unwrap());
    }
    #[test]
    fn tilted_anchor_spectral_floor_requires_transversality_and_encloses_exact_gram_eigenvalue() {
        let t = [0., 0., 1.].map(I::point);
        for z in [0.125_f64, 0.5, 0.875] {
            let x = (1. - z * z).sqrt();
            let l = [x, 0., z].map(I::point);
            let lower = singular_floor(t, l).unwrap().lo;
            let exact = z * z / (1. + (1. - z * z).sqrt());
            assert!(lower > 0. && lower <= exact && exact - lower < 1e-14);
        }
        assert_eq!(
            singular_floor(t, [I::point(1.), I::point(0.), I::point(0.)])
                .unwrap()
                .lo,
            0.
        );
        assert_eq!(
            singular_floor(
                t,
                [I::point(1.), I::point(0.), I::new(-0.01, 0.01).unwrap()]
            )
            .unwrap()
            .lo,
            0.
        );
        assert!(
            singular_floor(t, [I::point(0.), I::point(0.), I::point(-1.)])
                .unwrap()
                .lo
                > 0.99
        );
    }
}
