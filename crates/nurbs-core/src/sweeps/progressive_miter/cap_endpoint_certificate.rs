//! Ideal open endpoint cap normals from original transport/frame/guide laws.
//! Positive scale/axes do not change these normals; center only translates.
//! This is not a filled-region or ownership certificate.
use super::{Sweep, authored_frame_certificate as frame, scalar_certificate::Status};
use crate::{Result, check};
#[derive(Clone, Debug)]
pub struct Report {
    pub normals: Option<[[[f64; 2]; 3]; 2]>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    pub fn certify_endpoint_cap_normals(&self, max_cells: usize) -> Result<Report> {
        check(
            max_cells <= 100000,
            "Endpoint cap normal budget exceeds100000",
        )?;
        let mut out = Report {
            normals: None,
            cells: 0,
            reason: Some("frame-unproved"),
        };
        if self.options.closed {
            out.reason = Some("closed-path-has-no-caps");
            return Ok(out);
        }
        let transport = &self.transport_certificate;
        if transport.status != Status::Certified {
            return Ok(out);
        }
        if transport.cells > max_cells {
            out.reason = Some("work-limit");
            return Ok(out);
        }
        out.cells = transport.cells;
        let tangents = transport.tangents.as_ref().unwrap();
        let mut normals = [[[0.; 2]; 3]; 2];
        for endpoint in 0..2 {
            let edge = if endpoint == 0 { 0 } else { tangents.len() - 1 };
            // Open endpoint planes are absent: station construction has no miter
            // projection here. The section plane normal is the ideal frame axis.
            let traversal = [endpoint as f64; 2];
            let values = if let (Some((axis,_)),Some(guide)) = (self.frame_laws,self.orientation_guide) {
                let point=self.points[if endpoint==0 {0} else {self.points.len()-1}];
                Some(frame::certify_authored_guide_values(axis,guide,self.twist,traversal,point.map(|x|[x,x]),max_cells-out.cells)?)
            } else if let Some((longitudinal, transverse)) = self.frame_laws {
                Some(frame::certify_twisted_values(
                    longitudinal,
                    transverse,
                    self.twist,
                    traversal,
                    max_cells - out.cells,
                )?)
            } else if let Some(guide) = self.orientation_guide {
                let point = self.points[if endpoint == 0 {
                    0
                } else {
                    self.points.len() - 1
                }];
                Some(frame::certify_guide_values(
                    guide,
                    self.twist,
                    traversal,
                    tangents[edge],
                    point.map(|x| [x, x]),
                    max_cells - out.cells,
                )?)
            } else {
                None
            };
            if let Some(values) = values {
                out.cells += values.cells;
                if values.status != Status::Certified {
                    out.reason = Some("endpoint-frame-unproved");
                    return Ok(out);
                }
                normals[endpoint] = values.longitudinal.unwrap();
            } else {
                normals[endpoint] = tangents[edge];
            }
        }
        out.normals = Some(normals);
        out.reason = None;
        Ok(out)
    }
}
/// Conditional geometric prerequisite; caps still need material-region audits.
#[derive(Clone, Debug)]
pub struct CapParallelReport {
    pub parallel: Option<[bool; 2]>,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    /// Parallelism uses original endpoint axes before normalization. Clamped
    /// rational endpoints equal their end control points exactly. Identical
    /// original poles define a constant rational vector even without clamping,
    /// since positive rational basis weights sum to one. No rounded curve
    /// evaluation is relabelled as an original axis.
    pub fn certify_endpoint_cap_parallelism(
        &self,
        caps: [&crate::surface::Surface; 2],
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<CapParallelReport> {
        let normals = self.certify_endpoint_cap_normals(max_cells)?;
        let mut out = CapParallelReport {
            parallel: None,
            cells: normals.cells,
            exact_work: 0,
            reason: normals.reason,
        };
        if normals.normals.is_none() {
            return Ok(out);
        }
        let mut pair = [false; 2];
        for k in 0..2 {
            let (a, b) = if let Some((axis, _)) = self.frame_laws {
                let p = axis.degree;
                let knots = &axis.knots;
                let lo = knots[p];
                let hi = knots[axis.control_points.len()];
                let constant = axis
                    .control_points
                    .iter()
                    .all(|point| point == &axis.control_points[0]);
                if !constant
                    && (axis.periodic
                        || !knots[..=p].iter().all(|x| *x == lo)
                        || !knots[axis.control_points.len()..].iter().all(|x| *x == hi))
                {
                    out.reason = Some("original-endpoint-axis-unproved");
                    return Ok(out);
                }
                let index = if k == 0 {
                    0
                } else {
                    axis.control_points.len() - 1
                };
                (
                    [0.; 3],
                    std::array::from_fn(|i| axis.control_points[index][i]),
                )
            } else {
                let index = if k == 0 { 0 } else { self.points.len() - 2 };
                (self.points[index], self.points[index + 1])
            };
            let r = super::cap_retained_plane_certificate::inspect_parallel_axis(
                caps[k],
                a,
                b,
                max_exact_work - out.exact_work,
            );
            out.exact_work += r.work;
            if !r.parallel && r.reason != Some("planes-not-parallel") {
                out.reason = r.reason;
                return Ok(out);
            }
            pair[k] = r.parallel;
        }
        out.parallel = Some(pair);
        out.reason = None;
        Ok(out)
    }
}

/// Conditional geometric prerequisite; caps still need material-region audits.
#[derive(Clone, Debug)]
pub struct ProjectionReport {
    pub normal_dots: Option<[[f64; 2]; 2]>,
    pub reverses_orientation: Option<[bool; 2]>,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    pub fn certify_endpoint_cap_projection(
        &self,
        caps: [&crate::surface::Surface; 2],
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<ProjectionReport> {
        let mut out = ProjectionReport {
            normal_dots: None,
            reverses_orientation: None,
            cells: 0,
            exact_work: 0,
            reason: Some("work-limit"),
        };
        check(
            max_cells <= 100000,
            "Endpoint projection budget exceeds100000",
        )?;
        if max_cells < 2 {
            return Ok(out);
        }
        let ideal = self.certify_endpoint_cap_normals(max_cells - 2)?;
        out.cells = ideal.cells;
        let Some(normals) = ideal.normals else {
            out.reason = ideal.reason;
            return Ok(out);
        };
        let mut dots = [[0.; 2]; 2];
        let mut reverse = [false; 2];
        for k in 0..2 {
            let retained = super::cap_retained_plane_certificate::inspect(
                caps[k],
                max_exact_work - out.exact_work,
            );
            out.exact_work += retained.work;
            let Some(normal) = retained.normal else {
                out.reason = Some("retained-plane-unproved");
                return Ok(out);
            };
            let projection = super::cap_projection_certificate::certify(normals[k], normal, 1)?;
            out.cells += projection.cells;
            if !projection.projection_regular {
                out.reason = Some("projection-unproved");
                return Ok(out);
            }
            dots[k] = projection.normal_dot.unwrap();
            reverse[k] = projection.reverses_orientation.unwrap();
        }
        out.normal_dots = Some(dots);
        out.reverses_orientation = Some(reverse);
        out.reason = None;
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    fn scalar(x: f64) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![x, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        }
    }
    fn vector(a: [f64; 3], b: [f64; 3]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![2., 2., 5., 5.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        }
    }
    #[test]
    fn original_endpoint_normals_and_shared_budget() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
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
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let r = sweep.certify_endpoint_cap_normals(100).unwrap();
        for normal in r.normals.unwrap() {
            assert!(normal[2][0] <= 1. && 1. <= normal[2][1]);
        }
        let axis = vector([1., 0., 1.], [0., 1., 1.]);
        let transverse = vector([0., 1., 0.], [1., 0., 0.]);
        let authored = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_frame_laws(&axis, &transverse)
            .unwrap();
        let r = authored.certify_endpoint_cap_normals(100).unwrap();
        let n = r.normals.unwrap();
        let x = 1. / 2f64.sqrt();
        assert!(n[0][0][0] <= x && x <= n[0][0][1]);
        assert!(n[1][1][0] <= x && x <= n[1][1][1]);

        let retained = [[0., 0.], [0., 0.], [1., 1.]];
        for normal in n {
            let projection =
                super::super::cap_projection_certificate::certify(normal, retained, 1).unwrap();
            assert!(projection.projection_regular);
        }
        let cap = crate::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let projection = authored
            .certify_endpoint_cap_projection([&cap, &cap], 100, 1000000)
            .unwrap();
        assert!(projection.normal_dots.is_some());
        let tilted = authored
            .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
            .unwrap();
        assert_eq!(tilted.parallel, Some([false, false]));
        let original = sweep
            .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
            .unwrap();
        assert_eq!(original.parallel, Some([true, true]));
        assert!(
            sweep
                .certify_endpoint_cap_parallelism([&cap, &cap], 100, original.exact_work - 1)
                .unwrap()
                .parallel
                .is_none()
        );
        let fixed_axis = vector([0., 0., 2.], [0., 0., 3.]);
        let mut constant_axis = vector([0., 0., 2.], [0., 0., 2.]);
        constant_axis.knots = vec![1., 2., 5., 6.];
        constant_axis.weights = vec![0.5, 3.];
        let constant_sweep = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_frame_laws(&constant_axis, &transverse)
            .unwrap();
        assert!(
            constant_sweep
                .certify_endpoint_cap_normals(100)
                .unwrap()
                .normals
                .is_some()
        );
        assert_eq!(
            constant_sweep
                .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
                .unwrap()
                .parallel,
            Some([true, true])
        );
        let mut varying_axis = constant_axis.clone();
        varying_axis.control_points[1][0] = f64::EPSILON;
        let varying = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_frame_laws(&varying_axis, &transverse)
            .unwrap();
        let refused = varying
            .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
            .unwrap();
        assert!(refused.parallel.is_none());
        assert_eq!(refused.reason, Some("original-endpoint-axis-unproved"));
        let fixed = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_frame_laws(&fixed_axis, &transverse)
            .unwrap();
        assert_eq!(
            fixed
                .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
                .unwrap()
                .parallel,
            Some([true, true])
        );
        #[cfg(feature = "transport")]
        {
            let request = value_codec::json!({"op":"curve_progressive_miter_cap_projection","profiles":profiles,"points":points,"scale":scale,"twist":twist,"frame_axis":axis,"frame_normal":transverse,"normal":opts.normal,"closed":false,"miter_limit":opts.miter_limit,"initial_steps":opts.initial_steps,"max_steps":opts.max_steps,"max_deviation":opts.max_deviation,"caps":[cap.clone(),cap.clone()],"maxCells":100,"maxExactWork":1000000});
            let report = crate::transport::dispatch(request.clone()).unwrap();
            assert!(report["normalDots"].is_array());
            assert_eq!(report["continuousBound"], false);
            let mut parallel_request = request.clone();
            parallel_request["op"] = value_codec::json!("curve_progressive_miter_cap_parallelism");
            let tilted = crate::transport::dispatch(parallel_request.clone()).unwrap();
            assert_eq!(tilted["parallel"], value_codec::json!([false, false]));
            parallel_request["frame_axis"] = value_codec::json!(fixed_axis);
            let parallel = crate::transport::dispatch(parallel_request.clone()).unwrap();
            assert_eq!(parallel["parallel"], value_codec::json!([true, true]));
            assert_eq!(parallel["continuousBound"], false);
            let mut exhausted_parallel = parallel_request.clone();
            exhausted_parallel["maxExactWork"] =
                value_codec::json!(parallel["exactWork"].as_u64().unwrap() - 1);
            assert!(crate::transport::dispatch(exhausted_parallel).unwrap()["parallel"].is_null());
            parallel_request["caps"] = value_codec::json!([cap.clone()]);
            assert!(crate::transport::dispatch(parallel_request).is_err());
            let mut exhausted = request.clone();
            exhausted["maxCells"] = value_codec::json!(0);
            assert!(crate::transport::dispatch(exhausted).unwrap()["normalDots"].is_null());
            let mut missing = request.clone();
            missing["caps"] = value_codec::json!([cap.clone()]);
            assert!(crate::transport::dispatch(missing).is_err());
            let mut invalid = request.clone();
            invalid["maxExactWork"] = value_codec::json!(-1);
            assert!(crate::transport::dispatch(invalid).is_err());
            let mut invalid = request.clone();
            invalid["maxCells"] = value_codec::json!(100001);
            assert!(crate::transport::dispatch(invalid).is_err());
        }

        assert!(
            authored
                .certify_endpoint_cap_projection([&cap, &cap], projection.cells - 1, 1000000)
                .unwrap()
                .normal_dots
                .is_none()
        );
        assert!(
            authored
                .certify_endpoint_cap_projection([&cap, &cap], 100, projection.exact_work - 1)
                .unwrap()
                .normal_dots
                .is_none()
        );
        let axes = vector([2., 3., 1.], [2., 3., 1.]);
        let center = vector([0.125, 0., 0.25], [0.125, 0., 0.25]);
        let affine = authored.with_affine_laws(&axes, &center).unwrap();
        assert_eq!(
            affine.certify_endpoint_cap_normals(100).unwrap().normals,
            Some(n)
        );
        let low = affine.certify_endpoint_cap_normals(r.cells - 1).unwrap();
        assert!(low.normals.is_none());
        assert!(low.cells < r.cells);
        let rail = vector([1., 0., 0.], [1., 1., 10.]);
        let guide = Sweep::new(&profiles, &points, &scale, &twist, opts)
            .unwrap()
            .with_orientation_guide(&rail)
            .unwrap();
        let r = guide.certify_endpoint_cap_normals(100).unwrap();
        assert!(r.normals.is_some());
        assert_eq!(
            guide
                .certify_endpoint_cap_parallelism([&cap, &cap], 100, 1000000)
                .unwrap()
                .parallel,
            Some([true, true])
        );
        assert!(
            guide
                .certify_endpoint_cap_normals(r.cells - 1)
                .unwrap()
                .normals
                .is_none()
        );
        assert!(
            guide
                .certify_endpoint_cap_normals(0)
                .unwrap()
                .normals
                .is_none()
        );
    }
}
