//! Transfer a sealed triple-surface witness to a different ordered SS pair.
//! The selected global point remains owned by its original graph and clock.
use super::*;

/// Roles 0,1,2 denote the witness's first, second and third original surfaces.
/// Bind without changing native parameter periods.
pub fn bind_original_ss_third_port(
    witness: Arc<QualifiedSsThirdSurfacePoint>,
    target: Arc<OriginalSsRootCurve>,
    roles: [usize; 2],
    limits: OriginalSsCurveLimits,
) -> Result<QualifiedOriginalSsPort> {
    bind_original_ss_third_port_with_phase(witness, target, roles, [0; 4], limits)
}

/// Shift target UV coordinates by explicit native periods with exact seam evidence.
pub fn bind_original_ss_third_port_with_phase(
    witness: Arc<QualifiedSsThirdSurfacePoint>,
    target: Arc<OriginalSsRootCurve>,
    roles: [usize; 2],
    phase: [i32; 4],
    limits: OriginalSsCurveLimits,
) -> Result<QualifiedOriginalSsPort> {
    check(
        phase.iter().all(|p| (-2..=2).contains(p)),
        "Third-port phase allowance exceeded",
    )?;
    check(
        roles.iter().all(|r| *r < 3) && roles[0] != roles[1],
        "Invalid triple-surface port roles",
    )?;
    // Sealed witnesses carry exact original-equation identities. No sampled
    // world position or interval overlap is used as an incidence proof.
    let source = witness.source();
    let surfaces = [
        source.original().source().first(),
        source.original().source().second(),
        source.third(),
    ];
    let original = target.original_deflated_span_ideal().ok_or_else(|| {
        crate::input("Third-surface port requires an original deflated target graph")
    })?;
    check(
        target.source().first() == surfaces[roles[0]]
            && target.source().second() == surfaces[roles[1]],
        "Triple-surface target roles do not match original surfaces",
    )?;
    let old = source.original().spans();
    let third = source.third_spans();
    let spans = [old[0], old[1], old[2], old[3], third[0], third[1]];
    let target_spans = original.spans();
    let mut local = std::array::from_fn::<_, 4, _>(|_| [Q::zero(), Q::zero()]);
    check(
        limits.projection.max_work >= 24 && limits.projection.max_coefficient_bits >= 64,
        "Triple-surface port frame budget is insufficient",
    )?;
    for k in 0..4 {
        let role = roles[k / 2];
        let g = 2 * role + k % 2;
        let s = surfaces[role];
        let knots = if k % 2 == 0 { &s.knots_u } else { &s.knots_v };
        let a = membership::q(knots[spans[g]])?;
        let width = membership::q(knots[spans[g] + 1])? - &a;
        let b = membership::q(knots[target_spans[k]])?;
        let target_width = membership::q(knots[target_spans[k] + 1])? - &b;
        check(
            width > Q::zero() && target_width > Q::zero(),
            "Empty triple-surface Cox frame",
        )?;
        let shift = if phase[k] == 0 {
            Q::zero()
        } else {
            let (degree, count, periodic) = if k % 2 == 0 {
                (s.degree_u, s.control_points.len(), s.periodic_u)
            } else {
                (s.degree_v, s.control_points[0].len(), s.periodic_v)
            };
            check(
                periodic && crate::periodic_trim::verify_surface_seam_identity(s, k % 2)?.is_some(),
                "Third-port phase lacks exact original seam identity",
            )?;
            (membership::q(knots[count])? - membership::q(knots[degree])?)
                * Q::from_integer(phase[k].into())
        };
        for j in 0..2 {
            let native = &a + &width * &witness.local_bounds()[g][j] + &shift;
            local[k][j] = (native - &b) / &target_width;
            check(
                local[k][j].numer().bits() <= limits.projection.max_coefficient_bits as u64
                    && local[k][j].denom().bits() <= limits.projection.max_coefficient_bits as u64,
                "Triple-surface port coefficient budget exceeded",
            )?;
        }
        let d = target.coordinate_domain()[k];
        check(
            local[k][0] >= membership::q(d[0])?.max(Q::zero())
                && local[k][1] <= membership::q(d[1])?.min(Q::one()),
            "Triple-surface point leaves target graph or finite Cox span",
        )?;
    }
    // Original equations prove equality on the selected surface pair. Uniform
    // target-fiber uniqueness identifies its point at this exact projected clock.
    let parameter = local[target.free_axis().index()].clone();
    Ok(QualifiedOriginalSsPort {
        point: witness.point().clone(),
        target,
        phase,
        parameter,
        kind: OriginalSsPortKind::OriginalThirdSurface,
        limits,
        third: Some((witness, roles, local)),
    })
}
