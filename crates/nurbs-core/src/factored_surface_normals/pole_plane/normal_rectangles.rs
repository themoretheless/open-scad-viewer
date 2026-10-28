//! Whole original rectangles, including their exact collapsed-boundary limits.
use super::*;
pub struct DesingularizedPairReport {
    pub aligned: Option<bool>,
    pub sine_squared_bounds: Option<[f64; 2]>,
    pub coefficients: usize,
    pub reason: &'static str,
}
impl PlaneLimit<'_> {
    fn rectangle_normals(&self, domain: [[f64; 2]; 2]) -> Result<[[f64; 2]; 3]> {
        let s = self.surface;
        let domains = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        check(
            (0..2).all(|a| {
                domain[a][0].is_finite()
                    && domain[a][1].is_finite()
                    && domains[a][0] <= domain[a][0]
                    && domain[a][0] <= domain[a][1]
                    && domain[a][1] <= domains[a][1]
            }),
            "Use a closed rectangle of the original pole chart",
        )?;
        let (axis, end) = axes(self.boundary);
        let mut ranges = [[I::point(0.); 2]; 2];
        for a in 0..2 {
            let length = I::point(domains[a][1]).sub(I::point(domains[a][0]))?;
            for e in 0..2 {
                ranges[a][e] = I::point(domain[a][e])
                    .sub(I::point(domains[a][0]))?
                    .div(length)?;
            }
        }
        if end {
            ranges[axis] = [
                I::point(1.).sub(ranges[axis][1])?,
                I::point(1.).sub(ranges[axis][0])?,
            ];
        }
        // The private net is inward radial first, angular second. Its world
        // normal orientation was already corrected by prepare_plane_limit.
        let ranges = [ranges[axis], ranges[1 - axis]];
        let point = [
            domain[axis][0] == domain[axis][1],
            domain[1 - axis][0] == domain[1 - axis][1],
        ];
        let mut net = self.net.clone();
        for v in 0..net[0].len() {
            let column = net.iter().map(|row| row[v]).collect::<Vec<_>>();
            let bounded = restrict(&column, ranges[0], point[0])?;
            for (row, p) in net.iter_mut().zip(bounded) {
                row[v] = p;
            }
        }
        for row in &mut net {
            *row = restrict(row, ranges[1], point[1])?;
        }
        Ok(std::array::from_fn(|k| {
            [
                net.iter()
                    .flatten()
                    .map(|p| p[k].lo)
                    .fold(f64::INFINITY, f64::min),
                net.iter()
                    .flatten()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        }))
    }
    /// Compare the complete private factored normal nets on original closed
    /// rectangles. The removed inward parameter power is positive off the
    /// collapsed boundary. At the boundary these are limit normals, never a
    /// claim of regular parameterization or quotient topology. Each source-net
    /// vector coefficient is charged once per complete query, before work.
    pub fn compare_normal_rectangles(
        &self,
        other: &PlaneLimit<'_>,
        domains: [[[f64; 2]; 2]; 2],
        max_sine_squared: f64,
        max_coefficients: usize,
    ) -> Result<DesingularizedPairReport> {
        check(
            max_sine_squared.is_finite()
                && (0. ..1.).contains(&max_sine_squared)
                && (1..=100000000).contains(&max_coefficients),
            "Choose bounded factored-normal comparison work and angle",
        )?;
        let cost = self.net.len() * self.net[0].len() + other.net.len() * other.net[0].len();
        if cost > max_coefficients {
            return Ok(DesingularizedPairReport {
                aligned: None,
                sine_squared_bounds: None,
                coefficients: 0,
                reason: "desingularized-normal-coefficient-limit",
            });
        }
        let result = (|| {
            let components = [
                self.rectangle_normals(domains[0])?,
                other.rectangle_normals(domains[1])?,
            ];
            crate::normal_alignment::inspect_pair_bounds(components, max_sine_squared)
        })();
        let r = match result {
            Ok(r) => r,
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                return Ok(DesingularizedPairReport {
                    aligned: None,
                    sine_squared_bounds: None,
                    coefficients: cost,
                    reason: "desingularized-normal-interval-unresolved",
                });
            }
            Err(e) => return Err(e),
        };
        Ok(DesingularizedPairReport {
            aligned: r.aligned,
            sine_squared_bounds: r.sine_squared_interval,
            coefficients: cost,
            reason: r.reason,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factored_closed_rectangles_cover_all_axes_domains_and_work_limits() {
        for b in [
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ] {
            let mut s = super::super::tests::oriented(super::super::tests::source(1, false), b);
            s.knots_u.iter_mut().for_each(|x| *x = 2. + 3. * *x);
            s.knots_v.iter_mut().for_each(|x| *x = -4. + 12. * *x);
            // Keep only the exact tangent plane: flat polar fan, nonzero
            // desingularized normal on the full radial interval.
            for p in s.control_points.iter_mut().flatten() {
                p[2] = 0.;
            }
            let original = s.clone();
            let p = prepare_plane_limit(&s, b, 1000000)
                .unwrap()
                .certificate
                .unwrap();
            let domains = [[[2., 5.], [-4., 8.]]; 2];
            let r = p
                .compare_normal_rectangles(&p, domains, 1e-12, 10000)
                .unwrap();
            assert_eq!(r.aligned, Some(true));
            assert!(r.sine_squared_bounds.unwrap()[1] <= 1e-12);
            let short = p
                .compare_normal_rectangles(&p, domains, 1e-12, r.coefficients - 1)
                .unwrap();
            assert_eq!(short.aligned, None);
            assert_eq!(short.coefficients, 0);
            let (axis, end) = axes(b);
            let mut point = domains;
            let d = point[0][axis][usize::from(end)];
            for rect in &mut point {
                rect[axis] = [d, d];
            }
            assert_eq!(
                p.compare_normal_rectangles(&p, point, 1e-12, 10000)
                    .unwrap()
                    .aligned,
                Some(true)
            );
            let mut invalid = domains;
            invalid[0][0] = [1., 5.];
            assert!(
                p.compare_normal_rectangles(&p, invalid, 1e-12, 10000)
                    .is_err()
            );
            assert_eq!(s, original);
        }
    }
    #[test]
    fn matching_pole_planes_do_not_hide_oblique_meridian_interiors() {
        let curved = super::super::tests::source(1, false);
        let mut flat = curved.clone();
        for p in flat.control_points.iter_mut().flatten() {
            p[2] = 0.;
        }
        let a = prepare_plane_limit(&curved, Boundary::UMin, 1000000)
            .unwrap()
            .certificate
            .unwrap();
        let b = prepare_plane_limit(&flat, Boundary::UMin, 1000000)
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(
            compare_plane_limits(&a, &b, 1000000).unwrap().same_plane,
            Some(true)
        );
        let at_pole = [[[0., 0.], [0., 1.]]; 2];
        assert_eq!(
            a.compare_normal_rectangles(&b, at_pole, 1e-12, 10000)
                .unwrap()
                .aligned,
            Some(true)
        );
        let at_far = [[[1., 1.], [0., 1.]]; 2];
        assert_eq!(
            a.compare_normal_rectangles(&b, at_far, 1e-12, 10000)
                .unwrap()
                .aligned,
            Some(false)
        );
    }
}
