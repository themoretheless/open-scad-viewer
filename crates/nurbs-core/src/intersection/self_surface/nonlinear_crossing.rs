//! Original-source planar Krawczyk proofs on exactly representable iso-curves.
use super::*;
use crate::{curve::Curve, distance_bounds::Interval as I};
fn exact_f64(q: &BigRational) -> Option<f64> {
    let x = q.to_f64()?;
    (BigRational::from_f64(x).as_ref() == Some(q)).then_some(x)
}
fn original_iso(s: &Surface, axis: usize) -> Option<Curve> {
    let homogeneous = super::linear_crossing::iso(s, axis)?;
    let mut controls = Vec::new();
    let mut weights = Vec::new();
    for h in homogeneous {
        let w = exact_f64(&h[3])?;
        let mut p = Vec::new();
        for c in &h[..3] {
            p.push(exact_f64(&(c / &h[3]))?);
        }
        controls.push(p);
        weights.push(w);
    }
    let (degree, knots) = if axis == 0 {
        (s.degree_u, s.knots_u.clone())
    } else {
        (s.degree_v, s.knots_v.clone())
    };
    Some(Curve {
        degree,
        knots,
        control_points: controls,
        weights,
        periodic: false,
    })
}
pub(super) fn find(s: &Surface, max_cells: usize) -> Result<Option<CertifiedCrossing>> {
    let mut cells = 0;
    for axis in 0..2 {
        let degree = if axis == 0 { s.degree_u } else { s.degree_v };
        if !(2..=16).contains(&degree) {
            continue;
        }
        let Some(c) = original_iso(s, axis) else {
            continue;
        };
        if cells == max_cells {
            return Ok(None);
        }
        let [start, end] = c.domain();
        let width = I::point(end).sub(I::point(start))?;
        let report = crate::intersection::intersect_distinct_spans_certified_report(
            &c,
            1e-7,
            max_cells - cells,
        )?;
        cells += report.cells;
        for crossing in report.crossings {
            let mut boxes = [[[0.5, 0.5]; 2]; 2];
            let mut interior = true;
            for (j, range) in crossing.parameters.into_iter().enumerate() {
                let normalized = I::new(range[0], range[1])?
                    .sub(I::point(start))?
                    .div(width)?;
                if normalized.lo <= 0. || normalized.hi >= 1. {
                    interior = false;
                    break;
                }
                boxes[j][axis] = [normalized.lo, normalized.hi];
            }
            // Disjoint source parameter intervals establish distinct preimages.
            if interior
                && (boxes[0][axis][1] < boxes[1][axis][0] || boxes[1][axis][1] < boxes[0][axis][0])
            {
                return Ok(Some(CertifiedCrossing {
                    parameter_boxes: boxes,
                    position_bounds: crossing.position_bounds,
                    cells,
                }));
            }
        }
    }
    Ok(None)
}
