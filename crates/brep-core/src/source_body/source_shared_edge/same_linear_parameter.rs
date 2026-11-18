use super::*;

// For transverse original linear Beziers, the UV equations themselves give
// an exact rational parameter. Comparing fractions does not round either root.
pub(super) fn same_linear_parameter(
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    reversed: [bool; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    let line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..2].iter().all(|&t| t == d[0])
            && c.knots[2..].iter().all(|&t| t == d[1])
    };
    let mut values = Vec::new();
    for (point, role) in points {
        let (main, other) = match role {
            Role::Boundary => (point.boundary(), point.contact()),
            Role::Contact => (point.contact(), point.boundary()),
        };
        if !line(main) || !line(other) {
            return Ok(false);
        }
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend(p.iter().copied());
            values.push(*w);
        }
        for p in &other.control_points {
            values.extend(p.iter().copied());
        }
    }
    if out.work_used == max_work {
        return Ok(false);
    }
    let arena = SourceArena::authored(
        "source-crossing-parameter",
        1,
        values
            .iter()
            .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("BREP_SOURCE_SHARED_EDGE", "Invalid exact root source"))?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof =
        cad_predicates::line_crossing_parameter_identity_oriented(&mut ctx, main, cutter, reversed)
            .map_err(|_| {
                Error::new(
                    "BREP_SOURCE_SHARED_EDGE",
                    "Invalid exact root identity request",
                )
            })?;
    out.work_used += proof.work_used;
    Ok(proof.outcome == ParameterIdentity::Equal)
}
// Both known source roots must lie inside a box on which a projection of the
// canonical 3D crossing equations has exactly one root. Thus projection cannot
// introduce ambiguity between the two actual roots, even on different charts.
pub(super) fn common_world_root(
    world: &Curve,
    cutter: &Curve,
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    let mut ranges = [[[0.; 2]; 2]; 2];
    for (i, (point, role)) in points.iter().enumerate() {
        let (main, other, index) = match role {
            Role::Boundary => (point.boundary(), point.contact(), 0),
            Role::Contact => (point.contact(), point.boundary(), 1),
        };
        if out.work_used == max_work {
            return Ok(false);
        }
        let Some(proof) = curve_surface_agreement::verify_exact_algebraic(
            cutter,
            other,
            point.surface(),
            false,
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?
        else {
            return Ok(false);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            return Ok(false);
        }
        let mapped = |range: [f64; 2], source: [f64; 2], target: [f64; 2]| -> Result<[f64; 2]> {
            use nurbs_core::interval_eval::Interval as I;
            let normalized = I::new(range[0], range[1])?
                .sub(I::point(source[0]))?
                .div(I::point(source[1]).sub(I::point(source[0]))?)?
                .intersect(0., 1.)?;
            let value = I::point(target[0])
                .add(normalized.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?
                .intersect(target[0], target[1])?;
            Ok([value.lo, value.hi])
        };
        ranges[i] = [
            mapped(point.selector()[index], main.domain(), world.domain())?,
            mapped(point.selector()[1 - index], other.domain(), cutter.domain())?,
        ];
    }
    let box_: [[f64; 2]; 2] = std::array::from_fn(|axis| {
        [
            ranges[0][axis][0].min(ranges[1][axis][0]),
            ranges[0][axis][1].max(ranges[1][axis][1]),
        ]
    });
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let project = |c: &Curve| {
            let mut p = c.clone();
            p.control_points = c
                .control_points
                .iter()
                .map(|v| vec![v[axes[0]], v[axes[1]]])
                .collect();
            p
        };
        out.root_checks += 1;
        if nurbs_core::uv_curve_crossings::certify_box(&project(world), &project(cutter), box_)?
            .state
            == nurbs_core::uv_curve_crossings::State::Unique
        {
            return Ok(true);
        }
    }
    Ok(false)
}
