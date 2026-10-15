use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    RotationMinimizing,
    /// Preserve the initial world orientation; twist uses the initial tangent.
    Fixed,
    /// Project the authored normal perpendicular to each path tangent.
    FixedNormal,
    /// Principal normal. Zero curvature and undefined second derivatives fail.
    Frenet,
    /// Sign-continuous sampled principal normal; RMF transport at zero curvature.
    CorrectedFrenet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    Parameter,
    /// Certified inverse arc-length residual, in model length units.
    ArcLength {
        tolerance: f64,
        max_cells: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub normal: V,
    pub orientation: Orientation,
    pub spacing: Spacing,
    pub initial_sections: usize,
    pub max_sections: usize,
    pub max_deviation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelReport {
    pub accepted: bool,
    pub sections: usize,
    pub stations: usize,
    pub sampled_control_deviation: f64,
    pub budget: f64,
    pub closed_path: bool,
    pub length_residual_upper: Option<f64>,
    pub continuous_error_upper: Option<f64>,
    /// Bounds both original end-section contours, before decomposition/correction.
    pub original_section_endpoint_error_upper: Option<f64>,
    /// Retained end contours, including decomposition, before correction/caps.
    pub endpoint_contour_error_upper: Option<[f64;2]>,
    /// Maximum among certified profiles, even when the complete union is unresolved.
    /// This controls conservative admission but is not a bound for every profile.
    pub known_profile_error_upper: Option<f64>,
    pub error_certificate_cells: usize,
    pub decomposition_products: usize,
    pub error_certificate_reason: Option<&'static str>,
    pub continuous_bound: bool,
}

#[derive(Clone, Debug)]
pub struct Level {
    /// Preview is always available, even when this level misses the budget.
    pub patches: Vec<Surface>,
    pub report: LevelReport,
}

#[derive(Clone, Debug)]
pub struct Approximation {
    /// Only an accepted final level is promoted to a construction result.
    pub patches: Option<Vec<Surface>>,
    pub levels: Vec<LevelReport>,
}

/// Caller-controlled progression: consume one level for preview, stop or resume.
/// Scale is positive [value,0,0]; twist is signed [radians,0,0]. Each law domain
/// independently maps to normalized traversal (parameter or arc length).
pub struct Sweep<'a> {
    pub(super) profile: &'a Curve,
    pub(super) path: &'a Curve,
    pub(super) scale: &'a Curve,
    pub(super) twist: &'a Curve,
    pub(super) affine_laws: Option<(&'a Curve, &'a Curve)>,
    pub(super) frame_laws: Option<(&'a Curve, &'a Curve)>,
    pub(super) orientation_guide: Option<&'a Curve>,
    pub(super) contact_point: Option<V>,
    /// Original anchor ownership, including the reference contour in MultiSweep.
    pub(super) contact_source: Option<(&'a Curve, f64)>,
    pub(super) spatial_rmf_error_limits: Option<[usize;3]>,
    pub(super) options: Options,
    pub(super) next_sections: Option<usize>,
}

pub(super) fn unit(a: V) -> Result<V> {
    let largest = a.iter().fold(0_f64, |m, x| m.max(x.abs()));
    check(
        largest.is_finite() && largest > 0.,
        "Sweep direction must be finite and nonzero",
    )?;
    let scaled = a.map(|x| x / largest);
    let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
    Ok(scaled.map(|x| x / length))
}
pub(super) fn project(normal: V, tangent: V) -> Result<V> {
    let n = unit(normal)?;
    let transverse = sub(n, tangent.map(|x| x * dot(n, tangent)));
    check(
        norm(transverse) > 1e-12,
        "Sweep normal is parallel or numerically inseparable from tangent",
    )?;
    unit(transverse)
}
pub(super) fn reflect(a: V, direction: V) -> Result<V> {
    let n = unit(direction)?;
    Ok(sub(a, n.map(|x| 2. * dot(a, n) * x)))
}
pub(super) fn transported_normal(normal: V, previous_tangent: V, tangent: V, chord: V) -> Result<V> {
    let r = reflect(normal, chord)?;
    let t = reflect(previous_tangent, chord)?;
    let difference = sub(tangent, t);
    let r = if norm(difference) <= 1e-14 { r } else { reflect(r, difference)? };
    project(r, tangent)
}
pub(super) fn rotate(a: V, axis: V, angle: f64) -> V {
    let (s, c) = angle.sin_cos();
    let perpendicular = cross(axis, a);
    let parallel = dot(axis, a) * (1. - c);
    std::array::from_fn(|k| c * a[k] + s * perpendicular[k] + parallel * axis[k])
}
pub(super) fn law(curve: &Curve, fraction: f64) -> Result<f64> {
    let [a, b] = curve.domain();
    Ok(curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point[0])
}
/// A retained constant 3-vector law without a degenerate geometric line.
pub fn constant_vector_law(value: V) -> Result<Curve> {
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![value.to_vec(); 2],
        weights: vec![1., 1.],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

pub(super) fn vector_law(curve: &Curve, fraction: f64) -> Result<V> {
    let [a, b] = curve.domain();
    // At clamped endpoints the real rational point is the authored control,
    // independent of its weight; avoid a spurious rounding-induced seam gap.
    if fraction == 0. && curve.knots[..=curve.degree].iter().all(|k| *k == a) {
        let p = &curve.control_points[0];
        return Ok([p[0], p[1], p[2]]);
    }
    if fraction == 1.
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|k| *k == b)
    {
        let p = curve.control_points.last().unwrap();
        return Ok([p[0], p[1], p[2]]);
    }
    let p = curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point;
    Ok([p[0], p[1], p[2]])
}

pub(super) fn validate_law(curve: &Curve, positive: bool) -> Result<()> {
    curve.validate()?;
    check(
        curve
            .control_points
            .iter()
            .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0. && (!positive || p[0] > 0.)),
        "Sweep laws require [value,0,0] controls; scale must be positive",
    )
}
pub(super) fn sample(path: &Curve, t: f64) -> Result<(V, V)> {
    let e = path.evaluate(t)?;
    let derivative = match e.d1 {
        Some(d) => d,
        None => {
            let [a, b] = path.domain();
            let side = if t > a {
                path.trim(a, t)?
            } else {
                path.trim(t, b)?
            };
            side.evaluate(t)?
                .d1
                .ok_or_else(|| crate::input("Sweep path has no unique regular tangent"))?
        }
    };
    Ok((
        [e.point[0], e.point[1], e.point[2]],
        unit([derivative[0], derivative[1], derivative[2]])?,
    ))
}

