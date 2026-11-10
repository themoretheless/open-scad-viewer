use super::*;
use nurbs_core::interval_eval::Interval as I;
pub(super) fn endpoints(m: &Model, edges: &[usize], limits: Limits, work: &mut Work) -> Result<()> {
    for &edge in edges {
        for end in 0..2 {
            work.remaining(limits)?;
            work.cells += 1;
            let e = &m.edges[edge];
            let domain = e.curve.domain();
            let count = e.curve.control_points.len();
            if !e.curve.knots[..=e.curve.degree]
                .iter()
                .all(|v| *v == domain[0])
                || !e.curve.knots[count..].iter().all(|v| *v == domain[1])
            {
                return Err(super::error("Stored partition endpoint is not clamped"));
            }

            let v = e.vertices[end];
            let tol = m
                .tolerance_mm
                .min(
                    m.1.local_tolerances_mm
                        .get(&m.1.edges[edge])
                        .copied()
                        .unwrap_or(m.tolerance_mm),
                )
                .min(
                    m.1.local_tolerances_mm
                        .get(&m.1.vertices[v])
                        .copied()
                        .unwrap_or(m.tolerance_mm),
                );
            // Partition extraction clamps original intervals. These exact stored
            // endpoint controls define the candidate endpoints; no vertex is snapped.
            let p = if end == 0 {
                &e.curve.control_points[0]
            } else {
                e.curve.control_points.last().unwrap()
            };
            let mut error = I::point(0.);
            for k in 0..3 {
                if p[k] != m.vertices[v].point[k] {
                    let d = I::point(p[k]).sub(I::point(m.vertices[v].point[k]))?;
                    error = error.add(I::point(d.lo.abs().max(d.hi.abs())))?;
                }
            }
            if error.hi > tol {
                return Err(super::error(
                    "Partition endpoint exceeds its edge/vertex tolerance",
                ));
            }
        }
    }
    Ok(())
}
