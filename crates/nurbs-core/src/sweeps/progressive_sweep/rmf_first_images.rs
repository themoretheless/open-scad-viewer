//! Joint original-position/Bishop-frame first images. No second derivatives
//! or retained surface smoothness are inferred from this first-order owner.
use super::{Sweep, rmf_transport, source_jets::SourceJets};
use crate::numerics::interval_vec3::{Vec3, add, cross, div, norm, scale};
use crate::sweeps::progressive_miter::{scalar_certificate::Status, trigonometric_certificate};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Copy)]
pub(super) struct Image {
    pub value: Vec3,
    pub first: Vec3,
}
impl Image {
    fn constant(v: Vec3) -> Self {
        Self {
            value: v,
            first: [I::point(0.); 3],
        }
    }
    fn add(self, b: Self) -> Result<Self> {
        Ok(Self {
            value: add(self.value, b.value)?,
            first: add(self.first, b.first)?,
        })
    }
    fn product(self, value: I, first: I) -> Result<Self> {
        Ok(Self {
            value: scale(self.value, value)?,
            first: add(scale(self.first, value)?, scale(self.value, first)?)?,
        })
    }
}
fn decode(v: [[f64; 2]; 3]) -> Result<Vec3> {
    Ok([
        I::new(v[0][0], v[0][1])?,
        I::new(v[1][0], v[1][1])?,
        I::new(v[2][0], v[2][1])?,
    ])
}
struct Law {
    owner: Option<SourceJets>,
    constant: Option<Image>,
    width: I,
}
impl Law {
    fn new(curve: &Curve, budget: usize) -> Result<(usize, Option<Self>)> {
        curve.validate()?;
        check(
            curve.control_points.iter().all(|p| p.len() == 3),
            "First image laws require XYZ controls",
        )?;
        let [a, b] = curve.domain();
        let width = I::point(b).sub(I::point(a))?;
        // Original rational partition of unity makes equal poles constant,
        // independently of weights, knots, active domain, or periodicity.
        let mut spent = 0;
        let mut constant = true;
        for p in &curve.control_points {
            if spent == budget {
                return Ok((spent, None));
            }
            spent += 1;
            constant &= p == &curve.control_points[0];
        }
        if constant {
            let value = std::array::from_fn(|k| I::point(curve.control_points[0][k]));
            return Ok((
                spent,
                Some(Self {
                    owner: None,
                    constant: Some(Image::constant(value)),
                    width,
                }),
            ));
        }
        let (work, owner) = SourceJets::new_order(curve, 1, budget - spent)?;
        spent += work;
        Ok((
            spent,
            owner.map(|owner| Self {
                owner: Some(owner),
                constant: None,
                width,
            }),
        ))
    }
    fn query(&self, interval: [f64; 2], budget: usize) -> Result<(usize, Option<Image>, bool)> {
        if let Some(image) = self.constant {
            return Ok((0, Some(image), true));
        }
        let report = self.owner.as_ref().unwrap().query(interval, budget)?;
        let image = report
            .jets
            .map(|j| -> Result<Image> {
                Ok(Image {
                    value: decode(j[0])?,
                    first: scale(decode(j[1])?, self.width)?,
                })
            })
            .transpose()?;
        Ok((report.cells, image, report.single_span))
    }
}
pub(super) struct Owner {
    scale: Law,
    twist: Law,
    affine: Option<(Law, Law)>,
    width: I,
    constant_normal: Option<Vec3>,
}
pub(super) struct Report {
    pub cells: usize,
    pub images: Option<Vec<Image>>,
    pub continuous_laws: bool,
    pub pose: Option<([Image; 4], Image)>,
}
impl Owner {
    pub(super) fn new(sweep: &Sweep<'_>, budget: usize) -> Result<(usize, Option<Self>)> {
        let mut cells = 0;
        let mut law = |curve: &Curve| -> Result<Option<Law>> {
            let (work, owner) = Law::new(curve, budget - cells)?;
            cells += work;
            Ok(owner)
        };
        let Some(scale) = law(sweep.scale)? else {
            return Ok((cells, None));
        };
        let Some(twist) = law(sweep.twist)? else {
            return Ok((cells, None));
        };
        let affine = if let Some((a, b)) = sweep.affine_laws {
            let Some(a) = law(a)? else {
                return Ok((cells, None));
            };
            let Some(b) = law(b)? else {
                return Ok((cells, None));
            };
            Some((a, b))
        } else {
            None
        };
        let [a, b] = sweep.path.domain();
        let width = I::point(b).sub(I::point(a))?;
        // Equal original coordinate coefficients and an axial seed prove
        // N'=0 exactly. Closed unit-tangent identity is supplied by transport;
        // its source holonomy is then exactly zero, regardless of rounded
        // constructor correction or the general angle enclosure's width.
        let constant_normal =
            if super::authored_error::original_planar_rmf(sweep.path, sweep.options.normal) {
                let charge = sweep.path.control_points.len();
                if charge > budget - cells {
                    return Ok((cells, None));
                }
                cells += charge;
                Some(rmf_transport::unit(sweep.options.normal.map(I::point))?)
            } else {
                None
            };
        Ok((
            cells,
            Some(Self {
                scale,
                twist,
                affine,
                width,
                constant_normal,
            }),
        ))
    }
    pub(super) fn query(
        &self,
        transport: &rmf_transport::OriginalRmfTransportReport,
        qs: &[[[f64; 2]; 3]],
        interval: [f64; 2],
        budget: usize,
    ) -> Result<Report> {
        let mut out = Report {
            cells: 0,
            images: None,
            continuous_laws: false,
            pose: None,
        };
        if transport.status != Status::Certified {
            return Ok(out);
        }
        let Some(source) = rmf_transport::source_jets(transport, interval, budget)? else {
            return Ok(out);
        };
        out.cells += source.cells;
        let Some(jets) = source.jets else {
            return Ok(out);
        };
        let (normal, correction) = if let Some(normal) = self.constant_normal {
            (normal, (I::point(0.), I::point(0.)))
        } else {
            let (frame, correction) = rmf_transport::original_frame_with_correction(
                transport,
                interval,
                budget - out.cells,
                Some(jets[1]),
            )?;
            out.cells += frame.cells;
            let Some(correction) = correction.filter(|_| frame.status == Status::Certified) else {
                return Ok(out);
            };
            (decode(frame.transverse.unwrap())?, correction)
        };
        let mut continuous = true;
        let mut law = |owner: &Law| -> Result<Option<Image>> {
            let (work, image, single) = owner.query(interval, budget - out.cells)?;
            out.cells += work;
            continuous &= single;
            Ok(image)
        };
        let Some(uniform) = law(&self.scale)? else {
            return Ok(out);
        };
        let Some(twist) = law(&self.twist)? else {
            return Ok(out);
        };
        let (axes, center) = if let Some((a, b)) = &self.affine {
            let Some(a) = law(a)? else {
                return Ok(out);
            };
            let Some(b) = law(b)? else {
                return Ok(out);
            };
            (a, b)
        } else {
            (
                Image::constant([I::point(1.); 3]),
                Image::constant([I::point(0.); 3]),
            )
        };
        let result = (|| -> Result<(Vec<Image>, [Image; 4], Image)> {
            let v = decode(jets[1])?;
            let a = decode(jets[2])?;
            let speed = norm(v)?;
            let t = rmf_transport::unit(v)?;
            let n = normal;
            let omega = scale(div(cross(v, a)?, speed.mul(speed)?)?, self.width)?;
            let td = cross(omega, t)?;
            let nd = if self.constant_normal.is_some() {
                [I::point(0.); 3]
            } else {
                cross(omega, n)?
            };
            let b = cross(t, n)?;
            let bd = add(cross(td, n)?, cross(t, nd)?)?;
            let tangent = Image {
                value: t,
                first: td,
            };
            let normal = Image {
                value: n,
                first: nd,
            };
            let binormal = Image {
                value: b,
                first: bd,
            };
            let angle = twist.value[0].add(correction.0)?;
            let angle_first = twist.first[0].add(correction.1.mul(speed)?.mul(self.width)?)?;
            let trig = trigonometric_certificate::certify([angle.lo, angle.hi])?;
            let c = I::new(trig.cos[0], trig.cos[1])?;
            let s = I::new(trig.sin[0], trig.sin[1])?;
            let cd = I::point(0.).sub(s.mul(angle_first)?)?;
            let sd = c.mul(angle_first)?;
            let normal = normal.product(c, cd)?.add(binormal.product(s, sd)?)?;
            let binormal = Image {
                value: cross(tangent.value, normal.value)?,
                first: add(
                    cross(tangent.first, normal.value)?,
                    cross(tangent.value, normal.first)?,
                )?,
            };
            let directions = [normal, binormal, tangent];
            let path = Image {
                value: decode(jets[0])?,
                first: scale(v, self.width)?,
            };
            let mut pose = [Image::constant([I::point(0.); 3]); 4];
            for k in 0..3 {
                pose[0] = pose[0].add(directions[k].product(center.value[k], center.first[k])?)?;
                pose[k + 1] = directions[k].product(
                    uniform.value[0].mul(axes.value[k])?,
                    uniform.first[0]
                        .mul(axes.value[k])?
                        .add(uniform.value[0].mul(axes.first[k])?)?,
                )?;
            }
            let images = qs
                .iter()
                .map(|q| {
                    let q = decode(*q)?;
                    let mut image = path;
                    image = image.add(pose[0])?;
                    for k in 0..3 {
                        image = image.add(pose[k + 1].product(q[k], I::point(0.))?)?;
                    }
                    Ok(image)
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((images, pose, path))
        })();
        if let Ok((images, pose, path)) = result {
            out.images = Some(images);
            out.pose = Some((pose, path));
            out.continuous_laws = continuous;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joint_first_images_enclose_independent_pose_laws_and_refuse_partial_work() {
        let path = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 5., 5., 5.],
            control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let line = |a: [f64; 3], b: [f64; 3], domain: [f64; 2]| Curve {
            degree: 1,
            knots: vec![domain[0], domain[0], domain[1], domain[1]],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        };
        let scale = line([1., 0., 0.], [2., 0., 0.], [7., 11.]);
        let twist = line([0.; 3], [0.3, 0., 0.], [13., 17.]);
        let axes = line([1., 2., 3.], [1.1, 2., 3.], [19., 23.]);
        let center = line([0.; 3], [1., 0., -0.2], [29., 31.]);
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            super::super::Options {
                normal: [0., 0., 1.],
                orientation: super::super::Orientation::RotationMinimizing,
                spacing: super::super::Spacing::Parameter,
                initial_sections: 5,
                max_sections: 17,
                max_deviation: 1.,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
        let transport = rmf_transport::certify_original_rmf_transport_shared(
            &path,
            [0., 0., 1.],
            512,
            10000,
            false,
        )
        .unwrap();
        assert_eq!(transport.status, Status::Certified);
        let (work, owner) = Owner::new(&sweep, 10000).unwrap();
        assert!(work > 0);
        assert!(Owner::new(&sweep, 0).unwrap().1.is_none());
        assert!(Owner::new(&sweep, work - 1).unwrap().1.is_none());
        let owner = owner.unwrap();
        let qs = [
            [[1.; 2], [-2.; 2], [0.5; 2]],
            [[-0.25, 0.75], [0., 1.], [-0.5, 0.5]],
        ];
        for interval in [[0.; 2], [0.125, 0.1875], [0.375, 0.5], [1.; 2]] {
            let report = owner.query(&transport, &qs, interval, 10000).unwrap();
            assert!(report.continuous_laws);
            for budget in [0, report.cells - 1] {
                let short = owner.query(&transport, &qs, interval, budget).unwrap();
                assert!(short.cells <= budget && short.images.is_none());
            }
            let images = report.images.unwrap();
            for f in [0., 0.25, 0.5, 0.75, 1.] {
                let t = interval[0] + (interval[1] - interval[0]) * f;
                let r = (1. + 4. * t * t).sqrt();
                let tangent = [1. / r, 2. * t / r, 0.];
                let td = [-4. * t / r.powi(3), 2. / r.powi(3), 0.];
                let b = [2. * t / r, -1. / r, 0.];
                let bd = [2. / r.powi(3), 4. * t / r.powi(3), 0.];
                let (s, c) = (0.3 * t).sin_cos();
                let n = std::array::from_fn::<_, 3, _>(|k| [0., 0., 1.][k] * c + b[k] * s);
                let nd = std::array::from_fn::<_, 3, _>(|k| {
                    -[0., 0., 1.][k] * s * 0.3 + bd[k] * s + b[k] * c * 0.3
                });
                let b = std::array::from_fn::<_, 3, _>(|k| b[k] * c - [0., 0., 1.][k] * s);
                let bd = std::array::from_fn::<_, 3, _>(|k| {
                    bd[k] * c
                        - (if k == 0 {
                            2. * t / r
                        } else if k == 1 {
                            -1. / r
                        } else {
                            0.
                        }) * s
                            * 0.3
                        - [0., 0., 1.][k] * c * 0.3
                });
                for (j, q) in qs.iter().enumerate() {
                    for fraction in [0., 0.5, 1.] {
                        let q = std::array::from_fn::<_, 3, _>(|k| {
                            q[k][0] + fraction * (q[k][1] - q[k][0])
                        });
                        let a = [1. + 0.1 * t, 2., 3.];
                        let ad = [0.1, 0., 0.];
                        let center = [t, 0., -0.2 * t];
                        let cd = [1., 0., -0.2];
                        let mut value = [t, t * t, 0.];
                        let mut first = [1., 2. * t, 0.];
                        for k in 0..3 {
                            let coefficient = (1. + t) * a[k] * q[k] + center[k];
                            let derivative = (a[k] + (1. + t) * ad[k]) * q[k] + cd[k];
                            for axis in 0..3 {
                                value[axis] += coefficient * [n, b, tangent][k][axis];
                                first[axis] += derivative * [n, b, tangent][k][axis]
                                    + coefficient * [nd, bd, td][k][axis];
                            }
                        }
                        for k in 0..3 {
                            assert!(
                                images[j].value[k].lo <= value[k]
                                    && value[k] <= images[j].value[k].hi,
                                "value {interval:?}/{j}/{k}"
                            );
                            assert!(
                                images[j].first[k].lo <= first[k]
                                    && first[k] <= images[j].first[k].hi,
                                "first {interval:?}/{j}/{k}: {:?} / {first:?}",
                                images[j].first
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod closed_plane_tests;