impl<'a> Sweep<'a> {
    pub fn new(
        profile: &'a Curve,
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        options: Options,
    ) -> Result<Self> {
        profile.validate()?;
        path.validate()?;
        validate_law(scale, true)?;
        validate_law(twist, false)?;
        check(
            profile.control_points[0].len() == 3 && path.control_points[0].len() == 3,
            "Progressive sweep requires 3D profile and path",
        )?;
        check(
            (2..=1025).contains(&options.initial_sections)
                && options.initial_sections <= options.max_sections
                && options.max_sections <= 1025,
            "Progressive sweep needs 2..1025 sections with initial <= maximum",
        )?;
        check(
            options.max_deviation.is_finite() && options.max_deviation > 0.,
            "Progressive sweep deviation must be positive and finite",
        )?;
        unit(options.normal)?;
        if let Spacing::ArcLength {
            tolerance,
            max_cells,
        } = options.spacing
        {
            check(
                tolerance.is_finite() && tolerance > 0. && (1..=100_000).contains(&max_cells),
                "Sweep arc-length tolerance/work budget is invalid",
            )?;
            // Fourfold station inversion uses the curve-measure 1024-segment limit.
            check(
                options.max_sections <= 257,
                "Arc-length sweep supports at most 257 sections",
            )?;
        }
        let [a, b] = path.domain();
        for &k in &path.knots {
            if options.orientation != Orientation::Fixed
                && k > a
                && k < b
                && path.knots.iter().filter(|&&v| v == k).count() >= path.degree
            {
                let left = sample(&path.trim(a, k)?, k)?.1;
                let right = sample(&path.trim(k, b)?, k)?.1;
                check(
                    norm(sub(left, right)) < 1e-10,
                    "Split the sweep path at tangent discontinuities",
                )?;
            }
        }
        Ok(Self {
            profile,
            path,
            scale,
            twist,
            affine_laws: None,
            frame_laws: None,
            orientation_guide: None,
            contact_point: None,
            contact_source: None,
            spatial_rmf_error_limits: None,
            options,
            next_sections: Some(options.initial_sections),
        })
    }

