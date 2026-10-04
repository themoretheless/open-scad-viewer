//! Material ownership transferred from an exact original profile plane to the
//! ideal initial local plane. No rounded UV fit and no endpoint/cap error claim.
use super::{
    Sweep, cap_projection_certificate as projection, cap_retained_plane_certificate as plane,
    scalar_certificate::Status,
};
use crate::{Result, check};
#[derive(Clone, Debug)]
pub struct Report {
    pub local_domain_certified: bool,
    pub source_plane_axis: Option<usize>,
    pub cells: usize,
    pub pairs: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    pub fn certify_local_profile_domain(
        &self,
        loop_sizes: &[usize],
        tolerance: f64,
        max_pairs: usize,
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<Report> {
        check(
            !loop_sizes.is_empty()
                && loop_sizes.len() <= 16
                && loop_sizes.iter().all(|n| *n > 0 && *n <= 64)
                && loop_sizes.iter().sum::<usize>() == self.profiles.len()
                && self.profiles.len() <= 64
                && max_cells <= 100000
                && max_pairs <= 100000
                && tolerance.is_finite()
                && tolerance > 0.,
            "Invalid ideal local profile domain input/budget",
        )?;
        let mut out = Report {
            local_domain_certified: false,
            source_plane_axis: None,
            cells: 0,
            pairs: 0,
            exact_work: 0,
            reason: Some("source-plane-unproved"),
        };
        let points = self
            .profiles
            .iter()
            .flat_map(|c| c.control_points.iter().cloned())
            .collect::<Vec<_>>();
        if points.len() < 3 {
            return Ok(out);
        }
        // Floating operations propose anchors only. Exact orient predicates below
        // must prove noncollinearity and coplanarity of every original pole.
        let Some(second) = (1..points.len()).find(|i| points[*i] != points[0]) else {
            return Ok(out);
        };
        let delta = |i: usize| std::array::from_fn(|k| points[i][k] - points[0][k]);
        let Some(third) = (1..points.len()).find(|i| {
            math_core::cross(delta(second), delta(*i))
                .iter()
                .any(|x| x.is_finite() && *x != 0.)
        }) else {
            return Ok(out);
        };
        let source = plane::inspect_points(&points, [0, second, third], max_exact_work);
        out.exact_work = source.work;
        let Some(normal) = source.normal else {
            return Ok(out);
        };
        let Some(axis) = (0..3).find(|k| normal[*k][0] > 0. || normal[*k][1] < 0.) else {
            return Ok(out);
        };
        out.source_plane_axis = Some(axis);
        let transport = &self.transport_certificate;
        if transport.status != Status::Certified {
            out.reason = Some("initial-frame-unproved");
            return Ok(out);
        }
        if transport.cells >= max_cells {
            out.reason = Some("work-limit");
            return Ok(out);
        }
        out.cells = transport.cells;
        let projection = projection::certify(
            normal,
            transport.tangents.as_ref().unwrap()[0],
            max_cells - out.cells,
        )?;
        out.cells += projection.cells;
        if !projection.projection_regular {
            out.reason = Some("local-projection-unproved");
            return Ok(out);
        }
        // Dropping a coordinate with nonzero plane normal is an exact affine
        // homeomorphism. Copying poles/weights preserves rational curves exactly.
        let mut at = 0;
        let mut loops = Vec::with_capacity(loop_sizes.len());
        for size in loop_sizes {
            let mut wire = Vec::with_capacity(*size);
            for curve in &self.profiles[at..at + size] {
                let mut c = curve.clone();
                c.control_points = c
                    .control_points
                    .iter()
                    .map(|p| {
                        let mut q = (0..3)
                            .filter(|k| *k != axis)
                            .map(|k| p[k])
                            .collect::<Vec<_>>();
                        q.push(0.);
                        q
                    })
                    .collect();
                wire.push(c);
            }
            at += size;
            loops.push(wire);
        }
        let region = crate::sweep_contour_audit::inspect(
            &loops,
            tolerance,
            max_pairs,
            max_cells - out.cells,
        )?;
        out.cells += region.cells;
        out.pairs = region.pairs;
        if !region.cap_domain_certified {
            out.reason = region.reason;
            return Ok(out);
        }
        out.local_domain_certified = true;
        out.reason = None;
        Ok(out)
    }
}
/// Endpoint material domains are affine images of the certified local region.
/// This certifies ownership geometry, not shell winding/orientation or error.
#[derive(Clone, Debug)]
pub struct EndpointDomains {
    pub ideal_cap_domains_certified: bool,
    pub source: Report,
    pub endpoint_normals: Option<[[[f64; 2]; 3]; 2]>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    pub fn certify_ideal_cap_domains(
        &self,
        loop_sizes: &[usize],
        tolerance: f64,
        max_pairs: usize,
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<EndpointDomains> {
        let source = self.certify_local_profile_domain(
            loop_sizes,
            tolerance,
            max_pairs,
            max_cells,
            max_exact_work,
        )?;
        let mut out = EndpointDomains {
            ideal_cap_domains_certified: false,
            cells: source.cells,
            reason: source.reason,
            source,
            endpoint_normals: None,
        };
        if self.options.closed {
            out.reason = Some("closed-path-has-no-caps");
            return Ok(out);
        }
        if !out.source.local_domain_certified {
            return Ok(out);
        }
        let frames = self.certify_endpoint_cap_normals(max_cells - out.cells)?;
        out.cells += frames.cells;
        let Some(normals) = frames.normals else {
            out.reason = frames.reason;
            return Ok(out);
        };
        // Sweep::new / with_affine_laws validate strictly positive original scale
        // and axis controls with positive rational weights. At both endpoints,
        // q(x,y)=S Ax x n + S Ay y b + center is thus a nonsingular affine map.
        // The certified endpoint frames supply orthonormal n,b. Translation and
        // twist preserve the domain, including holes. Open endpoints have no shear.
        out.endpoint_normals = Some(normals);
        out.ideal_cap_domains_certified = true;
        out.reason = None;
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    fn square(lo: f64, hi: f64, oblique: bool) -> Vec<Curve> {
        let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
        (0..4)
            .map(|i| {
                crate::primitives::line(
                    [p[i][0], p[i][1], if oblique { p[i][0] } else { 0. }],
                    [
                        p[(i + 1) % 4][0],
                        p[(i + 1) % 4][1],
                        if oblique { p[(i + 1) % 4][0] } else { 0. },
                    ],
                )
                .unwrap()
            })
            .collect()
    }
    fn scalar(x: f64) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![x, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        }
    }
    #[test]
    fn original_coordinate_and_oblique_hollow_domains() {
        for oblique in [false, true] {
            let mut profiles = square(0., 4., oblique);
            profiles.extend(square(1., 2., oblique));
            let before = profiles.clone();
            let points = [
                [0., 0., 0.],
                if oblique {
                    [-10., 0., 10.]
                } else {
                    [0., 0., 10.]
                },
            ];
            let scale = scalar(1.);
            let twist = scalar(0.);
            let opts = super::super::Options {
                normal: [0., 1., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 16,
                max_deviation: 0.01,
            };
            let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
            let r = sweep
                .certify_local_profile_domain(&[4, 4], 1e-6, 10000, 10000, 1000000)
                .unwrap();
            assert!(r.local_domain_certified, "{r:?}");
            assert!(
                !sweep
                    .certify_local_profile_domain(&[4, 4], 1e-6, 10000, 10000, r.exact_work - 1)
                    .unwrap()
                    .local_domain_certified
            );
            assert!(
                !sweep
                    .certify_local_profile_domain(&[4, 4], 1e-6, 10000, 0, 1000000)
                    .unwrap()
                    .local_domain_certified
            );

            assert!(
                !sweep
                    .certify_local_profile_domain(&[4, 4], 1e-6, 10000, r.cells - 1, 1000000)
                    .unwrap()
                    .local_domain_certified
            );
            let xy = [[0., 0.], [4., 4.], [0., 4.], [4., 0.]];
            let crossed = (0..4)
                .map(|i| {
                    let point =
                        |j: usize| [xy[j][0], xy[j][1], if oblique { xy[j][0] } else { 0. }];
                    crate::primitives::line(point(i), point((i + 1) % 4)).unwrap()
                })
                .collect::<Vec<_>>();
            let crossed = Sweep::new(&crossed, &points, &scale, &twist, opts).unwrap();
            assert!(
                !crossed
                    .certify_local_profile_domain(&[4], 1e-6, 10000, 10000, 1000000)
                    .unwrap()
                    .local_domain_certified
            );
            assert_eq!(profiles, before);
            let mut outside = square(0., 4., oblique);
            outside.extend(square(5., 6., oblique));
            let outside = Sweep::new(&outside, &points, &scale, &twist, opts).unwrap();
            assert!(
                !outside
                    .certify_local_profile_domain(&[4, 4], 1e-6, 10000, 10000, 1000000)
                    .unwrap()
                    .local_domain_certified
            );
            assert!(
                sweep
                    .certify_local_profile_domain(&[usize::MAX, 1], 1e-6, 10000, 10000, 1000000)
                    .is_err()
            );

            let mut warped = profiles.clone();
            warped[1].control_points[1][2] += 1e-12;
            let warped = Sweep::new(&warped, &points, &scale, &twist, opts).unwrap();
            assert!(
                !warped
                    .certify_local_profile_domain(&[4, 4], 1e-6, 10000, 10000, 1000000)
                    .unwrap()
                    .local_domain_certified
            );
        }
    }
    #[test]
    fn endpoint_domains_cover_affine_authored_and_guide_and_refuse_singular_frames() {
        let mut profiles = square(0., 4., false);
        profiles.extend(square(1., 2., false));
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1.);
        let twist = scalar(0.);
        let opts = super::super::Options {
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 4.,
            initial_steps: 1,
            max_steps: 16,
            max_deviation: 0.01,
        };
        let vector = |a: [f64; 3], b: [f64; 3]| Curve {
            degree: 1,
            knots: vec![2., 2., 5., 5.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        };
        let axes = vector([2., 1., 1.], [2., 1., 1.]);
        let center = vector([0.125, 0., 0.25], [0.125, 0., 0.25]);
        let axis = vector([1., 0., 1.], [0., 1., 1.]);
        let normal = vector([0., 1., 0.], [1., 0., 0.]);
        let guide = vector([1., 0., 0.], [1., 1., 10.]);
        for mode in 0..3 {
            let mut sweep = Sweep::new(&profiles, &points, &scale, &twist, opts)
                .unwrap()
                .with_affine_laws(&axes, &center)
                .unwrap();
            if mode == 1 {
                sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
            }
            if mode == 2 {
                sweep = sweep.with_orientation_guide(&guide).unwrap();
            }
            let r = sweep
                .certify_ideal_cap_domains(&[4, 4], 1e-6, 10000, 10000, 1000000)
                .unwrap();
            assert!(r.ideal_cap_domains_certified, "{r:?}");
            assert!(r.endpoint_normals.is_some());
            #[cfg(feature = "transport")]
            if mode == 1 {
                let request = value_codec::json!({"op":"curve_progressive_miter_cap_domains","profiles":profiles,"points":points,"scale":scale,"twist":twist,"axis_scale":axes,"center_law":center,"frame_axis":axis,"frame_normal":normal,"normal":opts.normal,"closed":false,"miter_limit":opts.miter_limit,"initial_steps":opts.initial_steps,"max_steps":opts.max_steps,"max_deviation":opts.max_deviation,"loopSizes":[4,4],"tolerance":1e-6,"maxPairs":10000,"maxCells":10000,"maxExactWork":1000000});
                let report = crate::transport::dispatch(request.clone()).unwrap();
                assert_eq!(report["idealCapDomainsCertified"], true);
                assert_eq!(report["continuousBound"], false);
                assert!(report["endpointNormals"].is_array());
                let mut low = request.clone();
                low["maxCells"] = value_codec::json!(r.cells - 1);
                assert_eq!(
                    crate::transport::dispatch(low).unwrap()["idealCapDomainsCertified"],
                    false
                );
                let mut bad = request;
                bad["loopSizes"] = value_codec::json!([4]);
                assert!(crate::transport::dispatch(bad).is_err());
            }

            let low = sweep
                .certify_ideal_cap_domains(&[4, 4], 1e-6, 10000, r.cells - 1, 1000000)
                .unwrap();
            assert!(!low.ideal_cap_domains_certified);
            assert!(low.endpoint_normals.is_none());
        }
        let zero = vector([0.; 3], [0.; 3]);
        let singular = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_frame_laws(&zero, &normal)
            .unwrap();
        let r = singular
            .certify_ideal_cap_domains(&[4, 4], 1e-6, 10000, 10000, 1000000)
            .unwrap();
        assert!(r.source.local_domain_certified);
        assert!(!r.ideal_cap_domains_certified);
        assert!(r.endpoint_normals.is_none());
    }
}
