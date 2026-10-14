//! Original endpoint affine images for complete filled-cap comparisons.
//! A point enclosure is neither a retained-cap nor a whole-body certificate.
use super::{MultiSweep, constant_vector_law, path_is_closed, profile_domain};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
use crate::numerics::interval_vec3::{dot_tight, sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate as frame, scalar_certificate::Status, vector_certificate,
};

pub struct Report {
    pub points: Option<Vec<[[[f64; 2]; 3]; 2]>>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}

fn decode(v: [[f64; 2]; 3]) -> Result<[I; 3]> {
    Ok([I::new(v[0][0], v[0][1])?, I::new(v[1][0], v[1][1])?, I::new(v[2][0], v[2][1])?])
}

impl MultiSweep<'_> {
    /// Borrow original profiles, before transport, decomposition or correction.
    pub fn source_profiles(&self) -> Vec<&Curve> {
        self.sweeps.iter().map(|s| s.profile).collect()
    }

    /// The original transport is affine in a profile point. Thus this map also
    /// encloses exact Bezier extraction coefficients, without substituting a
    /// rounded retained section for the original source. Shared frame, law,
    /// anchor and point work all consume this single allowance.
    pub fn certify_endpoint_points(&self, points: &[[f64; 3]], max_cells: usize) -> Result<Report> {
        check(points.len() <= 4096 && points.iter().flatten().all(|x| x.is_finite())
            && max_cells <= 100_000, "Invalid smooth endpoint points/budget")?;
        let mut out = Report {points: None, cells: 0, reason: Some("endpoint-frame-unproved")};
        let sweep = &self.sweeps[0];
        if path_is_closed(sweep.path)? {
            out.reason = Some("closed-path-has-no-caps");
            return Ok(out);
        }
        if points.is_empty() {
            out.reason = Some("empty-point-family");
            return Ok(out);
        }
        let start = vector_certificate::certify_values_traversal(sweep.path, [0.; 2], max_cells, false)?;
        out.cells += start.cells;
        let Some(start) = start.value else {return Ok(out)};
        let zero = constant_vector_law([0.; 3])?;
        let Some(initial) = profile_domain::original_frame(sweep, &zero, [0.; 2], max_cells-out.cells, true)?
            else {return Ok(out)};
        out.cells += initial.cells;
        if initial.status != Status::Certified {return Ok(out)}
        if points.len() > max_cells-out.cells {
            out.reason = Some("endpoint-point-budget-exhausted");
            return Ok(out);
        }
        // Interval products below are actual work even when source/frame laws
        // are constant; a large point family cannot receive a zero-cost map.
        out.cells += points.len();
        let origin = decode(start)?;
        let basis = [decode(initial.transverse.unwrap())?, decode(initial.binormal.unwrap())?,
            decode(initial.longitudinal.unwrap())?];
        let coordinates = points.iter().map(|p| {
            let delta = sub(p.map(I::point), origin)?;
            Ok([dot_tight(delta, basis[0])?, dot_tight(delta, basis[1])?, dot_tight(delta, basis[2])?]
                .map(|x| [x.lo, x.hi]))
        }).collect::<Result<Vec<_>>>()?;
        let anchor = if sweep.contact_point.is_some() {
            let proof = super::contact_anchor::certify(sweep, max_cells-out.cells)?;
            out.cells += proof.cells;
            if proof.status != Status::Certified {
                out.reason = proof.reason;
                return Ok(out);
            }
            Some(proof.coordinates.unwrap()[0])
        } else {None};
        let mut result = vec![[[[0.; 2]; 3]; 2]; points.len()];
        for end in 0..2 {
            if points.len() > max_cells-out.cells {
                out.reason = Some("endpoint-point-budget-exhausted");
                return Ok(out);
            }
            out.cells += points.len();
            let fraction = [end as f64; 2];
            let fit = if let Some(anchor) = anchor {
                let proof = frame::certify_contact_fit_value(sweep.path, sweep.orientation_guide.unwrap(),
                    sweep.scale, sweep.affine_laws, anchor, fraction, max_cells-out.cells)?;
                out.cells += proof.cells;
                if proof.status != Status::Certified {
                    out.reason = Some("endpoint-contact-fit-unproved");
                    return Ok(out);
                }
                let value = proof.value.unwrap();
                if value[0] <= 0. {
                    out.reason = Some("endpoint-contact-fit-positivity-unproved");
                    return Ok(out);
                }
                Some(I::new(value[0], value[1])?)
            } else {None};
            let Some(values) = profile_domain::original_frame(sweep, sweep.twist, fraction,
                max_cells-out.cells, false)? else {return Ok(out)};
            let image = frame::control_values_from_owned_frame(sweep.path, sweep.scale, sweep.affine_laws,
                &coordinates, fraction, max_cells-out.cells, values, fit)?;
            out.cells += image.cells;
            if image.status != Status::Certified {
                out.reason = image.reason;
                return Ok(out);
            }
            for (pair, point) in result.iter_mut().zip(image.values.unwrap()) {pair[end] = point;}
        }
        out.points = Some(result);
        out.reason = None;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Options, Orientation, Spacing};
    use crate::primitives::{circle, line};

    fn options(orientation: Orientation, spacing: Spacing) -> Options {
        Options {normal: [1., 0., 0.], orientation, spacing,
            initial_sections: 3, max_sections: 9, max_deviation: 0.1}
    }
    fn contains(actual: [[f64; 2]; 3], expected: [f64; 3]) {
        for k in 0..3 {assert!(actual[k][0] <= expected[k] && expected[k] <= actual[k][1],
            "axis {k}: {:?} misses {}", actual[k], expected[k]);}
    }

    #[test]
    fn affine_and_authored_endpoints_share_work_and_keep_original_source() {
        let profiles = [circle([0.; 3], [0., 0., 1.], 1.).unwrap()];
        let path = line([0.; 3], [0., 0., 10.]).unwrap();
        let scale = constant_vector_law([2., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let axes = constant_vector_law([2., 3., 4.]).unwrap();
        let center = constant_vector_law([0.25, 0.5, 0.75]).unwrap();
        let axis = constant_vector_law([0., 0., 1.]).unwrap();
        let normal = constant_vector_law([1., 0., 0.]).unwrap();
        for spacing in [Spacing::Parameter, Spacing::ArcLength {tolerance: 0.001, max_cells: 1000}] {
            for authored in [false, true] {
                let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist,
                    options(Orientation::Fixed, spacing)).unwrap().with_affine_laws(&axes, &center).unwrap();
                if authored {sweep = sweep.with_frame_laws(&axis, &normal).unwrap();}
                assert!(std::ptr::eq(sweep.source_profiles()[0], &profiles[0]));
                let points = [[1., 2., 3.], [-0.5, 0.25, 0.]];
                let proof = sweep.certify_endpoint_points(&points, 1000).unwrap();
                assert!(proof.reason.is_none(), "{:?}", proof.reason);
                assert!(proof.cells >= 3*points.len());
                let image = proof.points.unwrap();
                for (i, p) in points.iter().enumerate() {for end in 0..2 {
                    contains(image[i][end], [4.*p[0]+0.25, 6.*p[1]+0.5, 8.*p[2]+0.75+10.*end as f64]);
                }}
                let short = sweep.certify_endpoint_points(&points, proof.cells-1).unwrap();
                assert!(short.points.is_none() && short.cells <= proof.cells-1);
                let empty = sweep.certify_endpoint_points(&points, 0).unwrap();
                assert!(empty.points.is_none() && empty.cells == 0);
            }
        }
    }

    #[test]
    fn curved_original_frames_enclose_analytic_endpoint_rotation() {
        let profiles = [circle([0.; 3], [1., 0., 0.], 1.).unwrap()];
        let path = Curve {degree: 2, knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![5., 0., 0.], vec![5., 5., 0.]],
            weights: vec![1.; 3], periodic: false};
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        for orientation in [Orientation::Frenet, Orientation::RotationMinimizing, Orientation::FixedNormal, Orientation::Fixed] {
            let mut opt = options(orientation, Spacing::Parameter);opt.normal = [0., 0., 1.];
            let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, opt).unwrap();
            let proof = sweep.certify_endpoint_points(&[[0., 1., 0.]], 10000).unwrap();
            assert!(proof.points.is_some(), "{orientation:?}: {:?}", proof.reason);
            let image = proof.points.unwrap()[0];contains(image[0], [0., 1., 0.]);
            contains(image[1], if orientation == Orientation::Fixed {[5., 6., 0.]} else {[4., 5., 0.]});
        }
    }

    #[test]
    fn contact_endpoints_keep_anchor_fit_and_refuse_closed_caps() {
        let profiles = [circle([0.; 3], [0., 0., 1.], 0.5).unwrap()];
        let path = line([0.; 3], [0., 0., 10.]).unwrap();
        let guide = line([1., 0., 0.], [1., 0., 10.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = MultiSweep::new(&profiles, &path, &scale, &twist,
            options(Orientation::RotationMinimizing, Spacing::Parameter)).unwrap()
            .with_contact_guide(&guide, 0, 0.).unwrap();
        let proof = sweep.certify_endpoint_points(&[[0.5, 0., 0.]], 10000).unwrap();
        assert!(proof.points.is_some(), "{:?}", proof.reason);
        let image = proof.points.unwrap()[0];contains(image[0], [1., 0., 0.]);contains(image[1], [1., 0., 10.]);
        let closed_path = circle([0.; 3], [0., 0., 1.], 5.).unwrap();
        let closed = MultiSweep::new(&profiles, &closed_path, &scale, &twist,
            options(Orientation::RotationMinimizing, Spacing::Parameter)).unwrap();
        let refused = closed.certify_endpoint_points(&[[0.5, 0., 0.]], 10000).unwrap();
        assert!(refused.points.is_none());assert_eq!(refused.reason, Some("closed-path-has-no-caps"));
        assert_eq!(refused.cells, 0);
    }
}
