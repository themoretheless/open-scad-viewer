//! Original nominal-clock images and actual stored authored correspondence.
use super::*;
use num_traits::Zero;
fn upper(value: Q) -> Result<f64> {
    let value = if value < Q::zero() { -value } else { value };
    let v = stored(&value)?;
    let out = if q(v) < value { v.next_up() } else { v };
    if !out.is_finite() {
        return Err(error("Edge authored clock error overflow"));
    }
    Ok(out)
}
pub(super) fn transfer(
    source: &Model,
    model: &mut Model,
    edge: usize,
    original_upper: f64,
    tolerance: f64,
    limits: Limits,
    work: &mut Work,
) -> Result<Vec<Relation>> {
    let sd = source.edges[edge].curve.domain();
    let cd = model.edges[edge].curve.domain();
    let mut records = Vec::new();
    for (face, f) in source.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (slot, c) in source.loops[wire].coedges.iter().enumerate() {
                if c.edge != edge {
                    continue;
                }
                let Some(original) = c.trim() else {
                    continue;
                };
                let exact = original.curve_parameter.map(|v| image(v, sd, cd));
                let mut cp = [stored(&exact[0])?, stored(&exact[1])?];
                for i in 0..2 {
                    if original.curve_parameter[i] == sd[0] {
                        cp[i] = cd[0];
                    } else if original.curve_parameter[i] == sd[1] {
                        cp[i] = cd[1];
                    }
                }
                if cp.iter().any(|v| *v < cd[0] || *v > cd[1]) {
                    return Err(error("Authored reclocked interval leaves actual domain"));
                }
                let drift = upper(&exact[0] - q(cp[0]))?.max(upper(&exact[1] - q(cp[1]))?);
                let (clock, cells) = lift::parameter_clock_error_bound(
                    &model.edges[edge].curve,
                    drift,
                    work.remaining(limits)?.max_cells,
                )?;
                work.cells += cells;
                let bound = I::point(original_upper)
                    .add(I::point(clock.ok_or_else(|| {
                        error("Reclocked authored parameter drift remains unproven")
                    })?))?
                    .hi;
                if bound > tolerance {
                    return Err(error(
                        "Reclocked authored interval exceeds original world tolerance",
                    ));
                }
                let transferred = brep_topology::CoedgeTrim {
                    curve_parameter: cp,
                    pcurve_parameter: original.pcurve_parameter,
                    periodic_lift: original.periodic_lift,
                };
                transferred.validate()?;
                let nodes =
                    std::array::from_fn::<_, 2, _>(|i| [cp[i], original.pcurve_parameter[i]]);
                let proof = lift::verify_parameter_correspondence(
                    &model.edges[edge].curve,
                    &c.pcurve,
                    &f.surface,
                    &nodes,
                    c.reversed,
                    tolerance,
                    work.remaining(limits)?.max_cells,
                )?;
                work.cells += proof.cells;
                if proof.status != lift::Status::WithinTolerance {
                    return Err(error("Actual reclocked authored lift remains unproven"));
                }
                model.0.loops[wire].coedges[slot] = c.clone().with_trim(transferred)?;
                records.push(Relation {
                    face,
                    wire,
                    coedge: slot,
                    original,
                    curve_parameters: exact,
                    transferred,
                    original_curve_distance_upper: bound,
                });
            }
        }
    }
    Ok(records)
}
// Unlike Split extraction, clock changes retain original unclamped storage.
// A p-fold active endpoint is its literal active pole; otherwise use the native
// outward original homogeneous enclosure at the actual endpoint parameter.
pub(super) fn endpoints(model: &Model, edge: usize, limits: Limits, work: &mut Work) -> Result<()> {
    let e = &model.edges[edge];
    let c = &e.curve;
    let d = c.domain();
    for end in 0..2 {
        work.remaining(limits)?;
        work.cells += 1;
        let p = if c.knots.iter().filter(|k| **k == d[end]).count() >= c.degree {
            let span = if end == 0 {
                (c.degree..c.control_points.len()).find(|&i| c.knots[i] < c.knots[i + 1])
            } else {
                (c.degree..c.control_points.len())
                    .rev()
                    .find(|&i| c.knots[i] < c.knots[i + 1])
            }
            .unwrap();
            c.control_points[if end == 0 { span - c.degree } else { span }]
                .iter()
                .map(|&v| I::point(v))
                .collect::<Vec<_>>()
        } else {
            nurbs_core::interval_eval::evaluate_interval(c, I::point(d[end]))?
        };
        let vertex = e.vertices[end];
        let tolerance = model
            .tolerance_mm
            .min(
                model
                    .1
                    .local_tolerances_mm
                    .get(&model.1.edges[edge])
                    .copied()
                    .unwrap_or(model.tolerance_mm),
            )
            .min(
                model
                    .1
                    .local_tolerances_mm
                    .get(&model.1.vertices[vertex])
                    .copied()
                    .unwrap_or(model.tolerance_mm),
            );
        let mut bound = I::point(0.);
        for k in 0..3 {
            let v = model.vertices[vertex].point[k];
            if p[k].lo == v && p[k].hi == v {
                continue;
            }
            let delta = p[k].sub(I::point(v))?;
            bound = bound.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
        }
        if bound.hi > tolerance {
            return Err(error(
                "Reclocked actual endpoint exceeds original edge/vertex tolerance",
            ));
        }
    }
    Ok(())
}