    /// Orient the normal toward a second spatial rail, projected into the
    /// guide-tangent normal plane. Correspondence uses normalized traversal:
    /// parameter fraction, or independent arc-length fraction on both rails.
    /// This controls orientation only; it does not scale a profile onto the rail.
    pub fn with_orientation_guide(mut self, guide: &'a Curve) -> Result<Self> {
        guide.validate()?;
        check(
            guide.control_points[0].len() == 3,
            "Sweep orientation guide must be 3D",
        )?;
        check(
            self.frame_laws.is_none() && self.options.orientation != Orientation::Fixed,
            "Orientation guide cannot be combined with fixed or authored frames",
        )?;
        self.orientation_guide = Some(guide);
        self.contact_point = None;
        self.contact_source = None;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    /// Fit the profile transverse extent so its selected point follows the rail
    /// at retained stations. Initial anchor must coincide with the initial rail,
    /// and corresponding rail points must lie in the main tangent normal plane.
    /// Contact fixes normal orientation, so nonzero twist is incompatible.
    /// Only the transverse coordinate is fitted; other scale axes are retained.
    /// This is sampled contact, not a continuous interpolated-rail certificate.
    pub fn with_contact_guide(self, guide: &'a Curve, profile_parameter: f64) -> Result<Self> {
        let [a, b] = self.profile.domain();
        check(
            profile_parameter.is_finite() && profile_parameter >= a && profile_parameter <= b,
            "Contact anchor parameter is outside profile domain",
        )?;
        let p = self.profile.evaluate(profile_parameter)?.point;
        let profile = self.profile;
        let mut result = self.with_contact_point(guide, [p[0], p[1], p[2]])?;
        result.contact_source = Some((profile, profile_parameter));
        Ok(result)
    }

    pub(super) fn with_contact_point(self, guide: &'a Curve, point: V) -> Result<Self> {
        check(
            self.twist.control_points.iter().all(|p| p[0] == 0.),
            "Contact guide fixes orientation and requires zero twist",
        )?;
        let mut result = self.with_orientation_guide(guide)?;
        result.contact_point = Some(point);
        Ok(result)
    }

    /// Complete authored orientation: normalize the longitudinal axis and project
    /// the transverse direction perpendicular to it; the third axis is their
    /// right-handed cross product. Both rational vector-law domains map to the
    /// normalized traversal. Unlike RMF these axes need not follow the guide tangent.
    /// Zero/parallel evaluated directions refuse; sampling is not a continuous
    /// regularity certificate. Twist rotates around the authored longitudinal axis.
    pub fn new_authored(
        profile: &'a Curve,
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
        mut options: Options,
    ) -> Result<Self> {
        options.orientation = Orientation::Fixed;
        Self::new(profile, path, scale, twist, options)?.with_frame_laws(longitudinal, transverse)
    }

    /// Replace orientation and restart refinement; applies to a Fixed sweep.
    /// Use new_authored when constructing a sweep with these laws directly.
    pub fn with_frame_laws(
        mut self,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
    ) -> Result<Self> {
        check(
            self.options.orientation == Orientation::Fixed,
            "Authored frame laws require Fixed orientation or new_authored",
        )?;
        for curve in [longitudinal, transverse] {
            curve.validate()?;
            check(
                curve.control_points[0].len() == 3,
                "Authored frame laws require three coordinates",
            )?;
        }
        check(
            self.orientation_guide.is_none(),
            "Authored frames cannot be combined with an orientation guide",
        )?;
        self.frame_laws = Some((longitudinal, transverse));
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    pub(super) fn authored_frame(&self, fraction: f64) -> Result<Option<(V, V)>> {
        self.frame_laws
            .map(|(longitudinal, transverse)| {
                let axis = unit(vector_law(longitudinal, fraction)?)?;
                Ok((axis, project(vector_law(transverse, fraction)?, axis)?))
            })
            .transpose()
    }



    /// Axis scale is a positive dimensionless 3-vector law; center is a local
    /// frame offset in model units. q' = uniform_scale * axis_scale * q + center,
    /// then twist/frame transport. Laws use normalized traversal independently.
    pub fn with_affine_laws(mut self, axis_scale: &'a Curve, center: &'a Curve) -> Result<Self> {
        for curve in [axis_scale, center] {
            curve.validate()?;
            check(
                curve.control_points[0].len() == 3,
                "Affine sweep laws require three coordinates",
            )?;
        }
        check(
            axis_scale.control_points.iter().flatten().all(|x| *x > 0.),
            "Every axis-scale control must be positive",
        )?;
        // Equal rational poles define the exact identity for every parameter,
        // regardless of knots or positive weights. Keep generated default laws
        // from consuming certificate work for an absent affine transform.
        let identity=axis_scale.control_points.iter().all(|p|p.iter().all(|x|*x==1.))
            && center.control_points.iter().all(|p|p.iter().all(|x|*x==0.));
        self.affine_laws = if identity {None} else {Some((axis_scale, center))};
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    pub(super) fn parameters(&self, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
        if let (Some(guide),Some(_),Spacing::ArcLength {tolerance,max_cells})=(self.orientation_guide,self.contact_point,self.options.spacing) {
            let phase=arc_guide::certify(self.path,guide,max_cells)?;
            let tolerance=phase.scale_upper.filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down());
            return self.curve_parameters_with_budget(self.path,count,Some(max_cells-phase.cells),Some(tolerance));
        }
        self.curve_parameters(self.path, count)
    }

    pub(super) fn curve_parameters(&self, curve: &Curve, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
        self.curve_parameters_with_budget(curve,count,None,None)
    }

    pub(super) fn curve_parameters_with_budget(&self,curve:&Curve,count:usize,remaining:Option<usize>,tolerance_override:Option<f64>)->Result<(Vec<f64>,Option<f64>)> {
        let [a, b] = curve.domain();
        match self.options.spacing {
            Spacing::Parameter => Ok((
                (0..count)
                    .map(|i| {
                        if i + 1 == count {
                            b
                        } else {
                            a + (b - a) * i as f64 / (count - 1) as f64
                        }
                    })
                    .collect(),
                None,
            )),
            Spacing::ArcLength {
                tolerance,
                max_cells,
            } => {
                let division =
                    crate::curve_measure::divide_by_length(curve, count - 1, tolerance_override.unwrap_or(tolerance), remaining.map_or(max_cells,|r|r.min(max_cells)))?;
                check(
                    division.within_tolerance && division.points.len() == count,
                    &format!(
                        "Sweep inverse arc-length budget was not met: {:?}, {} cells, {}/{} stations",
                        division.stop_reason,
                        division.cells,
                        division.points.len(),
                        count
                    ),
                )?;
                let residual = division
                    .points
                    .iter()
                    .map(|p| p.residual_upper)
                    .fold(0_f64, f64::max);
                Ok((
                    division.points.into_iter().map(|p| p.parameter).collect(),
                    Some(residual),
                ))
            }
        }
    }

    pub(super) fn sections(&self, count: usize) -> Result<(Vec<Curve>, bool, Option<f64>)> {
        let (curves,closed,residual,_)=self.sections_with_frame_identity(count)?;
        Ok((curves,closed,residual))
    }



    /// Retained section curves for downstream authored topology. These are
    /// previews; admission still requires a level report satisfying the budget.
    pub fn sections_at(&self, count: usize) -> Result<Vec<Curve>> {
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Section preview count outside configured sweep budget",
        )?;
        Ok(self.sections(count)?.0)
    }

    /// Compute one bounded preview without advancing the iterator. This allows
    /// hosts to request levels independently and cancel between requests.
    /// Unaccepted patches remain previews and must not be promoted to a body.
    pub fn preview_at(&self, count: usize) -> Result<Level> {
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Preview section count outside configured sweep budget",
        )?;
        self.level(count)
    }

