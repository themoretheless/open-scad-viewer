//! Original-source closed SS cycles to paired finite-face BRep subdivisions.
//! Both source faces retain both sectors. This does not classify material.
use super::*;
use nurbs_core::ss_intersection as ss;
#[derive(Clone, Copy, Debug, Default)]
pub struct ClosedSsLimits {
    pub branches: ss::ClosedSSBuildLimits,
    pub cycle: ss::ClosedFiberCycleLimits,
    pub trace: ss::ClosedFiberTraceLimits,
    pub cut: ClosedCutLimits,
}
pub struct ClosedSsPartition {
    pub first: ClosedPartition,
    pub second: ClosedPartition,
    pub trace: ss::QualifiedClosedFiberTrace,
    pub coverage_domain: [[f64; 2]; 4],
    pub covers_entire_surfaces: bool,
}
/// Partition one wholly interior regular closed original SS component on both
/// finite faces. All natural-product roots must belong to this one component;
/// extra components, face-boundary contacts and singularities remain refusals.
pub fn partition_closed_ss(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    limits: ClosedSsLimits,
) -> Result<ClosedSsPartition> {
    first.validate()?;
    second.validate()?;
    let a = first
        .faces
        .get(first_face)
        .ok_or_else(|| invalid("Missing first closed SS face"))?;
    let b = second
        .faces
        .get(second_face)
        .ok_or_else(|| invalid("Missing second closed SS face"))?;
    if limits.trace.tolerance_world > first.tolerance_mm.min(second.tolerance_mm) {
        return Err(invalid(
            "Closed SS trace tolerance exceeds original model tolerance",
        ));
    }
    let source = ss::FiberSources::new(&a.surface, &b.surface)?;
    let owner = discover_closed_branches(
        &source,
        [(first, a.outer), (second, b.outer)],
        limits.branches,
    )?;
    let coverage = owner.owner().proof().coverage();
    let coverage_domain = coverage.domain();
    let covers_entire_surfaces = coverage.covers_entire_surfaces();
    if owner.branches().len() != 1 {
        return Err(unproven(
            "Paired closed SS cut requires exactly one complete original component",
        ));
    }
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, limits.cycle)?;
    let trace = ss::construct_closed_fiber_trace(&cycle, limits.trace)?;
    let paths: [Vec<Segment>; 2] = std::array::from_fn(|k| {
        trace
            .segments()
            .iter()
            .map(|s| Segment {
                curve: s.curve().clone(),
                pcurve: s.pcurves()[k].clone(),
            })
            .collect()
    });
    let first = closed::partition_closed(
        first,
        first_face,
        &paths[0],
        limits.trace.tolerance_uv,
        limits.cut,
    )?;
    let second = closed::partition_closed(
        second,
        second_face,
        &paths[1],
        limits.trace.tolerance_uv,
        limits.cut,
    )?;
    Ok(ClosedSsPartition {
        first,
        second,
        trace,
        coverage_domain,
        covers_entire_surfaces,
    })
}

pub(super) fn discover_closed_branches(
    source: &ss::FiberSources,
    faces: [(&Model, usize); 2],
    limits: ss::ClosedSSBuildLimits,
) -> Result<ss::QualifiedClosedFiberBranches> {
    let [a, b] = faces.map(|(model, outer)| closed::outer_uv_control_bounds(model, outer));
    let domain = [a[0], a[1], b[0], b[1]];
    let [first, second] = [
        lifted_search_surface(source.first(), a)?,
        lifted_search_surface(source.second(), b)?,
    ];
    let lifted = if first.is_some() || second.is_some() {
        Some(ss::FiberSources::new(
            first.as_ref().unwrap_or(source.first()),
            second.as_ref().unwrap_or(source.second()),
        )?)
    } else {
        None
    };
    let search = lifted.as_ref().unwrap_or(source);
    let natural =
        [source.first(), source.second()].map(crate::boolean_robustness::surface_uv_domains);
    let natural = [natural[0][0], natural[0][1], natural[1][0], natural[1][1]];
    if domain == natural {
        ss::construct_qualified_closed_ss_branches(search, limits)
    } else {
        ss::construct_qualified_closed_ss_branches_in_box(search, domain, limits)
    }
}

// Preserve the face's lifted UV coordinates so the partition can replay
// the resulting boundary curves against the original periodic carrier.
fn lifted_search_surface(
    source: &crate::Surface,
    bounds: [[f64; 2]; 2],
) -> Result<Option<crate::Surface>> {
    use num_rational::BigRational as Q;
    use num_traits::ToPrimitive;
    use nurbs_core::surface::Axis;

    let native = crate::boolean_robustness::surface_uv_domains(source);
    let mut lifted = None;
    for (index, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let [lo, hi] = native[index];
        let [start, end] = bounds[index];
        if ![source.periodic_u, source.periodic_v][index] || (lo < start && end < hi) {
            continue;
        }
        let exact = |x| Q::from_float(x).ok_or_else(|| invalid("Nonfinite closed SS chart bound"));
        let period = exact(hi)? - exact(lo)?;
        if exact(end)? - exact(start)? >= period {
            return Err(unproven(
                "Lifted closed SS search requires a subperiod face",
            ));
        }
        let offset = ((exact(start)? - exact(lo)?) / &period).floor() * &period;
        let mut chart =
            nurbs_core::surface_edit::lift_two_periods(lifted.as_ref().unwrap_or(source), axis)?;
        let knots = match axis {
            Axis::U => &mut chart.knots_u,
            Axis::V => &mut chart.knots_v,
        };
        for knot in knots {
            let translated = exact(*knot)? + &offset;
            *knot = translated
                .to_f64()
                .filter(|x| Q::from_float(*x).as_ref() == Some(&translated))
                .ok_or_else(|| unproven("Lifted closed SS knot translation is not exact"))?;
        }
        chart.validate()?;
        lifted = Some(chart);
    }
    Ok(lifted)
}
