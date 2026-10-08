//! Progressive spatial polyline miter sweep with scalar laws and closed holonomy correction.
pub mod profile_domain_certificate;
pub mod cap_retained_plane_certificate;
pub mod cap_endpoint_certificate;
pub mod cap_projection_certificate;
pub mod frame_certificate;
pub mod authored_frame_certificate;
pub mod level_certificate;
pub mod scalar_certificate;
pub mod trigonometric_certificate;
pub mod vector_certificate;
/// Certificates for the authored laws and their moving frames.
pub mod law_certificates {
    pub use super::{scalar_certificate as scalar, vector_certificate as vector,
        trigonometric_certificate as trigonometric, frame_certificate as frame,
        authored_frame_certificate as authored_frame, level_certificate as level};
}
/// Miter-specific profile domains and endpoint cap prerequisites.
pub mod boundary_certificates {
    pub use super::{profile_domain_certificate as profile_domain,
        cap_retained_plane_certificate as retained_plane,
        cap_endpoint_certificate as endpoint, cap_projection_certificate as projection};
}
mod law;
use crate::{Result, check, sweep_support::vec3_ext::norm, curve::Curve};
use law::{law, scalar_bounds, validate_law};
use math_core::{cross, dot, sub};
type V = [f64; 3];
fn unit(v: V) -> Result<V> {
    crate::sweep_support::vec3_ext::unit(v, "Miter direction must be finite and nonzero")
}
fn rotate(n: V, t: V, a: f64) -> V {
    let (s, c) = a.sin_cos();
    let b = cross(t, n);
    std::array::from_fn(|k| c * n[k] + s * b[k])
}
fn bisector(a: V, b: V, limit: f64) -> Result<V> {
    let c = dot(a, b).clamp(-1., 1.);
    check(c > -1. + 1e-12, "Miter path reverses direction")?;
    check(
        (2. / (1. + c)).sqrt() <= limit,
        "Miter corner exceeds its limit",
    )?;
    unit(std::array::from_fn(|k| a[k] + b[k]))
}
fn transport(n: V, a: V, b: V, limit: f64) -> Result<V> {
    let h = bisector(a, b, limit)?;
    let r = sub(n, h.map(|x| x * dot(n, b) / dot(a, h)));
    unit(sub(r, b.map(|x| x * dot(r, b))))
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub normal: V,
    pub closed: bool,
    pub miter_limit: f64,
    pub initial_steps: usize,
    pub max_steps: usize,
    pub max_deviation: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub accepted: bool,
    pub phase_resolved: bool,
    pub frame_transport_certified: bool,
    pub frame_transport_reason: Option<&'static str>,
    pub steps: usize,
    pub sections: usize,
    pub stations: usize,
    pub sampled_control_deviation: f64,
    pub continuous_error_upper: f64,
    pub certified_error_upper: Option<f64>,
    pub affine_laws_applied: bool,
    pub authored_frames_applied: bool,
    pub orientation_guide_applied: bool,
    pub endpoint_contour_error_upper: Option<[f64;2]>,
    pub error_certificate_cells: usize,
    pub error_certificate_reason: Option<&'static str>,
    pub profile_regularity_certified: bool,
    pub wall_regularity_certified: Option<bool>,
    pub regularity_cells: usize,
    pub unresolved_wall_patches: Vec<[usize; 2]>,
    pub budget: f64,
    pub closed_path: bool,
    pub holonomy_correction: f64,
}
#[derive(Clone, Debug)]
pub struct Level {
    pub sections: Vec<Vec<Curve>>,
    pub report: Report,
}
#[derive(Clone, Debug)]
pub struct Approximation {
    pub sections: Option<Vec<Vec<Curve>>>,
    pub levels: Vec<Report>,
}
pub struct Sweep<'a> {
    profiles: &'a [Curve],
    points: &'a [V],
    scale: &'a Curve,
    twist: &'a Curve,
    affine_laws: Option<(&'a Curve,&'a Curve)>,
    frame_laws: Option<(&'a Curve,&'a Curve)>,
    orientation_guide: Option<&'a Curve>,
    options: Options,
    tangents: Vec<V>,
    normals: Vec<V>,
    planes: Vec<Option<V>>,
    lengths: Vec<f64>,
    offsets: Vec<Vec<[f64; 2]>>,
    correction: f64,
    transport_certificate: frame_certificate::Report,
    next: Option<usize>,
}
impl<'a> Sweep<'a> {
    pub fn new(
        profiles: &'a [Curve],
        points: &'a [V],
        scale: &'a Curve,
        twist: &'a Curve,
        options: Options,
    ) -> Result<Self> {
        check(
            (1..=64).contains(&profiles.len()),
            "Progressive miter needs 1..64 profiles",
        )?;
        check(
            if options.closed {
                (3..=16).contains(&points.len())
            } else {
                (2..=17).contains(&points.len())
            },
            "Miter needs 2..17 open or 3..16 cyclic sites",
        )?;
        check(
            points.first() != points.last() && points.iter().flatten().all(|x| x.is_finite()),
            "Miter sites must be finite and omit a repeated endpoint",
        )?;
        check(
            options.miter_limit.is_finite() && options.miter_limit >= 1.,
            "Miter limit must be finite and at least one",
        )?;
        check(
            options.max_deviation.is_finite() && options.max_deviation >= 0.,
            "Miter budget must be finite and nonnegative",
        )?;
        let edges = points.len() - usize::from(!options.closed);
        check(
            options.initial_steps >= 1
                && options.initial_steps <= options.max_steps
                && options.max_steps <= 1024 / edges,
            "Progressive miter exceeds the 1025-section budget",
        )?;
        validate_law(scale, true)?;
        validate_law(twist, false)?;
        let mut lengths = vec![0.];
        let mut tangents = Vec::new();
        for i in 0..edges {
            let delta = sub(points[(i + 1) % points.len()], points[i]);
            let size = norm(delta);
            tangents.push(unit(delta)?);
            lengths.push(lengths.last().unwrap() + size);
        }
        check(
            lengths.last().unwrap().is_finite() && lengths.windows(2).all(|x| x[0] < x[1]),
            "Miter traversal lengths collapse or overflow",
        )?;
        let authored = unit(options.normal)?;
        let transverse = sub(
            authored,
            tangents[0].map(|x| x * dot(authored, tangents[0])),
        );
        check(
            norm(transverse) > 1e-12,
            "Miter normal is parallel to its initial tangent",
        )?;
        let initial = unit(transverse)?;
        let mut normals = vec![initial];
        let mut planes = vec![None; points.len()];
        for i in 1..edges {
            planes[i] = Some(bisector(tangents[i - 1], tangents[i], options.miter_limit)?);
            normals.push(transport(
                normals[i - 1],
                tangents[i - 1],
                tangents[i],
                options.miter_limit,
            )?);
        }
        let correction = if options.closed {
            planes[0] = Some(bisector(
                tangents[edges - 1],
                tangents[0],
                options.miter_limit,
            )?);
            let end = transport(
                normals[edges - 1],
                tangents[edges - 1],
                tangents[0],
                options.miter_limit,
            )?;
            let angle = dot(tangents[0], cross(end, initial)).atan2(dot(end, initial));
            check(
                law(scale, 0.)? == law(scale, 1.)?,
                "Closed miter scale endpoints must agree",
            )?;
            let twist_delta = law(twist, 1.)? - law(twist, 0.)?;
            let turns = twist_delta / std::f64::consts::TAU;
            check(
                turns.is_finite() && (turns - turns.round()).abs() <= 1e-12,
                "Closed miter twist endpoints must differ by whole turns",
            )?;
            let corrected = rotate(end, tangents[0], angle + law(twist, 1.)?);
            check(
                norm(sub(
                    corrected,
                    rotate(initial, tangents[0], law(twist, 0.)?),
                )) < 1e-10,
                "Closed corrected miter frame does not agree at the seam",
            )?;
            angle
        } else {
            0.
        };
        let initial_binormal = cross(tangents[0], initial);
        let offsets = profiles
            .iter()
            .map(|c| {
                c.validate()?;
                check(
                    c.control_points.iter().all(|p| p.len() == 3),
                    "Miter profiles must be 3D",
                )?;
                c.control_points
                    .iter()
                    .map(|p| {
                        let q = sub([p[0], p[1], p[2]], points[0]);
                        check(
                            dot(q, tangents[0]).abs() <= 1e-10 * norm(q),
                            "Miter profiles must lie in the initial normal plane",
                        )?;
                        Ok([dot(q, initial), dot(q, initial_binormal)])
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let transport_certificate = frame_certificate::certify(
            points,
            options.normal,
            options.closed,
            options.miter_limit,
            64,
        )?;
        Ok(Self {
            profiles,
            points,
            scale,
            twist,
            affine_laws: None,
            frame_laws: None,
            orientation_guide: None,
            options,
            tangents,
            normals,
            planes,
            lengths,
            offsets,
            correction,
            transport_certificate,
            next: Some(options.initial_steps),
        })
    }
    pub fn with_affine_laws(mut self,axes:&'a Curve,center:&'a Curve)->Result<Self> {
        for (curve,positive) in [(axes,true),(center,false)] {
            curve.validate()?;
            check(curve.control_points.iter().all(|p|p.len()==3),"Miter affine laws require XYZ controls")?;
            for axis in 0..3 {
                let mut component=curve.clone();
                component.control_points=curve.control_points.iter().map(|p|vec![p[axis],0.,0.]).collect();
                validate_law(&component,positive)?;
            }
            if self.options.closed {
                let [a,b]=curve.domain();
                check(curve.evaluate(a)?.point==curve.evaluate(b)?.point,"Closed miter affine law endpoints must agree")?;
            }
        }
        self.affine_laws=Some((axes,center));
        self.next=Some(self.options.initial_steps);
        Ok(self)
    }
    /// Authored longitudinal/transverse directions replace transported
    /// orientation; the miter planes still belong to the polyline path.
    pub fn with_frame_laws(mut self,longitudinal:&'a Curve,transverse:&'a Curve)->Result<Self> {
        for curve in [longitudinal,transverse] {
            curve.validate()?;
            check(curve.control_points.iter().all(|p|p.len()==3),"Authored miter frame laws require XYZ controls")?;
            for axis in 0..3 {
                let mut component=curve.clone();component.control_points=curve.control_points.iter().map(|p|vec![p[axis],0.,0.]).collect();
                validate_law(&component,false)?;
            }
            if self.options.closed {
                let [a,b]=curve.domain();check(curve.evaluate(a)?.point==curve.evaluate(b)?.point,"Closed authored frame endpoints must agree")?;
            }
        }
        self.frame_laws=Some((longitudinal,transverse));self.correction=0.;
        self.next=Some(self.options.initial_steps);Ok(self)
    }
    pub fn with_orientation_guide(mut self,guide:&'a Curve)->Result<Self> {
        guide.validate()?;check(guide.control_points.iter().all(|p|p.len()==3),"Miter guide requires XYZ controls")?;
        for axis in 0..3 {let mut component=guide.clone();component.control_points=guide.control_points.iter().map(|p|vec![p[axis],0.,0.]).collect();validate_law(&component,false)?;}
        if self.options.closed {let [a,b]=guide.domain();check(guide.evaluate(a)?.point==guide.evaluate(b)?.point,"Closed miter guide endpoints must agree")?;}
        self.orientation_guide=Some(guide);self.correction=0.;self.next=Some(self.options.initial_steps);Ok(self)
    }
    fn frame_source(&self) -> Option<frame_certificate::FrameSource<'a>> {
        match (self.frame_laws,self.orientation_guide) {
            (Some((axis,_)),Some(guide)) => Some(frame_certificate::FrameSource::AuthoredGuide(axis,guide)),
            (Some((axis,normal)),None) => Some(frame_certificate::FrameSource::Authored(axis,normal)),
            (None,Some(guide)) => Some(frame_certificate::FrameSource::Guide(guide)),
            (None,None) => None,
        }
    }
    fn station(&self, edge: usize, f: f64) -> Result<Vec<Curve>> {
        let total = *self.lengths.last().unwrap();
        let traversal =
            (self.lengths[edge] + f * (self.lengths[edge + 1] - self.lengths[edge])) / total;
        let t = self.tangents[edge];
        let angle = law(self.twist, traversal)? + self.correction * traversal;
        let (n,b,frame_axis)=if let Some((longitudinal,transverse))=self.frame_laws {
            let value=|curve:&Curve|->Result<V>{let [a,b]=curve.domain();let p=curve.evaluate(a+(b-a)*traversal)?.point;Ok([p[0],p[1],p[2]])};
            let axis=unit(value(longitudinal)?)?;
            let v=if let Some(guide)=self.orientation_guide {
                let rail=value(guide)?;let next=(edge+1)%self.points.len();
                let path:V=std::array::from_fn(|k|(1.-f)*self.points[edge][k]+f*self.points[next][k]);
                sub(rail,path)
            } else {value(transverse)?};
            let normal=unit(sub(v,axis.map(|x|x*dot(axis,v))))?;
            let n=rotate(normal,axis,law(self.twist,traversal)?);(n,cross(axis,n),axis)
        } else if let Some(guide)=self.orientation_guide {
            let [a,b]=guide.domain();let rail=guide.evaluate(a+(b-a)*traversal)?.point;
            let next=(edge+1)%self.points.len();
            let path:V=std::array::from_fn(|k|(1.-f)*self.points[edge][k]+f*self.points[next][k]);
            let v=sub([rail[0],rail[1],rail[2]],path);
            let normal=unit(sub(v,t.map(|x|x*dot(t,v))))?;
            let n=rotate(normal,t,law(self.twist,traversal)?);(n,cross(t,n),t)
        } else {let n=rotate(self.normals[edge],t,angle);(n,cross(t,n),t)};
        let scale = law(self.scale, traversal)?;
        let (axes,center)=if let Some((axes,center))=self.affine_laws {
            let value=|curve:&Curve|->Result<V>{let [a,b]=curve.domain();let p=curve.evaluate(a+(b-a)*traversal)?.point;Ok([p[0],p[1],p[2]])};
            (value(axes)?,value(center)?)
        } else {([1.;3],[0.;3])};
        let next = (edge + 1) % self.points.len();
        self.profiles
            .iter()
            .zip(&self.offsets)
            .map(|(profile, offsets)| {
                let mut curve = profile.clone();
                for (p, offset) in curve.control_points.iter_mut().zip(offsets) {
                    let q: V =
                        if self.affine_laws.is_some(){std::array::from_fn(|k|
                            (scale*axes[0]*offset[0]+center[0])*n[k]+(scale*axes[1]*offset[1]+center[1])*b[k]+center[2]*frame_axis[k])}
                        else {std::array::from_fn(|k| scale * (offset[0] * n[k] + offset[1] * b[k]))};
                    let shift = |plane: Option<V>| plane.map_or(0., |h| dot(q, h) / dot(t, h));
                    let axial = (1. - f) * shift(self.planes[edge]) + f * shift(self.planes[next]);
                    *p = (0..3)
                        .map(|k| {
                            (1. - f) * self.points[edge][k] + f * self.points[next][k] + q[k]
                                - axial * t[k]
                        })
                        .collect();
                }
                curve.validate()?;
                Ok(curve)
            })
            .collect()
    }
    fn at(&self, index: usize, steps: usize) -> Result<Vec<Curve>> {
        if self.options.closed && index == self.tangents.len() * steps {
            return self.station(0, 0.);
        }
        let edge = (index / steps).min(self.tangents.len() - 1);
        self.station(edge, (index - edge * steps) as f64 / steps as f64)
    }
    pub fn sections_at(&self, steps: usize) -> Result<Vec<Vec<Curve>>> {
        check(
            steps >= 1 && steps <= 1024 / self.tangents.len(),
            "Miter section count exceeds 1025",
        )?;
        let count = self.tangents.len() * steps + 1;
        check(
            self.profiles
                .iter()
                .map(|p| p.control_points.len())
                .sum::<usize>()
                * count
                <= 1_000_000,
            "Miter retained-control budget exceeds one million",
        )?;
        (0..count).map(|i| self.at(i, steps)).collect()
    }
    pub fn preview_at(&self, steps: usize) -> Result<Level> {
        check(
            steps >= self.options.initial_steps && steps <= self.options.max_steps,
            "Miter preview steps are outside the authored progression",
        )?;
        let sections = self.sections_at(steps)?;
        let fine_steps = 4 * steps;
        let stations = self.tangents.len() * fine_steps + 1;
        let mut maximum = 0_f64;
        let mut previous = self.at(0, fine_steps)?;
        for i in 1..stations {
            let fine = self.at(i, fine_steps)?;
            let j = (i / 4).min(sections.len() - 2);
            let f = (i - 4 * j) as f64 / 4.;
            for ((p, a), b) in fine.iter().zip(&sections[j]).zip(&sections[j + 1]) {
                for ((p, a), b) in p
                    .control_points
                    .iter()
                    .zip(&a.control_points)
                    .zip(&b.control_points)
                {
                    maximum = maximum.max(norm(std::array::from_fn(|k| {
                        p[k] - (1. - f) * a[k] - f * b[k]
                    })));
                }
            }
            let tangent = self.tangents[(i - 1) / fine_steps];
            for (a, b) in previous.iter().zip(&fine) {
                for (a, b) in a.control_points.iter().zip(&b.control_points) {
                    check(
                        dot(sub([b[0], b[1], b[2]], [a[0], a[1], a[2]]), tangent) > 0.,
                        "Progressive miter has a sampled backward control advance",
                    )?;
                }
            }
            previous = fine;
        }
        check(maximum.is_finite(), "Miter sampled deviation overflows")?;
        // A uniform probe grid can alias several whole turns. Inspect the
        // local rational twist control hull before promoting a sampled level.
        // This is an admission guard, not a continuous/rounding certificate.
        let [a, b] = self.twist.domain();
        let total = *self.lengths.last().unwrap();
        let mut phase_resolved = true;
        let mut continuous_error_upper = 0_f64;
        for edge in 0..self.tangents.len() {
            for i in 0..steps {
                let traversal = |j: usize| {
                    (self.lengths[edge]
                        + (self.lengths[edge + 1] - self.lengths[edge]) * j as f64 / steps as f64)
                        / total
                };
                let s0 = traversal(i);
                let s1 = traversal(i + 1);
                let (scale, s1_bound, s2_bound, scale_smooth) = scalar_bounds(self.scale, s0, s1)?;
                let (_, t1_bound, t2_bound, twist_smooth) = scalar_bounds(self.twist, s0, s1)?;
                let angular_first = t1_bound + self.correction.abs() * (s1 - s0);
                let tangent = self.tangents[edge];
                let transverse = |plane: Option<V>| {
                    plane.map_or([0.; 3], |h| {
                        let d = dot(tangent, h);
                        std::array::from_fn(|k| h[k] / d - tangent[k])
                    })
                };
                let v0 = transverse(self.planes[edge]);
                let v1 = transverse(self.planes[(edge + 1) % self.points.len()]);
                let shear = norm(v0).max(norm(v1));
                let shear_derivative = norm(sub(v1, v0)) / steps as f64;
                for offsets in &self.offsets {
                    for offset in offsets {
                        let radius = offset[0].hypot(offset[1]);
                        let q1 = radius * (s1_bound + scale * angular_first);
                        let q2 = radius
                            * (s2_bound
                                + 2. * s1_bound * angular_first
                                + scale * (t2_bound + angular_first * angular_first));
                        // Linear interpolation remainder on [0,1]: ||F''||/8.
                        // Across internal law knots use the Lipschitz remainder L/2,
                        // which needs no agreement of one-sided derivatives.
                        let error = if scale_smooth && twist_smooth {
                            ((1. + shear) * q2 + 2. * shear_derivative * q1) / 8.
                        } else {
                            ((1. + shear) * q1 + shear_derivative * radius * scale) / 2.
                        };
                        continuous_error_upper = continuous_error_upper.max(error);
                    }
                }

                let lo = if edge == 0 && i == 0 {
                    a
                } else {
                    a + (b - a) * s0
                };
                let hi = if edge == self.tangents.len() - 1 && i + 1 == steps {
                    b
                } else {
                    a + (b - a) * s1
                };
                check(lo < hi, "Miter phase-guard parameter interval collapsed")?;
                let trimmed = self.twist.trim(lo, hi)?;
                let minimum = trimmed
                    .control_points
                    .iter()
                    .map(|p| p[0])
                    .fold(f64::INFINITY, f64::min);
                let maximum = trimmed
                    .control_points
                    .iter()
                    .map(|p| p[0])
                    .fold(f64::NEG_INFINITY, f64::max);
                if maximum - minimum + self.correction.abs() * (s1 - s0)
                    > std::f64::consts::FRAC_PI_2
                {
                    phase_resolved = false;
                }
            }
        }
        check(
            continuous_error_upper.is_finite(),
            "Miter continuous interpolation estimate overflows",
        )?;
        let certificate = self.certify_level(steps, &sections, 100000, 64)?;
        let profile_regularity = self.certify_profile_regularity(10000)?;
        let wall_regularity = if profile_regularity.spanwise_regular
            && phase_resolved
            && certificate
                .error_upper
                .is_some_and(|upper| upper <= self.options.max_deviation)
        {
            Some(self.certify_wall_regularity(&sections, 10000)?)
        } else {
            None
        };

        Ok(Level {
            report: Report {
                accepted: profile_regularity.spanwise_regular
                    && wall_regularity.as_ref().is_some_and(|r| r.spanwise_regular)
                    && phase_resolved
                    && self.transport_certificate.status == scalar_certificate::Status::Certified
                    && maximum <= self.options.max_deviation
                    && certificate
                        .error_upper
                        .is_some_and(|upper| upper <= self.options.max_deviation),
                phase_resolved,
                frame_transport_certified: self.transport_certificate.status
                    == scalar_certificate::Status::Certified,
                frame_transport_reason: self.transport_certificate.reason,
                steps,
                sections: sections.len(),
                stations,
                sampled_control_deviation: maximum,
                continuous_error_upper:if self.affine_laws.is_some() || self.frame_laws.is_some() || self.orientation_guide.is_some(){certificate.error_upper.unwrap_or(f64::MAX)}else{continuous_error_upper},
                certified_error_upper: certificate.error_upper,
                affine_laws_applied: self.affine_laws.is_some(),
                authored_frames_applied: self.frame_laws.is_some(),
                orientation_guide_applied: self.orientation_guide.is_some(),
                endpoint_contour_error_upper: certificate.endpoint_contour_error_upper,
                error_certificate_cells: certificate.cells,
                error_certificate_reason: certificate.reason,
                profile_regularity_certified: profile_regularity.spanwise_regular,
                wall_regularity_certified: wall_regularity.as_ref().map(|r| r.spanwise_regular),
                regularity_cells: profile_regularity.cells
                    + wall_regularity.as_ref().map_or(0, |r| r.cells),
                unresolved_wall_patches: wall_regularity
                    .map_or_else(Vec::new, |r| r.unresolved_patches),
                budget: self.options.max_deviation,
                closed_path: self.options.closed,
                holonomy_correction: self.correction,
            },
            sections,
        })
    }
}
impl Iterator for Sweep<'_> {
    type Item = Result<Level>;
    fn next(&mut self) -> Option<Self::Item> {
        let steps = self.next.take()?;
        let level = self.preview_at(steps);
        if let Ok(level) = &level {
            if !level.report.accepted && steps < self.options.max_steps {
                self.next = Some((2 * steps).min(self.options.max_steps));
            }
        }
        Some(level)
    }
}
pub fn approximate(
    profiles: &[Curve],
    points: &[V],
    scale: &Curve,
    twist: &Curve,
    options: Options,
) -> Result<Approximation> {
    let mut levels = Vec::new();
    let mut sections = None;
    for level in Sweep::new(profiles, points, scale, twist, options)? {
        let level = level?;
        if level.report.accepted {
            sections = Some(level.sections);
        }
        levels.push(level.report);
    }
    Ok(Approximation { sections, levels })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guide_affine_generation_certifies_actual_rail_orientation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let vector=|a:V,b:V,domain:[f64;2]|Curve{degree:1,knots:vec![domain[0],domain[0],domain[1],domain[1]],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.;2],periodic:false};
        let longitudinal=vector([0.,0.,1.],[0.,0.,1.],[2.,5.]);
        let transverse=vector([1.,0.,0.],[1.,1.,0.],[11.,13.]);
        let axes=vector([1.,1.,1.],[2.,1.,1.],[17.,19.]);
        let center=Curve{degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,1.,0.25]],weights:vec![1.;3],periodic:false};
        let guide=vector([1.,0.,0.],[1.,1.,10.],[31.,41.]);
        let options=Options{normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:64,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let level=sweep.preview_at(16).unwrap();assert!(level.report.accepted,"{:?}",level.report);
        assert!(level.report.affine_laws_applied && level.report.orientation_guide_applied && !level.report.authored_frames_applied);
        let reverse_order=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap().with_orientation_guide(&guide).unwrap().sections_at(16).unwrap();
        assert_eq!(level.sections.last().unwrap()[0].control_points,reverse_order.last().unwrap()[0].control_points);
        for framed in [
            Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_orientation_guide(&guide).unwrap().with_affine_laws(&axes,&center).unwrap(),
            Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_affine_laws(&axes,&center).unwrap(),
        ] {
            let combined=framed.preview_at(16).unwrap();
            assert!(combined.report.accepted && combined.report.authored_frames_applied && combined.report.orientation_guide_applied && combined.report.affine_laws_applied);
            for (a,b) in combined.sections.iter().zip(&level.sections) {for (a,b) in a.iter().zip(b) {assert_eq!(a.control_points,b.control_points);}}
        }
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {for local in [0.,0.25,0.5,0.75,1.] {let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
            for u in [0.,0.3,0.7,1.] {let r=0.1+0.1*u;let ideal=[(r*(1.+f)-f*f*f)/h,(r*(1.+f)*f+f*f)/h,10.*f+0.25*f];
                let a=level.sections[i][0].evaluate(u).unwrap().point;let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);assert!(norm(sub(ideal,retained))<=bound);
            }
        }}
        assert!(sweep.certify_level(16,&level.sections,10000,10).unwrap().error_upper.is_none());
        let mut altered=level.sections.clone();altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        assert!(sweep.certify_level(16,&altered,10000,64).unwrap().endpoint_contour_error_upper.unwrap()[1]>=0.125);
    }
    #[test]
    fn combined_authored_frame_affine_laws_certify_actual_interpolation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let vector=|a:V,b:V,domain:[f64;2]|Curve{degree:1,knots:vec![domain[0],domain[0],domain[1],domain[1]],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.;2],periodic:false};
        let longitudinal=vector([0.,0.,1.],[0.,0.,1.],[2.,5.]);
        let transverse=vector([1.,0.,0.],[1.,1.,0.],[11.,13.]);
        let axes=vector([1.,1.,1.],[2.,1.,1.],[17.,19.]);
        let center=Curve{degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,1.,0.25]],weights:vec![1.;3],periodic:false};
        let options=Options{normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:64,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let level=sweep.preview_at(16).unwrap();assert!(level.report.accepted,"{:?}",level.report);
        assert!(level.report.affine_laws_applied && level.report.authored_frames_applied);
        let reverse_order=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().sections_at(16).unwrap();
        assert_eq!(level.sections.last().unwrap()[0].control_points,reverse_order.last().unwrap()[0].control_points);
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {for local in [0.,0.25,0.5,0.75,1.] {let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
            for u in [0.,0.3,0.7,1.] {let r=0.1+0.1*u;let ideal=[(r*(1.+f)-f*f*f)/h,(r*(1.+f)*f+f*f)/h,10.*f+0.25*f];
                let a=level.sections[i][0].evaluate(u).unwrap().point;let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);assert!(norm(sub(ideal,retained))<=bound);
            }
        }}
        assert!(sweep.certify_level(16,&level.sections,10000,13).unwrap().error_upper.is_none());
        let mut altered=level.sections.clone();altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        assert!(sweep.certify_level(16,&altered,10000,64).unwrap().endpoint_contour_error_upper.unwrap()[1]>=0.125);
    }
    #[test]
    fn authored_frame_generation_refines_and_certifies_original_profile_interpolation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];
        let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let longitudinal=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![0.,0.,1.];2],weights:vec![1.;2],periodic:false};
        let transverse=Curve {degree:1,knots:vec![7.,7.,9.,9.],control_points:vec![vec![1.,0.,0.],vec![1.,1.,0.]],weights:vec![1.;2],periodic:false};
        let options=Options {normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:16,max_deviation:0.001};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let local=sweep.transport_certificate.profile_local_enclosure([0.1,0.,0.]).unwrap().unwrap();
        sweep.transport_certificate.interpolation_upper_authored_frame(0,[0.,1./16.],local,&longitudinal,&transverse,&scale,&twist,64).unwrap().unwrap();
        sweep.transport_certificate.station_local_authored_frame(0,[0.,0.],local,&longitudinal,&transverse,&scale,&twist,64).unwrap().unwrap();
        let level=sweep.preview_at(16).unwrap();
        assert!(level.report.accepted && level.report.authored_frames_applied,"{:?}",level.report);
        assert!(level.report.wall_regularity_certified.unwrap());
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {
            for local in [0.,0.25,0.5,0.75,1.] {
                let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
                for u in [0.,0.3,0.7,1.] {
                    let radius=0.1+0.1*u;let ideal=[radius/h,radius*f/h,10.*f];
                    let a=level.sections[i][0].evaluate(u).unwrap().point;
                    let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                    let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);
                    assert!(norm(sub(ideal,retained))<=bound);
                }
            }
        }
        let mut damaged=level.sections.clone();damaged.last_mut().unwrap()[0].control_points[0][0]+=0.125;
        let report=sweep.certify_level(16,&damaged,10000,64).unwrap();
        assert!(report.error_upper.unwrap()>=0.125);
        assert!(report.endpoint_contour_error_upper.unwrap()[1]>=0.125);
        assert!(sweep.certify_level(16,&level.sections,10000,7).unwrap().error_upper.is_none());
        let mut singular=longitudinal.clone();singular.control_points=vec![vec![0.;3];2];
        let singular_sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&singular,&transverse).unwrap();
        assert!(singular_sweep.certify_level(16,&level.sections,10000,64).unwrap().error_upper.is_none());
    }
    #[test]
    fn affine_closed_miter_checks_law_closure_and_last_retained_interval(){
        let profiles=[crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap()];
        let points=[[0.,0.,0.],[10.,0.,0.],[10.,10.,0.],[0.,10.,0.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let axes=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![1.,1.,1.],vec![2.,1.,1.],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let center=Curve {degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.2,0.,0.],vec![0.,0.,0.]],weights:vec![1.;3],periodic:false};
        let opts=Options {normal:[0.,0.,1.],closed:true,miter_limit:2.,initial_steps:1,max_steps:16,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let level=sweep.preview_at(8).unwrap();
        assert!(level.report.accepted && level.report.affine_laws_applied);
        assert_eq!(level.sections[0][0].control_points,level.sections.last().unwrap()[0].control_points);
        let mut altered=level.sections.clone();
        altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        let proof=sweep.certify_level(8,&altered,10000,64).unwrap();
        assert!(proof.error_upper.unwrap()>=0.125);
        assert!(proof.endpoint_contour_error_upper.unwrap()[1]>=0.125);
        for component in 0..2 {
            let mut bad_axes=axes.clone();let mut bad_center=center.clone();
            if component==0 {bad_axes.control_points[2][0]=1.25;}else{bad_center.control_points[2][0]=0.125;}
            assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&bad_axes,&bad_center).is_err());
        }
        let discontinuous=Curve {degree:1,knots:vec![0.,0.,0.5,0.5,1.,1.],control_points:vec![vec![0.;3];4],weights:vec![1.;4],periodic:false};
        assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&discontinuous).is_err());
    }
    #[test]
    fn affine_generator_retains_laws_and_certifies_entire_interpolated_profile(){
        let profiles=[crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap()];
        let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let axes=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![1.,1.,1.],vec![2.,1.,1.]],weights:vec![1.;2],periodic:false};
        let center=Curve {degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,1.,0.]],weights:vec![1.;3],periodic:false};
        let mut opts=options(false);opts.normal=[1.,0.,0.];opts.max_deviation=0.01;
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let level=sweep.preview_at(8).unwrap();
        assert!(level.report.affine_laws_applied && level.report.accepted);
        let bound=level.report.certified_error_upper.unwrap();
        assert!(bound<opts.max_deviation);
        assert_eq!(level.report.continuous_error_upper,bound);
        for (i,pair) in level.sections.windows(2).enumerate(){
            for j in 0..=10 {let local=j as f64/10.;let f=(i as f64+local)/8.;
                for u in [0.,0.25,0.5,0.75,1.] {
                    let a=pair[0][0].evaluate(u).unwrap().point;let b=pair[1][0].evaluate(u).unwrap().point;
                    let stored:V=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);
                    let ideal=[(1.+f)*(1.+u),f*f,10.*f];
                    assert!(norm(sub(stored,ideal))<=bound);
                }
            }
        }
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let mut invalid=axes.clone();invalid.control_points[0][0]=0.;
        assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&invalid,&center).is_err());
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_progressive_miter_level","profiles":profiles,"points":points,
                "scale":scale,"twist":twist,"axis_scale":axes,"center_law":center,"normal":opts.normal,
                "closed":false,"miter_limit":opts.miter_limit,"initial_steps":opts.initial_steps,"max_steps":opts.max_steps,
                "max_deviation":opts.max_deviation,"preview_steps":8});
            let report=crate::transport::dispatch(request).unwrap();
            assert_eq!(report["report"]["accepted"],true);
            assert_eq!(report["report"]["affineLawsApplied"],true);
            assert_eq!(report["report"]["continuousErrorMethod"],"interval-affine-law-interpolation");
            assert_eq!(report["report"]["continuousBound"],false);
        }
    }
    #[test]
    fn local_phase_hulls_prevent_whole_turn_probe_aliasing() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 4. * std::f64::consts::TAU);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 256;
        opts.max_deviation = 1e-3;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.sampled_control_deviation < 1e-12);
        assert!(!level.report.phase_resolved && !level.report.accepted);
        let result = approximate(&profiles, &points, &scale, &twist, opts).unwrap();
        assert!(result.levels.last().unwrap().accepted);
        assert!(result.levels.last().unwrap().steps >= 16);
    }
    #[test]
    fn real_arithmetic_bound_covers_nondyadic_rational_law_interpolation() {
        let profiles = [crate::paths::bezier(
            vec![vec![0.1, 0., 0.], vec![0.2, 0.1, 0.], vec![0.3, 0., 0.]],
            Some(vec![1., 0.7, 2.]),
        )
        .unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]];
        let scale = crate::paths::bezier(
            vec![vec![1., 0., 0.], vec![1.7, 0., 0.], vec![1.1, 0., 0.]],
            Some(vec![1., 0.8, 1.2]),
        )
        .unwrap();
        let twist = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![1.2, 0., 0.], vec![3., 0., 0.]],
            Some(vec![1., 1.1, 0.9]),
        )
        .unwrap();
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 64;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(8).unwrap();
        for edge in 0..2 {
            for interval in 0..8 {
                for probe in 1..127 {
                    let f = probe as f64 / 127.;
                    let actual = sweep.station(edge, (interval as f64 + f) / 8.).unwrap();
                    let a = &level.sections[edge * 8 + interval][0];
                    let b = &level.sections[edge * 8 + interval + 1][0];
                    for ((p, a), b) in actual[0]
                        .control_points
                        .iter()
                        .zip(&a.control_points)
                        .zip(&b.control_points)
                    {
                        let error =
                            norm(std::array::from_fn(|k| p[k] - (1. - f) * a[k] - f * b[k]));
                        assert!(
                            error <= level.report.continuous_error_upper + 1e-12,
                            "{error} exceeds {}",
                            level.report.continuous_error_upper
                        );
                    }
                }
            }
        }
        assert!(level.report.continuous_error_upper >= level.report.sampled_control_deviation);
    }
    #[test]
    fn discontinuous_laws_are_refused_and_internal_corners_use_lipschitz_bound() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 0.37, 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.continuous_error_upper >= 0.2);
        let certified = sweep.certify_level(1, &level.sections, 10, 4).unwrap();
        assert_eq!(certified.status, scalar_certificate::Status::Certified);
        assert!(certified.error_upper.unwrap() >= 0.2);
        assert!(
            sweep
                .certify_level(1, &level.sections, 10, 1)
                .unwrap()
                .error_upper
                .is_none()
        );
        for f in [0.13, 0.37, 0.63, 0.91] {
            let ideal = sweep.station(0, f).unwrap();
            for k in 0..2 {
                let a = &level.sections[0][0].control_points[k];
                let b = &level.sections[1][0].control_points[k];
                let p = &ideal[0].control_points[k];
                assert!(
                    norm(std::array::from_fn(|j| p[j] - ((1. - f) * a[j] + f * b[j])))
                        <= certified.error_upper.unwrap()
                );
            }
        }
        assert!(!level.report.accepted);
        let discontinuous = Curve {
            degree: 1,
            knots: vec![0., 0., 0.37, 0.37, 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![2., 0., 0.],
                vec![3., 0., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(Sweep::new(&profiles, &points, &discontinuous, &twist, opts).is_err());
    }
    #[test]
    fn unresolved_transport_limit_cannot_promote_a_zero_error_level() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 5.], [0., 0., 10.]];
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.miter_limit = 1.;
        opts.max_steps = 2;
        let approximation =
            approximate(&profiles, &points, &scalar(1., 1.), &scalar(0., 0.), opts).unwrap();
        assert!(approximation.sections.is_none());
        let r = approximation.levels.last().unwrap();
        assert!(r.sampled_control_deviation < 1e-12 && r.phase_resolved);
        assert!(!r.frame_transport_certified && !r.accepted);
        assert_eq!(
            r.frame_transport_reason,
            Some("transport-enclosure-unresolved")
        );
    }
    fn scalar(a: f64, b: f64) -> Curve {
        crate::paths::bezier(vec![vec![a, 0., 0.], vec![b, 0., 0.]], None).unwrap()
    }
    fn options(closed: bool) -> Options {
        Options {
            normal: [0., 0., 1.],
            closed,
            miter_limit: 4.,
            initial_steps: 1,
            max_steps: 64,
            max_deviation: 1e-4,
        }
    }
    #[test]
    fn constant_laws_match_existing_extrusion_miter_sections() {
        let profile = crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap();
        let profiles = [profile];
        let points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(false)).unwrap();
        let expected = crate::paths::miter_sections(&profiles, &points, [0., 0., 1.], 4.).unwrap();
        let actual = sweep.sections_at(1).unwrap();
        for (a, b) in actual.iter().zip(expected) {
            for (a, b) in a.iter().zip(b) {
                for (a, b) in a.control_points.iter().zip(b.control_points) {
                    assert!(norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]])) < 1e-12);
                }
            }
        }
        assert!(sweep.preview_at(1).unwrap().report.accepted);
    }
    #[test]
    fn distributed_twist_corrects_skew_loop_holonomy_and_shares_all_corners() {
        let profiles = [crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap()];
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        assert!(crate::paths::closed_miter_sections(&profiles, &points, [0., 0., 1.], 4.).is_err());
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(true)).unwrap();
        assert!(sweep.correction.abs() > 1e-3);
        let angle = sweep.transport_certificate.holonomy_angle.unwrap();
        assert!(angle[0] <= sweep.correction && sweep.correction <= angle[1]);
        for edge in 0..sweep.tangents.len() {
            for f in [0., 0.37, 1.] {
                let (n, b) = sweep
                    .transport_certificate
                    .rotated_frame(edge, [f, f], [0., 0.])
                    .unwrap()
                    .unwrap();
                let total = *sweep.lengths.last().unwrap();
                let traversal = (sweep.lengths[edge]
                    + f * (sweep.lengths[edge + 1] - sweep.lengths[edge]))
                    / total;
                let normal = rotate(
                    sweep.normals[edge],
                    sweep.tangents[edge],
                    sweep.correction * traversal,
                );
                let binormal = cross(sweep.tangents[edge], normal);
                let station = sweep.station(edge, f).unwrap();
                for (profile, section) in profiles.iter().zip(&station) {
                    for (authored, stored) in
                        profile.control_points.iter().zip(&section.control_points)
                    {
                        let bound = sweep
                            .transport_certificate
                            .station_point(
                                edge,
                                [f, f],
                                [authored[0], authored[1], authored[2]],
                                [1., 1.],
                                [0., 0.],
                            )
                            .unwrap()
                            .unwrap();
                        for k in 0..3 {
                            assert!(bound[k][0] <= stored[k] && stored[k] <= bound[k][1]);
                        }
                    }
                }
                let sheared = sweep
                    .transport_certificate
                    .miter_offset(edge, [f, f], n)
                    .unwrap()
                    .unwrap();
                let t = sweep.tangents[edge];
                let shift = |h: Option<V>| h.map_or(0., |h| dot(normal, h) / dot(t, h));
                let axial = (1. - f) * shift(sweep.planes[edge])
                    + f * shift(sweep.planes[(edge + 1) % points.len()]);
                for k in 0..3 {
                    let expected = normal[k] - axial * t[k];
                    assert!(sheared[k][0] <= expected && expected <= sheared[k][1]);
                }
                for k in 0..3 {
                    assert!(n[k][0] <= normal[k] && normal[k] <= n[k][1]);
                    assert!(b[k][0] <= binormal[k] && binormal[k] <= b[k][1]);
                }
            }
        }
        for i in 0..points.len() {
            let a = sweep.station(i, 1.).unwrap();
            let b = sweep.station((i + 1) % points.len(), 0.).unwrap();
            for (a, b) in a.iter().zip(b) {
                for (a, b) in a.control_points.iter().zip(b.control_points) {
                    assert!(norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]])) < 1e-10);
                }
            }
        }
        let result = approximate(&profiles, &points, &scale, &twist, options(true)).unwrap();
        assert!(
            result.levels.last().unwrap().accepted,
            "{:?}",
            result.levels
        );
        assert!(result.levels.len() > 1);
        let sections = result.sections.unwrap();
        assert_eq!(sections.first(), sections.last());
    }
    #[test]
    fn scalar_laws_follow_independent_scaled_full_turn_and_fail_closed_mismatch() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 2.);
        let twist = scalar(0., std::f64::consts::TAU);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 256;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        for f in [0., 0.17, 0.5, 0.93, 1.] {
            let p = &sweep.station(0, f).unwrap()[0].control_points[1];
            let angle = std::f64::consts::TAU * f;
            assert!((p[0] - 0.2 * (1. + f) * angle.cos()).abs() < 1e-12);
            assert!((p[1] - 0.2 * (1. + f) * angle.sin()).abs() < 1e-12);
            assert!((p[2] - 10. * f).abs() < 1e-12);
        }
        assert!(
            approximate(&profiles, &points, &scale, &twist, opts)
                .unwrap()
                .sections
                .is_some()
        );
        let closed_profiles = [crate::primitives::line([0., 0.1, 0.2], [0., 0.2, 0.3]).unwrap()];
        let closed_points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        assert!(
            Sweep::new(
                &closed_profiles,
                &closed_points,
                &scale,
                &twist,
                options(true)
            )
            .is_err()
        );
        assert!(
            Sweep::new(
                &closed_profiles,
                &closed_points,
                &scalar(1., 1.),
                &scalar(0., 1.),
                options(true)
            )
            .is_err()
        );
    }
    #[test]
    fn progression_keeps_unaccepted_preview_and_can_resume_without_promoting_it() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 2.);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 1;
        let result = approximate(&profiles, &points, &scale, &twist, opts).unwrap();
        assert!(result.sections.is_none());
        assert!(!result.levels[0].accepted);
        opts.max_steps = 256;
        let mut sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let preview = sweep.next().unwrap().unwrap();
        assert!(!preview.report.accepted);
        assert_eq!(preview.sections.len(), 2);
        let tail = sweep.collect::<Result<Vec<_>>>().unwrap();
        assert!(tail.last().unwrap().report.accepted);
        assert!(
            tail.windows(2)
                .all(|levels| levels[1].report.steps == 2 * levels[0].report.steps)
        );
    }
    #[test]
    fn retained_level_certificate_counts_stored_error_and_discards_partial_budget() {
        let profiles = [crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        assert!(
            sweep
                .certify_profile_regularity(1)
                .unwrap()
                .spanwise_regular
        );
        let regularity = sweep.certify_profile_regularity(0).unwrap();
        assert!(!regularity.spanwise_regular && regularity.unresolved_profiles == vec![0]);
        let mut sections = sweep.sections_at(2).unwrap();
        let separation = sweep
            .inspect_wall_separation(&sweep.sections_at(3).unwrap(), 0., 1e-6, 100, 63)
            .unwrap();
        assert!(!separation.all_pairs_separated);
        assert_eq!(separation.separated_pairs, 1);
        assert_eq!(separation.boundary_only_pairs.len(), 2);
        assert!(separation.all_pairs_compatible && separation.unresolved.is_empty());
        let geometry = sweep
            .inspect_wall_geometry(&sweep.sections_at(3).unwrap(), 0., 1e-6, 100, 100, 63)
            .unwrap();
        assert!(geometry.charts_and_pairs_certified, "{geometry:?}");
        assert!(geometry.declared_boundaries_c0);
        assert_eq!(geometry.c0_boundaries, vec![[0, 1], [1, 2]]);
        let geometry = sweep
            .inspect_wall_geometry(&sweep.sections_at(3).unwrap(), 0., 1e-6, 0, 100, 63)
            .unwrap();
        assert!(!geometry.charts_and_pairs_certified);
        assert_eq!(geometry.injectivity_cells, 0);
        assert_eq!(geometry.unresolved_charts, vec![0, 1, 2]);
        assert!(geometry.pairs.all_pairs_compatible);

        let wall = sweep.certify_wall_regularity(&sections, 2).unwrap();
        assert!(wall.spanwise_regular && wall.cells == 2);
        let exhausted = sweep.certify_wall_regularity(&sections, 1).unwrap();
        assert!(!exhausted.spanwise_regular && exhausted.unresolved_patches == vec![[1, 0]]);
        let mut collapsed = sections.clone();
        collapsed[1] = collapsed[0].clone();
        assert!(
            !sweep
                .certify_wall_regularity(&collapsed, 31)
                .unwrap()
                .spanwise_regular
        );

        let c = sweep.certify_level(2, &sections, 10, 4).unwrap();
        assert_eq!(c.status, scalar_certificate::Status::Certified);
        assert!(c.error_upper.unwrap() < 1e-10);
        assert!(c.endpoint_contour_error_upper.unwrap().iter().all(|v|*v>=0. && *v<1e-10));
        let mut altered_end=sections.clone();
        altered_end.last_mut().unwrap()[0].control_points[0][0]+=0.125;
        let endpoints=sweep.certify_level(2,&altered_end,10,4).unwrap().endpoint_contour_error_upper.unwrap();
        assert!(endpoints[0]<1e-10 && endpoints[1]>=0.125);
        let u = sweep.certify_level(2, &sections, 1, 4).unwrap();
        assert_eq!(u.status, scalar_certificate::Status::Unresolved);
        assert!(u.error_upper.is_none());
        assert!(u.endpoint_contour_error_upper.is_none());
        sections[1][0].control_points[0][0] += 0.1;
        assert!(
            sweep
                .certify_level(2, &sections, 10, 4)
                .unwrap()
                .error_upper
                .unwrap()
                >= 0.1
        );
        sections[1][0].weights[0] = 2.;
        assert!(sweep.certify_level(2, &sections, 10, 4).is_err());
    }
    #[test]
    fn wall_regularity_refuses_valid_sections_with_changed_periodicity() {
        let profiles = [Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.], vec![0., 1., 0.], vec![-1., 0., 0.],
                vec![0., -1., 0.], vec![1., 0., 0.], vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        }];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        let mut sections = sweep.sections_at(1).unwrap();
        assert!(sweep.certify_wall_regularity(&sections, 10000).unwrap().spanwise_regular);
        sections[1][0].periodic = false;
        sections[1][0].validate().unwrap();
        assert!(sweep.certify_wall_regularity(&sections, 10000).is_err());
    }
    #[test]
    fn certified_closed_rational_levels_cover_profile_interior_and_cyclic_ownership() {
        let profiles = [crate::paths::bezier(
            vec![vec![0., 0.1, 0.2], vec![0., 0.3, 0.4], vec![0., 0.2, 0.1]],
            Some(vec![1., 0.7, 1.3]),
        )
        .unwrap()];
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        let mut scale = crate::paths::bezier(
            vec![vec![1., 0., 0.], vec![1.5, 0., 0.], vec![1., 0., 0.]],
            Some(vec![1., 0.8, 1.]),
        )
        .unwrap();
        scale.knots.iter_mut().for_each(|k| *k = 3. + 4. * (*k));
        let mut twist = scalar(0., std::f64::consts::TAU);
        twist.knots.iter_mut().for_each(|k| *k = -2. + 4. * (*k));
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(true)).unwrap();
        let mut previous = f64::INFINITY;
        for steps in [8, 16] {
            let sections = sweep.sections_at(steps).unwrap();
            let certificate = sweep.certify_level(steps, &sections, 10000, 16).unwrap();
            let regularity = sweep.certify_wall_regularity(&sections, 10000).unwrap();
            if steps == 8 {
                let geometry = sweep
                    .inspect_wall_geometry(&sections, 0., 1e-6, 0, 10000, 256)
                    .unwrap();
                assert!(!geometry.charts_and_pairs_certified);
                assert_eq!(geometry.unresolved_charts.len(), sections.len() - 1);
                assert_eq!(geometry.injectivity_cells, 0);
                assert!(geometry.declared_boundaries_c0);
                assert_eq!(geometry.c0_boundaries.len(), sections.len() - 1);
                let separation = sweep
                    .inspect_wall_separation(&sections, 0., 1e-6, 10000, 256)
                    .unwrap();
                assert!(!separation.all_pairs_separated);
                assert!(
                    separation
                        .unresolved
                        .iter()
                        .any(|p| p.patches == [0, sections.len() - 2]
                            && p.reason == "shared-boundary-interior-separation-unproved")
                );
            }

            assert!(regularity.spanwise_regular, "{regularity:?}");
            assert!(regularity.cells <= 10000);

            assert_eq!(
                certificate.status,
                scalar_certificate::Status::Certified,
                "{certificate:?}"
            );
            let upper = certificate.error_upper.unwrap();
            assert!(upper < previous && upper < 0.1, "{upper}");
            previous = upper;
            for edge in 0..points.len() {
                for i in 0..steps {
                    for local in [0.13, 0.37, 0.81] {
                        let f = (i as f64 + local) / steps as f64;
                        let ideal = sweep.station(edge, f).unwrap();
                        let index = edge * steps + i;
                        for u in [0.17, 0.53, 0.91] {
                            let a = sections[index][0].evaluate(u).unwrap().point;
                            let b = sections[index + 1][0].evaluate(u).unwrap().point;
                            let exact = ideal[0].evaluate(u).unwrap().point;
                            let error = norm(std::array::from_fn(|k| {
                                exact[k] - ((1. - local) * a[k] + local * b[k])
                            }));
                            assert!(error <= upper, "{error}>{upper} edge{edge} interval{i}");
                        }
                    }
                }
            }
            let mut altered = sections.clone();
            altered.last_mut().unwrap()[0].control_points[0][0] += 0.25;
            assert!(
                sweep
                    .inspect_wall_separation(&altered, 0., 1e-6, 10000, 256)
                    .is_err()
            );
            assert!(
                sweep
                    .certify_level(steps, &altered, 10000, 16)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.25
            );
        }
    }
    #[test]
    fn admission_requires_certified_rounding_bound_even_when_samples_match() {
        let profiles = [crate::primitives::line([1e8 + 1., 0., 0.], [1e8 + 2., 0., 0.]).unwrap()];
        let points = [[1e8, 0., 0.], [1e8, 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        o.max_deviation = 1e-12;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.sampled_control_deviation < 1e-12);
        assert!(level.report.certified_error_upper.unwrap() > o.max_deviation);
        assert!(!level.report.accepted);
        o.max_deviation = 1e-4;
        let level = Sweep::new(&profiles, &points, &scale, &twist, o)
            .unwrap()
            .preview_at(1)
            .unwrap();
        assert!(level.report.accepted);
        assert!(level.report.certified_error_upper.unwrap() <= o.max_deviation);
    }
    #[test]
    fn unproved_profile_regularity_refuses_even_a_certified_zero_error_level() {
        let profile = crate::paths::bezier(vec![vec![1., 0., 0.], vec![1., 0., 0.]], None).unwrap();
        let profiles = [profile];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let level = Sweep::new(&profiles, &points, &scale, &twist, o)
            .unwrap()
            .preview_at(1)
            .unwrap();
        assert!(level.report.certified_error_upper.unwrap() < o.max_deviation);
        assert!(!level.report.profile_regularity_certified && !level.report.accepted);
        assert_eq!(level.report.wall_regularity_certified, None);
    }
}
