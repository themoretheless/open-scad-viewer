//! Complete UV winding-domain hull, evaluated on the original curved support.
//! A boundary-control hull is used in PARAMETER space only. No affine image
//! assumption, fitted support, or point sample may grant spatial separation.
use super::*;
pub(super) fn inspect(
    a: &Model,
    b: &Model,
    max_cells: usize,
    tolerance_uv: f64,
) -> Result<Option<(f64, usize)>> {
    let mut queries = Vec::new();
    let mut cells = 0usize;
    for m in [a, b] {
        if m.faces.len() != 1 {
            return Ok(None);
        }
        let f = &m.faces[0];
        let s = &f.surface;
        if s.periodic_u || s.periodic_v {
            return Ok(None);
        }
        // Validated winding loops include their connector chords. Positive
        // rational boundaries and those chords lie in this common UV hull;
        // every bounded filled winding component lies there as well. Holes
        // are conservatively filled by this enclosure, never discarded.
        FaceDomain::new(m, 0, tolerance_uv)?;
        let mut uv = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for co in &m.loops[wire].coedges {
                let c = &co.pcurve;
                if c.periodic
                    || c.control_points[0].len() != 2
                    || c.weights.iter().any(|w| *w <= 0.)
                {
                    return Ok(None);
                }
                cells = cells.saturating_add(c.control_points.len());
                for p in &c.control_points {
                    for k in 0..2 {
                        uv[k][0] = uv[k][0].min(p[k]);
                        uv[k][1] = uv[k][1].max(p[k]);
                    }
                }
            }
        }
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for k in 0..2 {
            // Restrict to the original chart, never extrapolate outside it.
            uv[k][0] = uv[k][0].max(natural[k][0]);
            uv[k][1] = uv[k][1].min(natural[k][1]);
            if uv[k][0] >= uv[k][1] {
                return Ok(None);
            }
        }
        let spans = |knots: &[f64], p: usize, n: usize, range: [f64; 2]| {
            (p..n)
                .filter(|&i| {
                    knots[i] < knots[i + 1] && knots[i] <= range[1] && knots[i + 1] >= range[0]
                })
                .count()
        };
        cells = cells.saturating_add(
            spans(&s.knots_u, s.degree_u, s.control_points.len(), uv[0]).saturating_mul(spans(
                &s.knots_v,
                s.degree_v,
                s.control_points[0].len(),
                uv[1],
            )),
        );
        queries.push((s, uv));
    }
    if cells > max_cells {
        return Ok(None);
    }
    let mut boxes = Vec::new();
    for (s, uv) in queries {
        boxes.push(interval_eval::evaluate_surface_interval(
            s,
            I::new(uv[0][0], uv[0][1])?,
            I::new(uv[1][0], uv[1][1])?,
        )?);
    }
    // A separated coordinate projection is a lower bound on Euclidean
    // distance. Use outward differences rather than a rounded scalar gap.
    let mut lower = 0f64;
    for k in 0..3 {
        lower = lower.max(I::point(boxes[0][k].lo).sub(I::point(boxes[1][k].hi))?.lo);
        lower = lower.max(I::point(boxes[1][k].lo).sub(I::point(boxes[0][k].hi))?.lo);
    }
    Ok(Some((lower, cells)))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn face(origin: [f64; 3]) -> Model {
        let mut m = crate::cuboid(origin, origin.map(|x|x+1.)).unwrap();
        m.faces.truncate(1);
        m.shells.clear();
        m.bodies.clear();
        m.rebuild_topology_ids();
        m
    }
    #[test]
    fn complete_original_uv_hulls_separate_faces_and_charge_work() {
        let a = face([0.; 3]);
        let b = face([4., 0., 0.]);
        let before = a.clone();
        let (lower, cells) = inspect(&a, &b, 100, 1e-8).unwrap().unwrap();
        assert!(lower > 2.99 && lower <= 3. && cells > 0 && cells <= 100);
        assert!(inspect(&a, &b, cells - 1, 1e-8).unwrap().is_none());
        assert!(inspect(&a, &b, 0, 1e-8).unwrap().is_none());
        assert_eq!(a, before);
    }
    #[test]
    fn touching_or_overlapping_hulls_never_become_a_separation_proof() {
        let a = face([0.; 3]);
        for origin in [[0.; 3], [0.5, 0., 0.], [1., 0., 0.]] {
            let b = face(origin);
            assert_eq!(inspect(&a, &b, 100, 1e-8).unwrap().unwrap().0, 0.);
        }
    }
}
