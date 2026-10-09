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
mod serialization;
use crate::{Result, check, core::vec3_ext::norm, curve::Curve, foundation::guards::{Budget, require_finite_point}};
use law::{law, scalar_bounds, validate_law};
use math_core::{cross, dot, sub};
type V = [f64; 3];
fn unit(v: V) -> Result<V> {
    crate::core::vec3_ext::unit(v, "Miter direction must be finite and nonzero")
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
        require_finite_point(&options.normal, "normal")?;
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
        // Unified guard as a backstop over the refinement-station budget.
        let mut guard = Budget::with_iterations(stations + 1)?.guard("miter_preview");
        for i in 1..stations {
            guard.tick()?;
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
    // Unified guard over the adaptive level progression (doubling toward
    // `max_steps`); the iterator's own stop conditions stay authoritative.
    let mut guard = Budget::with_iterations(64)?.guard("progressive_miter");
    for level in Sweep::new(profiles, points, scale, twist, options)? {
        guard.tick()?;
        let level = level?;
        if level.report.accepted {
            sections = Some(level.sections);
        }
        levels.push(level.report);
    }
    Ok(Approximation { sections, levels })
}

#[cfg(test)]
#[path = "tests/progressive_miter.rs"]
mod tests;
