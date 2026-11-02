//! Preserve authored clocks under actual world/UV concatenation. A single
//! stored relation may represent only a connected exactly collinear union.
use super::*;
use brep_topology::CoedgeTrim;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub source_edge: usize,
    pub original: CoedgeTrim,
    /// Exact nominal output-clock images before binary64 storage.
    pub curve_parameters: [Q; 2],
    pub pcurve_parameters: [Q; 2],
    pub transferred: CoedgeTrim,
    pub original_curve_distance_upper: f64,
    pub original_pcurve_distance_upper: f64,
    pub original_lift_distance_upper: f64,
}
fn q(v: f64) -> Q {
    Q::from_float(v).unwrap()
}
fn stored(v: &Q, domain: [f64; 2]) -> Result<f64> {
    let x = v
        .to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| error("Compacted relation clock overflow"))?;
    if x < domain[0] || x > domain[1] {
        return Err(error("Compacted relation leaves its stored domain"));
    }
    Ok(x)
}
fn upper(v: Q) -> Result<f64> {
    let v = if v < Q::zero() { -v } else { v };
    let x = v
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| error("Compacted relation clock error overflow"))?;
    let x = if q(x) < v { x.next_up() } else { x };
    if !x.is_finite() {
        return Err(error("Compacted relation clock error overflow"));
    }
    Ok(x)
}
fn image(v: Q, source: [f64; 2], output: [f64; 2]) -> Q {
    q(output[0])
        + (v - q(source[0])) * (q(output[1]) - q(output[0])) / (q(source[1]) - q(source[0]))
}
fn merge(
    records: &[Relation],
    surface: &nurbs_core::surface::Surface,
    pc: &Curve,
) -> Result<Option<CoedgeTrim>> {
    if records.is_empty() {
        return Ok(None);
    }
    let canonical = |c: [Q; 2], p: [Q; 2]| {
        if c[0] < c[1] {
            (c, p)
        } else {
            ([c[1].clone(), c[0].clone()], [p[1].clone(), p[0].clone()])
        }
    };
    // Check exact images AND stored images: rounding cannot fill a gap or
    // turn different requested clocks into a common line.
    let mut saved = None;
    for exact in [true, false] {
        let mut pieces = records
            .iter()
            .map(|r| {
                if exact {
                    canonical(r.curve_parameters.clone(), r.pcurve_parameters.clone())
                } else {
                    canonical(
                        r.transferred.curve_parameter.map(q),
                        r.transferred.pcurve_parameter.map(q),
                    )
                }
            })
            .collect::<Vec<_>>();
        pieces.sort_by(|a, b| a.0[0].cmp(&b.0[0]));
        let (c, p) = &pieces[0];
        let dc = &c[1] - &c[0];
        let dp = &p[1] - &p[0];
        let mut end = c[1].clone();
        for (nc, np) in &pieces[1..] {
            if nc[0] > end {
                return Err(error(
                    "Disconnected authored clock ranges require a compound coedge relation",
                ));
            }
            for j in 0..2 {
                if (&np[j] - &p[0]) * &dc != (&nc[j] - &c[0]) * &dp {
                    return Err(error(
                        "Nonaffine authored clocks require a compound coedge relation",
                    ));
                }
            }
            end = end.max(nc[1].clone());
        }
        if !exact {
            let last = &p[0] + (&end - &c[0]) * &dp / &dc;
            // Stored endpoints come from actual authored fragments; do not
            // round a newly extrapolated aggregate endpoint.
            let c0 = stored(&c[0], pc.domain())?;
            let c1 = stored(&end, pc.domain())?;
            let p0 = stored(&p[0], pc.domain())?;
            let p1 = stored(&last, pc.domain())?;
            if q(c0) != c[0] || q(c1) != end || q(p0) != p[0] || q(p1) != last {
                return Err(error("Aggregate authored clock is not exactly storable"));
            }
            let trim = CoedgeTrim {
                curve_parameter: [c0, c1],
                pcurve_parameter: [p0, p1],
                periodic_lift: crate::boundary_agreement::periodic_lifts(surface, pc, [p0, p1])?,
            };
            trim.validate()?;
            saved = Some(trim);
        }
    }
    Ok(saved)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn transfer(
    source: &Model,
    intermediate: &Model,
    face: usize,
    wire: usize,
    cycle: &[CycleUse],
    world: &Concatenation,
    uv: &Concatenation,
    reversed: bool,
    tolerance: f64,
    uv_tolerance: f64,
    limits: Limits,
    work: &mut Work,
) -> Result<(Option<CoedgeTrim>, Vec<Relation>)> {
    let surface = &intermediate.faces[face].surface;
    let mut records = Vec::new();
    for (slot, c) in intermediate.loops[wire].coedges.iter().enumerate() {
        let Some(original) = c.trim() else {
            continue;
        };
        let index = cycle.iter().position(|u| u.edge == c.edge).unwrap();
        let ws = &world.segments()[index];
        let us = &uv.segments()[slot];
        let edge = &intermediate.edges[c.edge].curve;
        let d = edge.domain();
        let wd = world.curve().domain();
        let curve_parameters = original.curve_parameter.map(|g| {
            let physical = if c.reversed {
                q(d[0]) + q(d[1]) - q(g)
            } else {
                q(g)
            };
            let t = image(physical, ws.source_parameters, ws.output_parameters);
            if reversed { q(wd[0]) + q(wd[1]) - t } else { t }
        });
        let pcurve_parameters = original
            .pcurve_parameter
            .map(|p| image(q(p), us.source_parameters, us.output_parameters));
        let cp = [
            stored(&curve_parameters[0], wd)?,
            stored(&curve_parameters[1], wd)?,
        ];
        let pp = [
            stored(&pcurve_parameters[0], uv.curve().domain())?,
            stored(&pcurve_parameters[1], uv.curve().domain())?,
        ];
        let transferred = CoedgeTrim {
            curve_parameter: cp,
            pcurve_parameter: pp,
            periodic_lift: crate::boundary_agreement::periodic_lifts(surface, uv.curve(), pp)?,
        };
        transferred.validate()?;
        let drift =
            upper(&curve_parameters[0] - q(cp[0]))?.max(upper(&curve_parameters[1] - q(cp[1]))?);
        let (clock, cells) = lift::parameter_clock_error_bound(
            world.curve(),
            drift,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += cells;
        let clock =
            clock.ok_or_else(|| error("Compacted authored world-clock drift remains unproven"))?;
        let original_curve_distance_upper =
            nurbs_core::interval_eval::Interval::point(ws.agreement.error_upper)
                .add(nurbs_core::interval_eval::Interval::point(clock))?
                .hi;
        if original_curve_distance_upper > tolerance {
            return Err(error(
                "Compacted authored world relation exceeds original tolerance",
            ));
        }
        let pc = partition::mapped::verify(
            &c.pcurve,
            uv.curve(),
            original.pcurve_parameter,
            pp,
            uv_tolerance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += pc.cells;
        if !pc.accepted {
            return Err(error(
                "Compacted original authored pcurve relation remains unproven",
            ));
        }
        let nodes = std::array::from_fn::<_, 2, _>(|i| [original.curve_parameter[i], pp[i]]);
        let original_lift = lift::verify_parameter_correspondence(
            edge,
            uv.curve(),
            surface,
            &nodes,
            c.reversed,
            tolerance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += original_lift.cells;
        if original_lift.status != lift::Status::WithinTolerance {
            return Err(error(
                "Original authored relation versus compacted UV lift remains unproven",
            ));
        }
        let nodes = std::array::from_fn::<_, 2, _>(|i| [cp[i], pp[i]]);
        let actual = lift::verify_parameter_correspondence(
            world.curve(),
            uv.curve(),
            surface,
            &nodes,
            reversed,
            tolerance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += actual.cells;
        if actual.status != lift::Status::WithinTolerance {
            return Err(error("Saved compacted authored relation remains unproven"));
        }
        records.push(Relation {
            face,
            wire,
            coedge: source.loops[wire]
                .coedges
                .iter()
                .position(|old| old.edge == c.edge)
                .unwrap(),
            source_edge: c.edge,
            original,
            curve_parameters,
            pcurve_parameters,
            transferred,
            original_curve_distance_upper,
            original_pcurve_distance_upper: pc.error_upper,
            original_lift_distance_upper: original_lift
                .distance_upper
                .ok_or_else(|| error("Missing original compacted lift bound"))?,
        });
    }
    Ok((merge(&records, surface, uv.curve())?, records))
}
