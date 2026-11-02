//! Original shared-chart or interval-qualified planar carrier proposals.
//! Failure to prove a common carrier is not evidence that charts are disjoint.
use super::*;
use nurbs_core::{curve::Curve, interval_eval::Interval as I, surface::Surface};
pub(super) struct Carrier {
    pub surface: Surface,
    pub curves: Option<BTreeMap<usize, Curve>>,
    pub source_loop: Option<usize>,
}
pub(super) fn propose(
    source: &Model,
    selected: &BTreeSet<usize>,
    incidence: &BTreeMap<usize, Vec<(usize, usize)>>,
    rim: &BTreeSet<usize>,
    limits: Limits,
    work: &mut ProofWork,
) -> Result<Option<Carrier>> {
    work.charge_cells(1, limits)?;
    let neighbors = rim
        .iter()
        .map(|e| {
            incidence[e]
                .iter()
                .find(|(f, _)| !selected.contains(f))
                .copied()
                .ok_or_else(|| error("Original rim support is missing"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let supports = neighbors
        .iter()
        .map(|(f, _)| *f)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let first = *supports
        .iter()
        .min_by_key(|f| source.1.faces[**f])
        .ok_or_else(|| error("Missing support carrier"))?;
    if supports
        .iter()
        .all(|f| source.faces[*f].surface == source.faces[first].surface)
        && neighbors.iter().all(|(_, w)| {
            source.loops[*w]
                .coedges
                .iter()
                .all(|c| rim.contains(&c.edge))
        })
    {
        return Ok(Some(Carrier {
            surface: source.faces[first].surface.clone(),
            curves: None,
            source_loop: if neighbors.len() == 1 {
                Some(neighbors.first().unwrap().1)
            } else {
                None
            },
        }));
    }
    Ok(
        planar(source, &supports, rim, limits, work)?.map(|(surface, curves)| Carrier {
            surface,
            curves: Some(curves),
            source_loop: None,
        }),
    )
}
fn planar(
    source: &Model,
    supports: &[usize],
    rim: &BTreeSet<usize>,
    limits: Limits,
    work: &mut ProofWork,
) -> Result<Option<(Surface, BTreeMap<usize, Curve>)>> {
    let first = *supports.iter().min_by_key(|f| source.1.faces[**f]).unwrap();
    let controls = supports
        .iter()
        .map(|f| {
            source.faces[*f]
                .surface
                .control_points
                .iter()
                .flatten()
                .count()
        })
        .sum::<usize>();
    if controls > 8192 {
        return Err(error("Original support plane control allowance exhausted"));
    }
    work.charge_cells(controls, limits)?;
    let points = source.faces[first]
        .surface
        .control_points
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    let origin = [points[0][0], points[0][1], points[0][2]];
    let vector = |p: &Vec<f64>| super::super::sub([p[0], p[1], p[2]], origin);
    let x = points
        .iter()
        .map(|p| vector(p))
        .max_by(|a, b| super::super::dot(*a, *a).total_cmp(&super::super::dot(*b, *b)))
        .ok_or_else(|| error("Empty closure plane"))?;
    let n = points
        .iter()
        .map(|p| super::super::cross(x, vector(p)))
        .max_by(|a, b| super::super::dot(*a, *a).total_cmp(&super::super::dot(*b, *b)))
        .ok_or_else(|| error("Closure plane has no rank"))?;
    let Ok(n) = super::super::unit(n) else {
        return Ok(None);
    };
    let Ok(x) = super::super::unit(x) else {
        return Ok(None);
    };
    let Ok(y) = super::super::unit(super::super::cross(n, x)) else {
        return Ok(None);
    };
    let lower = n.iter().map(|v| v.abs()).fold(0f64, f64::max);
    let tolerance = source.tolerance_mm / 16.;
    for &f in supports {
        for p in source.faces[f].surface.control_points.iter().flatten() {
            let mut value = I::point(0.);
            for k in 0..3 {
                value = value.add(
                    I::point(p[k])
                        .sub(I::point(origin[k]))?
                        .mul(I::point(n[k]))?,
                )?;
            }
            // Round the admissible threshold downward: an upward-rounded
            // product must not enlarge the original geometric allowance.
            if value.lo.abs().max(value.hi.abs()) > (tolerance * lower).next_down() {
                return Ok(None);
            }
        }
    }
    let projected = rim
        .iter()
        .map(|e| source.edges[*e].curve.control_points.len())
        .sum::<usize>();
    work.charge_cells(projected.saturating_mul(2), limits)?;
    let project = |p: &Vec<f64>| {
        let d = vector(p);
        [super::super::dot(d, x), super::super::dot(d, y)]
    };
    let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    for e in rim {
        for p in &source.edges[*e].curve.control_points {
            let q = project(p);
            for k in 0..2 {
                bounds[0][k] = bounds[0][k].min(q[k]);
                bounds[1][k] = bounds[1][k].max(q[k]);
            }
        }
    }
    let width = [bounds[1][0] - bounds[0][0], bounds[1][1] - bounds[0][1]];
    if width.iter().any(|v| !v.is_finite() || *v <= 0.) {
        return Err(error("Closure plane has no finite rim extent"));
    }
    let net = (0..2)
        .map(|i| {
            (0..2)
                .map(|j| {
                    let u = bounds[i][0];
                    let v = bounds[j][1];
                    (0..3).map(|k| origin[k] + u * x[k] + v * y[k]).collect()
                })
                .collect()
        })
        .collect();
    let surface = nurbs_core::patches::bezier(net, None)?;
    let mut curves = BTreeMap::new();
    for e in rim {
        let mut c = source.edges[*e].curve.clone();
        for p in &mut c.control_points {
            let q = project(p);
            *p = vec![
                (q[0] - bounds[0][0]) / width[0],
                (q[1] - bounds[0][1]) / width[1],
            ];
        }
        curves.insert(*e, c);
    }
    Ok(Some((surface, curves)))
}
