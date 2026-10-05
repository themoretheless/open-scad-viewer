//! Exact intersection locus of a positive clamped NURBS chart and a plane.
//! This does not establish retained-region ownership or chart injectivity.
use cad_predicates::Sign;
use nurbs_core::{surface::Surface, Error, Result};

pub struct Certificate {
    surface: Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    upper: bool,
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    /// The complete plane preimage is this natural chart boundary.
    pub fn boundary(&self) -> (usize, bool) {
        (self.axis, self.upper)
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// All poles on one natural boundary must lie exactly in the plane; every
/// other pole must lie strictly on the same side. Nonnegative B-spline bases
/// and partition of unity make the plane numerator nonzero elsewhere,
/// including the other three chart boundaries. No fitted normals are used.
pub fn certify(
    surface: &Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    upper: bool,
    max_work: u64,
) -> Result<Report> {
    surface.validate()?;
    if axis > 1
        || !(1..=100_000_000).contains(&max_work)
        || plane.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(Error::new(
            "BREP_SOURCE_FIBER",
            "Invalid plane, axis or exact budget",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-fiber-layout-unproven",
    };
    let counts = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let degrees = [surface.degree_u, surface.degree_v];
    let knots = [&surface.knots_u, &surface.knots_v];
    if surface.periodic_u
        || surface.periodic_v
        || surface
            .weights
            .iter()
            .flatten()
            .any(|w| !w.is_finite() || *w <= 0.)
    {
        return Ok(out);
    }
    for a in 0..2 {
        if degrees[a] == 0 || degrees[a] > 32 {
            return Ok(out);
        }
        let lo = knots[a][degrees[a]];
        let hi = knots[a][counts[a]];
        if lo >= hi
            || knots[a][..=degrees[a]].iter().any(|&x| x != lo)
            || knots[a][degrees[a] + 1..counts[a]]
                .iter()
                .any(|&x| x <= lo || x >= hi)
            || knots[a][counts[a]..].iter().any(|&x| x != hi)
        {
            return Ok(out);
        }
    }
    out.reason = "source-fiber-plane-unproven";
    if !crate::source_allowed_contact::independent(
        plane[0],
        plane[1],
        plane[2],
        &mut out.exact_work,
        max_work,
    )? {
        return Ok(out);
    }
    out.reason = "source-fiber-support-unproven";
    let boundary = if upper { counts[axis] - 1 } else { 0 };
    let mut side = None;
    for (u, row) in surface.control_points.iter().enumerate() {
        for (v, p) in row.iter().enumerate() {
            let Some(sign) = crate::source_allowed_contact::orient(
                &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                None,
                &mut out.exact_work,
                max_work,
            )?
            else {
                return Ok(out);
            };
            if [u, v][axis] == boundary {
                if sign != Sign::Zero {
                    return Ok(out);
                }
            } else {
                if sign == Sign::Zero || side.is_some_and(|s| s != sign) {
                    return Ok(out);
                }
                side = Some(sign);
            }
        }
    }
    out.certificate = Some(Certificate {
        surface: surface.clone(),
        plane,
        axis,
        upper,
    });
    out.reason = "source-fiber-proven";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curved() -> Surface {
        let k = 0.5_f64.sqrt();
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
            ],
            weights: vec![vec![1., k, 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    const PLANE: [[f64; 3]; 3] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    #[test]
    fn curved_rational_boundary_and_both_axis_ends() {
        let s = curved();
        let r = certify(&s, PLANE, 0, false, 1_000_000).unwrap();
        let c = r.certificate.unwrap();
        assert_eq!(c.surface(), &s);
        assert_eq!(c.boundary(), (0, false));
        assert_eq!(c.plane(), PLANE);
        let top = [[0., 0., 1.], [1., 0., 1.], [0., 1., 1.]];
        assert!(certify(&s, top, 0, true, 1_000_000)
            .unwrap()
            .certificate
            .is_some());
        let mut transposed = s.clone();
        transposed.degree_u = s.degree_v;
        transposed.degree_v = s.degree_u;
        transposed.knots_u = s.knots_v.clone();
        transposed.knots_v = s.knots_u.clone();
        transposed.control_points = (0..3)
            .map(|v| (0..2).map(|u| s.control_points[u][v].clone()).collect())
            .collect();
        transposed.weights = (0..3)
            .map(|v| (0..2).map(|u| s.weights[u][v]).collect())
            .collect();
        assert!(certify(&transposed, PLANE, 1, false, 1_000_000)
            .unwrap()
            .certificate
            .is_some());
    }
    #[test]
    fn original_multispan_chart_and_interior_plane_contact_refusal() {
        let mut s = curved();
        // A kinked height profile, with the unchanged rational curved rim.
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let mut middle = s.control_points[1].clone();
        for p in &mut middle {
            p[2] = 0.25;
        }
        s.control_points.insert(1, middle);
        s.weights.insert(1, s.weights[0].clone());
        let c = certify(&s, PLANE, 0, false, 1_000_000)
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(c.surface(), &s);
        // A whole extra interior row at the plane gives another intersection
        // fiber at u=.5; it must never be admitted as a natural-edge-only locus.
        for p in &mut s.control_points[1] {
            p[2] = 0.;
        }
        assert!(certify(&s, PLANE, 0, false, 1_000_000)
            .unwrap()
            .certificate
            .is_none());
    }
    #[test]
    fn extra_zero_opposite_sides_displacement_and_budget_refuse() {
        let s = curved();
        let mut extra = s.clone();
        extra.control_points[1][0][2] = 0.;
        assert!(certify(&extra, PLANE, 0, false, 1_000_000)
            .unwrap()
            .certificate
            .is_none());
        let mut mixed = s.clone();
        mixed.control_points[1][0][2] = -1.;
        assert!(certify(&mixed, PLANE, 0, false, 1_000_000)
            .unwrap()
            .certificate
            .is_none());
        let mut displaced = s.clone();
        displaced.control_points[0][1][2] = 1e-12;
        assert!(certify(&displaced, PLANE, 0, false, 1_000_000)
            .unwrap()
            .certificate
            .is_none());
        assert!(certify(&s, PLANE, 0, false, 1)
            .unwrap()
            .certificate
            .is_none());
        assert!(certify(&s, [[0.; 3]; 3], 0, false, 1_000_000)
            .unwrap()
            .certificate
            .is_none());
    }
}