    pub(super) fn level(&self, count: usize) -> Result<Level> {
        let [_,cells,products]=self.spatial_rmf_error_limits.unwrap_or([512,10000,1000000]);
        self.level_with_error_budget(count,cells,products)
    }
    pub(super) fn level_with_error_budget(&self, count: usize, max_cells: usize, max_products: usize) -> Result<Level> {
        self.level_with_error_budget_and_length(count,max_cells,max_products,None,None,None)
    }
    pub(super) fn level_with_error_budget_and_length(&self,count:usize,max_cells:usize,max_products:usize,
        shared_length:Option<&crate::curve_measure::DivisionReport>,shared_guide:Option<&crate::curve_measure::DivisionReport>,
        shared_rmf:Option<&rmf_transport::OriginalRmfTransportReport>)->Result<Level>{
        let (coarse, closed, coarse_residual) = self.sections(count)?;
        let (fine, _, fine_residual) = self.sections(4 * (count - 1) + 1)?;
        let mut error = 0_f64;
        for (i, section) in fine.iter().enumerate() {
            let j = (i / 4).min(count - 2);
            let f = (i - 4 * j) as f64 / 4.;
            for (k, p) in section.control_points.iter().enumerate() {
                let delta: V = std::array::from_fn(|axis| {
                    p[axis]
                        - ((1. - f) * coarse[j].control_points[k][axis]
                            + f * coarse[j + 1].control_points[k][axis])
                });
                error = error.max(norm(delta));
            }
        }
        check(error.is_finite(), "Progressive sweep refinement overflowed")?;
        let patches = patches(&coarse)?;
        let length_residual_upper = coarse_residual.map(|r| r.max(fine_residual.unwrap()));
        let certificate = if let Some([steps,_,_])=self.spatial_rmf_error_limits {
            Some(authored_error::rmf_spatial_patch_error_with_transport(self,count,steps,max_cells,max_products,shared_length,shared_rmf)?)
        }else if self.frame_laws.is_some() {
            Some(authored_error::patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.contact_source.is_some() {
            Some(authored_error::contact_patch_error_with_lengths(self,count,max_cells,max_products,shared_length,shared_guide)?)
        } else if self.orientation_guide.is_some() {
            Some(authored_error::guided_patch_error_with_lengths(self,count,max_cells,max_products,shared_length,shared_guide)?)
        } else if self.options.orientation==Orientation::Fixed {
            Some(authored_error::fixed_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::FixedNormal {
            Some(authored_error::fixed_normal_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::Frenet {
            Some(authored_error::frenet_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::RotationMinimizing {
            Some(if !authored_error::original_line(self.path) {
                authored_error::rmf_planar_patch_error_with_length(self,count,max_cells,max_products,shared_length)?
            } else {self.rmf_straight_patch_error_bound(count,max_cells,max_products)?})
        } else if self.options.orientation==Orientation::CorrectedFrenet {
            Some(authored_error::corrected_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else {None};
        Ok(Level {
            patches,
            report: LevelReport {
                accepted: error <= self.options.max_deviation
                    && certificate.as_ref().and_then(|r|r.error_upper).is_none_or(|upper|upper<=self.options.max_deviation)
                    && (self.spatial_rmf_error_limits.is_none() || certificate.as_ref().is_some_and(|r|
                        r.status==super::super::progressive_miter::scalar_certificate::Status::Certified && r.within_budget)),
                sections: count,
                stations: fine.len(),
                sampled_control_deviation: error,
                budget: self.options.max_deviation,
                closed_path: closed,
                length_residual_upper,
                continuous_error_upper: certificate.as_ref().and_then(|r|r.error_upper),
                original_section_endpoint_error_upper: certificate.as_ref().and_then(|r|r.original_section_endpoint_error_upper),
                endpoint_contour_error_upper: certificate.as_ref().and_then(|r|r.endpoint_contour_error_upper),
                known_profile_error_upper: certificate.as_ref().and_then(|r|r.error_upper),
                error_certificate_cells: certificate.as_ref().map_or(0,|r|r.cells),
                decomposition_products: certificate.as_ref().map_or(0,|r|r.products),
                error_certificate_reason: certificate.as_ref().and_then(|r|r.reason).or(if certificate.is_none() {Some("transport-mode-error-unproved")} else {None}),
                continuous_bound: certificate.as_ref().is_some_and(|r|r.status==super::super::progressive_miter::scalar_certificate::Status::Certified),
            },
        })
    }
}

impl Iterator for Sweep<'_> {
    type Item = Result<Level>;
    fn next(&mut self) -> Option<Self::Item> {
        let count = self.next_sections.take()?;
        let level = self.level(count);
        if let Ok(result) = &level {
            if !result.report.accepted && count < self.options.max_sections {
                self.next_sections = Some((2 * (count - 1) + 1).min(self.options.max_sections));
            }
        }
        Some(level)
    }
}
