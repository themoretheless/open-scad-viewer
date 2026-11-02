//! Filled-face XY bounds from an original closed outer trim, not the unused
//! portion of its carrier. Admission also requires the caller's source atlas.
use super::*;
pub(super) fn avoids(model: &Model, face: usize, column: &Box3, work: &mut Work) -> Result<bool> {
    let f = &model.faces[face];
    let ring = &model.loops[f.outer].coedges;
    if ring.is_empty() || ring.len() > 256 {
        return Ok(false);
    }
    for (i, c) in ring.iter().enumerate() {
        let p = &c.pcurve;
        let next = &ring[(i + 1) % ring.len()].pcurve;
        if p.periodic
            || p.weights.iter().any(|w| *w <= 0.)
            || !p.knots[..=p.degree].iter().all(|&t| t == p.domain()[0])
            || !p.knots[p.control_points.len()..]
                .iter()
                .all(|&t| t == p.domain()[1])
            || p.control_points.last() != next.control_points.first()
        {
            return Ok(false);
        }
    }
    let (_, xy) = match input(&f.surface, work) {
        Ok(r) => r,
        Err(e) if e.code == "MINKOWSKI_3D_REFUSED" => return Ok(false),
        Err(e) => return Err(e),
    };
    let s = &f.surface;
    let ranges = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let q = |x| Q::from_float(x).unwrap();
    let mut points = Vec::new();
    for c in ring {
        for p in &c.pcurve.control_points {
            work.charge(1)?;
            if p.len() != 2 {
                return Ok(false);
            }
            let uv: [Q; 2] = std::array::from_fn(|i| {
                (q(p[i]) - q(ranges[i][0])) / (q(ranges[i][1]) - q(ranges[i][0]))
            });
            let point: P = std::array::from_fn(|i| {
                if i == 2 {
                    Q::zero()
                } else {
                    &xy[i][0] + &xy[i][1] * &uv[0] + &xy[i][2] * &uv[1]
                }
            });
            guard(&point)?;
            points.push(point);
        }
    }
    // Positive rational curves lie in their control hull. A bounded filled
    // outer UV region cannot exceed extrema of an affine functional on its
    // boundary. Holes only remove material. Therefore these are whole-face
    // bounds, not a boundary-only or sampled containment claim.
    Ok(separated(&bounds(&points)[..2], &column[..2]))
}
