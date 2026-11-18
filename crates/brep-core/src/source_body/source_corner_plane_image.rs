//! Exact plane image confined to one original world corner (or collapsed edge).
use cad_predicates::Sign;
use nurbs_core::{Error, Result, surface::Surface};
pub struct Certificate {
    surface: Surface,
    plane: [[f64; 3]; 3],
    point: [f64; 3],
    side: Sign,
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    pub fn side(&self) -> Sign {
        self.side
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
}
pub fn certify(
    s: &Surface,
    plane: [[f64; 3]; 3],
    point: [f64; 3],
    max_work: u64,
) -> Result<Report> {
    s.validate()?;
    if !(1..=100_000_000).contains(&max_work)
        || plane
            .iter()
            .flatten()
            .chain(point.iter())
            .any(|x| !x.is_finite())
    {
        return Err(Error::new(
            "BREP_SOURCE_CORNER_IMAGE",
            "Choose finite original inputs and bounded work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
    };
    let clamped = |d: usize, k: &[f64], n: usize| {
        (1..=8).contains(&d)
            && n == d + 1
            && k.len() == 2 * (d + 1)
            && k[0] < k[d + 1]
            && k[..=d].iter().all(|t| *t == k[0])
            && k[d + 1..].iter().all(|t| *t == k[d + 1])
    };
    if s.periodic_u
        || s.periodic_v
        || !clamped(s.degree_u, &s.knots_u, s.control_points.len())
        || !clamped(s.degree_v, &s.knots_v, s.control_points[0].len())
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || s.weights.iter().flatten().any(|w| *w <= 0.)
    {
        return Ok(out);
    }
    if !crate::source_allowed_contact::independent(
        plane[0],
        plane[1],
        plane[2],
        &mut out.exact_work,
        max_work,
    )? {
        return Ok(out);
    }
    let mut signs = Vec::new();
    let mut side = None;
    for row in &s.control_points {
        let mut values = Vec::new();
        for p in row {
            let Some(sign) = crate::source_allowed_contact::orient(
                &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                None,
                &mut out.exact_work,
                max_work,
            )?
            else {
                return Ok(out);
            };
            if sign != Sign::Zero {
                if side.is_some_and(|s| s != sign) {
                    return Ok(out);
                }
                side = Some(sign);
            }
            values.push(sign);
        }
        signs.push(values);
    }
    let Some(side) = side else {
        return Ok(out);
    };
    let (p, q) = (s.degree_u, s.degree_v);
    let edges = [
        (0..=p).map(|i| (i, 0)).collect::<Vec<_>>(),
        (0..=q).map(|j| (p, j)).collect(),
        (0..=p).map(|i| (i, q)).collect(),
        (0..=q).map(|j| (0, j)).collect(),
    ];
    for edge in edges {
        if edge.iter().all(|&(i, j)| signs[i][j] == Sign::Zero)
            && edge
                .iter()
                .any(|&(i, j)| s.control_points[i][j].as_slice() != point.as_slice())
        {
            return Ok(out);
        }
    }
    let mut found = false;
    for (i, j) in [(0, 0), (p, 0), (p, q), (0, q)] {
        if signs[i][j] == Sign::Zero {
            if s.control_points[i][j].as_slice() != point.as_slice() {
                return Ok(out);
            }
            found = true;
        }
    }
    if found {
        out.certificate = Some(Certificate {
            surface: s.clone(),
            plane,
            point,
            side,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Surface {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| (0..3).map(|j| vec![i as f64, j as f64, 1.]).collect())
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        s.control_points[0][0][2] = 0.;
        s.control_points[1][1][2] = 0.;
        s
    }
    fn admitted(s: &Surface, budget: u64) -> bool {
        certify(
            s,
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            [0., 0., 0.],
            budget,
        )
        .unwrap()
        .certificate
        .is_some()
    }
    #[test]
    fn interior_zero_controls_do_not_create_plane_contacts() {
        let s = fixture();
        assert!(admitted(&s, 1000000));
        assert!(!admitted(&s, 1));
        let mut crossed = s.clone();
        crossed.control_points[1][1][2] = -1.;
        assert!(!admitted(&crossed, 1000000));
        let mut second = s.clone();
        second.control_points[2][2][2] = 0.;
        assert!(!admitted(&second, 1000000));
    }
    #[test]
    fn a_zero_boundary_must_map_to_the_owned_point() {
        let mut s = fixture();
        for j in 0..3 {
            s.control_points[0][j][2] = 0.;
        }
        assert!(!admitted(&s, 1000000));
        for j in 0..3 {
            s.control_points[0][j] = vec![0., 0., 0.];
        }
        assert!(admitted(&s, 1000000));
        s.control_points[0][1][0] = f64::from_bits(1);
        assert!(!admitted(&s, 1000000));
    }
}
