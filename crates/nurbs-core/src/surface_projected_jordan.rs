//! Global interior injectivity from exact orientation and a simple boundary.
//! Degree is ±1 inside a Jordan boundary and zero outside. Every interior
//! preimage has the same nonzero local degree, so there is exactly one inside
//! and none outside. Openness excludes interior images on the boundary.
use crate::{
    Result, check, curve::Curve, curve_quadratic_separator as separator, surface::Surface,
    surface_projection_jacobian as jacobian, trim_simplicity,
};
pub struct Certificate {
    orientation: jacobian::Certificate,
    boundary: Vec<Curve>,
    boundary_indices: Vec<usize>,
    collapsed: Vec<usize>,
    separators: Vec<separator::Certificate>,
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        self.orientation.surface()
    }
    pub fn axes(&self) -> [usize; 2] {
        self.orientation.axes()
    }
    pub fn boundary(&self) -> &[Curve] {
        &self.boundary
    }
    /// Natural rectangle edges: v-low, u-high, v-high, u-low.
    pub fn boundary_indices(&self) -> &[usize] {
        &self.boundary_indices
    }
    pub fn collapsed_boundaries(&self) -> &[usize] {
        &self.collapsed
    }
    pub fn separators(&self) -> &[separator::Certificate] {
        &self.separators
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub boundary_cells: usize,
    pub reason: &'static str,
}
pub fn certify(
    s: &Surface,
    axes: [usize; 2],
    tolerance: f64,
    max_work: u64,
    max_cells: usize,
) -> Result<Report> {
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Choose bounded projected boundary work",
    )?;
    let orientation = jacobian::certify(s, axes, max_work)?;
    let mut out = Report {
        certificate: None,
        exact_work: orientation.exact_work,
        boundary_cells: 0,
        reason: "projected-orientation-unproven",
    };
    let Some(orientation) = orientation.certificate else {
        return Ok(out);
    };
    let (p, q) = (s.degree_u, s.degree_v);
    let natural = [
        ((0..=p).map(|i| (i, 0)).collect::<Vec<_>>(), p),
        ((0..=q).map(|j| (p, j)).collect(), q),
        ((0..=p).rev().map(|i| (i, q)).collect(), p),
        ((0..=q).rev().map(|j| (0, j)).collect(), q),
    ];
    let mut boundary = Vec::new();
    let mut boundary_indices = Vec::new();
    let mut collapsed = Vec::new();
    for (index, (indices, degree)) in natural.into_iter().enumerate() {
        let c = Curve {
            degree,
            knots: std::iter::repeat_n(0., degree + 1)
                .chain(std::iter::repeat_n(1., degree + 1))
                .collect(),
            control_points: indices
                .iter()
                .map(|&(i, j)| {
                    vec![
                        s.control_points[i][j][axes[0]],
                        s.control_points[i][j][axes[1]],
                    ]
                })
                .collect(),
            weights: indices.iter().map(|&(i, j)| s.weights[i][j]).collect(),
            periodic: false,
        };
        if c.control_points.iter().all(|p| p == &c.control_points[0]) {
            collapsed.push(index);
        } else {
            boundary_indices.push(index);
            boundary.push(c);
        }
    }
    out.reason = "projected-boundary-simplicity-unproven";
    if boundary.len() < 3 {
        return Ok(out);
    }
    let audit = trim_simplicity::inspect(&boundary, tolerance, 6, max_cells)?;
    out.boundary_cells = audit.cells;
    if audit.exact_joins != Some(true)
        || audit.injective.iter().any(|b| !*b)
        || audit.pairs.len() != audit.total_pairs
    {
        return Ok(out);
    }
    let mut separators = Vec::new();
    for pair in audit.pairs {
        if pair.proven {
            continue;
        }
        let [a, b] = pair.curves;
        let ends = if b == a + 1 {
            [1, 0]
        } else if a == 0 && b == boundary.len() - 1 {
            [0, 1]
        } else {
            return Ok(out);
        };
        let curves = [&boundary[a], &boundary[b]];
        let frame_index = if ends[0] == 0 {
            1
        } else {
            curves[0].degree - 1
        };
        let join = &curves[0].control_points[ends[0] * curves[0].degree];
        let t = &curves[0].control_points[frame_index];
        let direction = [t[0] - join[0], t[1] - join[1]];
        let ratio = |c: &Curve, u: f64| -> Result<f64> {
            let point = c.evaluate(u)?.point;
            let delta = [point[0] - join[0], point[1] - join[1]];
            let dot = direction[0] * delta[0] + direction[1] * delta[1];
            Ok((direction[0] * delta[1] - direction[1] * delta[0]) / (dot * dot))
        };
        let mut proof = None;
        for fraction in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let parameters = ends.map(|end| if end == 0 { fraction } else { 1. - fraction });
            let beta = (ratio(curves[0], parameters[0])? + ratio(curves[1], parameters[1])?) * 0.5;
            if !beta.is_finite() {
                continue;
            }
            let remaining = max_work
                .saturating_sub(out.exact_work)
                .min(cad_predicates::MAX_WORK);
            if remaining == 0 {
                return Ok(out);
            }
            let r = separator::certify(curves, ends, frame_index, beta, remaining)?;
            out.exact_work += r.exact_work;
            if let Some(c) = r.certificate {
                proof = Some(c);
                break;
            }
        }
        let Some(proof) = proof else {
            return Ok(out);
        };
        separators.push(proof);
    }
    out.certificate = Some(Certificate {
        orientation,
        boundary,
        boundary_indices,
        collapsed,
        separators,
    });
    out.reason = "original-projection-jordan-interior-injective";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn surface(triangle: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![0., 0., 0.],
                    vec![0., if triangle { 0. } else { 1. }, 0.],
                ],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn square_and_collapsed_triangle_have_owned_original_projections() {
        for triangle in [false, true] {
            let s = surface(triangle);
            let r = certify(&s, [0, 1], 1e-8, 1_000_000, 1000).unwrap();
            let c = r.certificate.unwrap();
            assert_eq!(c.surface(), &s);
            assert_eq!(c.axes(), [0, 1]);
            assert_eq!(c.boundary().len(), if triangle { 3 } else { 4 });
            assert_eq!(
                c.collapsed_boundaries(),
                if triangle { &[3][..] } else { &[][..] }
            );
        }
    }
    #[test]
    fn fold_and_work_exhaustion_do_not_grant_global_projection() {
        let s = surface(false);
        assert!(
            certify(&s, [0, 1], 1e-8, 1, 1000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut damaged = s.clone();
        damaged.control_points[1][1][1] = -1.;
        assert!(
            certify(&damaged, [0, 1], 1e-8, 1_000_000, 1000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
}
