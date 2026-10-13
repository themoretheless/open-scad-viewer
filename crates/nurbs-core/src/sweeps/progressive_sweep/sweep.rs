use super::*;

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
        self.with_contact_point(guide, [p[0], p[1], p[2]])
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

    fn authored_frame(&self, fraction: f64) -> Result<Option<(V, V)>> {
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
        self.affine_laws = Some((axis_scale, center));
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    fn parameters(&self, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
        self.curve_parameters(self.path, count)
    }

    fn curve_parameters(&self, curve: &Curve, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
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
                    crate::curve_measure::divide_by_length(curve, count - 1, tolerance, max_cells)?;
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

    fn sections(&self, count: usize) -> Result<(Vec<Curve>, bool, Option<f64>)> {
        let (parameters, mut residual) = self.parameters(count)?;
        let samples = parameters
            .iter()
            .map(|&t| sample(self.path, t))
            .collect::<Result<Vec<_>>>()?;
        let (start, t0) = samples[0];
        let (end, t1) = samples[count - 1];
        let closed = closed_extent(self.path, start, end)?;
        check(
            !self.path.periodic || closed,
            "Periodic sweep endpoints must coincide",
        )?;
        if closed {
            check(
                count >= 4 && self.options.initial_sections >= 4,
                "Closed progressive sweep needs at least four sections",
            )?;
            check(
                self.options.orientation == Orientation::Fixed || norm(sub(t0, t1)) < 1e-10,
                "Closed sweep tangent endpoints must agree",
            )?;
            check(
                law(self.scale, 0.)? == law(self.scale, 1.)?,
                "Closed sweep scale endpoints must agree",
            )?;
            if let Some((axis_scale, center)) = self.affine_laws {
                for curve in [axis_scale, center] {
                    check(
                        vector_law(curve, 0.)? == vector_law(curve, 1.)?,
                        "Closed sweep axis scale and center endpoints must agree",
                    )?;
                }
            }
            let twist = law(self.twist, 1.)? - law(self.twist, 0.)?;
            check(
                twist.is_finite() && (twist / TAU - (twist / TAU).round()).abs() <= 1e-12,
                "Closed sweep twist endpoints must differ by whole turns",
            )?;
        }
        let principal_normal = |i: usize| -> Result<Option<V>> {
            let e = self.path.evaluate(parameters[i])?;
            let Some(d2) = e.d2 else { return Ok(None); };
            let acceleration = [d2[0], d2[1], d2[2]];
            check(acceleration.iter().all(|x| x.is_finite()), "Sweep curvature must be finite")?;
            if acceleration.iter().all(|x| *x == 0.) { return Ok(None); }
            let n = unit(acceleration)?;
            let tangent = samples[i].1;
            let transverse = sub(n, tangent.map(|x| x * dot(n, tangent)));
            if norm(transverse) <= 1e-12 { Ok(None) } else { unit(transverse).map(Some) }
        };
        let frenet = |i: usize| -> Result<V> {
            principal_normal(i)?.ok_or_else(|| crate::input("Frenet sweep needs a unique nonzero principal normal"))
        };
        let mut contact_widths = Vec::new();
        let guide_normals = if let Some(guide) = self.orientation_guide {
            let (guide_parameters, guide_residual) = self.curve_parameters(guide, count)?;
            residual = match (residual, guide_residual) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            let normals = guide_parameters
                .iter()
                .zip(&samples)
                .map(|(&t, &(position, tangent))| {
                    let p = guide.evaluate(t)?.point;
                    let offset = sub([p[0], p[1], p[2]], position);
                    if self.contact_point.is_some() {
                        let width = norm(offset);
                        check(
                            width.is_finite()
                                && width > 0.
                                && dot(offset, tangent).abs() <= 1e-10 * width,
                            "Contact rail must lie in the path normal plane",
                        )?;
                        contact_widths.push(width);
                    }
                    project(offset, tangent)
                })
                .collect::<Result<Vec<_>>>()?;
            if closed && self.contact_point.is_some() {
                check(
                    (contact_widths[0] - contact_widths[count - 1]).abs()
                        <= 1e-10 * contact_widths[0],
                    "Closed contact rail widths must agree",
                )?;
            }
            if closed {
                check(
                    norm(sub(normals[0], *normals.last().unwrap())) < 1e-10,
                    "Closed sweep orientation-guide endpoints must agree",
                )?;
            }
            Some(normals)
        } else {
            None
        };
        let initial_frame = self.authored_frame(0.)?;
        let coordinate_axis = initial_frame.map_or(t0, |f| f.0);
        let initial = if let Some(normals) = &guide_normals {
            normals[0]
        } else if let Some((_, normal)) = initial_frame {
            normal
        } else if self.options.orientation == Orientation::Frenet {
            frenet(0)?
        } else if self.options.orientation == Orientation::CorrectedFrenet {
            let mut first = None;
            for i in 0..count {
                if let Some(normal) = principal_normal(i)? {
                    first = Some((i, normal));
                    break;
                }
            }
            if let Some((index, mut normal)) = first {
                // Resolve an initially straight interval from the first available
                // principal normal rather than introducing a seed-normal jump.
                for i in (1..=index).rev() {
                    normal = transported_normal(normal, samples[i].1, samples[i - 1].1,
                        sub(samples[i - 1].0, samples[i].0))?;
                }
                normal
            } else { project(self.options.normal, t0)? }
        } else {
            project(self.options.normal, t0)?
        };
        let side = cross(coordinate_axis, initial);
        let contact_anchor = if let Some(p) = self.contact_point {
            let q = sub([p[0], p[1], p[2]], start);
            let width = dot(q, initial);
            check(
                width > 0.
                    && norm(sub(q, initial.map(|v| v * contact_widths[0])))
                        <= 1e-10 * contact_widths[0],
                "Initial contact anchor must coincide with orientation rail",
            )?;
            Some(width)
        } else {
            None
        };
        let coordinates: Vec<V> = self
            .profile
            .control_points
            .iter()
            .map(|p| {
                let q = sub([p[0], p[1], p[2]], start);
                [dot(q, initial), dot(q, side), dot(q, coordinate_axis)]
            })
            .collect();
        let mut normals = vec![initial];
        let mut lengths = vec![0.];
        for i in 1..count {
            let (previous, previous_tangent) = samples[i - 1];
            let (position, tangent) = samples[i];
            let chord = sub(position, previous);
            lengths.push(lengths[i - 1] + norm(chord));
            let next = if let Some(normals) = &guide_normals {
                normals[i]
            } else if let Some((_, normal)) = self.authored_frame(i as f64 / (count - 1) as f64)? {
                normal
            } else {
                match self.options.orientation {
                    Orientation::Fixed => initial,
                    Orientation::FixedNormal => project(self.options.normal, tangent)?,
                    Orientation::Frenet => frenet(i)?,
                    Orientation::CorrectedFrenet => {
                        let transported = transported_normal(normals[i - 1], previous_tangent, tangent, chord)?;
                        match principal_normal(i)? {
                            Some(n) => if dot(n, transported) < 0. { n.map(|x| -x) } else { n },
                            None => transported,
                        }
                    }
                    Orientation::RotationMinimizing => transported_normal(normals[i - 1], previous_tangent, tangent, chord)?,
                }
            };
            normals.push(next);
        }
        let correction = if closed
            && self.orientation_guide.is_none()
            && self.options.orientation == Orientation::RotationMinimizing
        {
            dot(t0, cross(normals[count - 1], initial)).atan2(dot(normals[count - 1], initial))
        } else {
            0.
        };
        if closed
            && matches!(
                self.options.orientation,
                Orientation::Frenet | Orientation::CorrectedFrenet | Orientation::FixedNormal
            )
        {
            check(
                norm(sub(normals[count - 1], initial)) < 1e-10,
                "Closed sweep orientation endpoints must agree",
            )?;
        }
        if closed && self.frame_laws.is_some() {
            let first = self.authored_frame(0.)?.unwrap();
            let last = self.authored_frame(1.)?.unwrap();
            check(
                norm(sub(first.0, last.0)) < 1e-10 && norm(sub(first.1, last.1)) < 1e-10,
                "Closed sweep authored frame endpoints must agree",
            )?;
        }
        let total = lengths[count - 1];
        check(
            total.is_finite() && total > 0.,
            "Sweep path traversal has zero or nonfinite sampled length",
        )?;
        let mut curves = Vec::with_capacity(count);
        for i in 0..count {
            let fraction = i as f64 / (count - 1) as f64;
            let scale = law(self.scale, fraction)?;
            let twist = law(self.twist, fraction)?;
            check(
                scale.is_finite() && scale > 0. && twist.is_finite(),
                "Sweep law evaluation is invalid",
            )?;
            let (axes, center) = if let Some((axes, center)) = self.affine_laws {
                (vector_law(axes, fraction)?, vector_law(center, fraction)?)
            } else {
                ([1.; 3], [0.; 3])
            };
            check(
                axes.iter().all(|x| x.is_finite() && *x > 0.)
                    && center.iter().all(|x| x.is_finite()),
                "Affine sweep law evaluation is invalid",
            )?;
            let contact_fit = if let Some(anchor) = contact_anchor {
                check(
                    center[1] == 0. && center[2] == 0.,
                    "Contact guide requires zero side/longitudinal center offset",
                )?;
                let transformed = scale * axes[0] * anchor + center[0];
                check(
                    transformed.is_finite() && transformed > 0.,
                    "Contact anchor scale must remain positive",
                )?;
                let fit = contact_widths[i] / transformed;
                check(fit.is_finite() && fit > 0., "Contact width fit overflowed")?;
                fit
            } else {
                1.
            };
            let tangent = if let Some((axis, _)) = self.authored_frame(fraction)? {
                axis
            } else if self.options.orientation == Orientation::Fixed {
                t0
            } else {
                samples[i].1
            };
            let normal = rotate(normals[i], tangent, twist + correction * lengths[i] / total);
            let side = cross(tangent, normal);
            let mut curve = self.profile.clone();
            curve.control_points = coordinates
                .iter()
                .map(|q| {
                    (0..3)
                        .map(|k| {
                            if self.affine_laws.is_none() && contact_anchor.is_none() {
                                return samples[i].0[k]
                                    + scale
                                        * (q[0] * normal[k] + q[1] * side[k] + q[2] * tangent[k]);
                            }
                            samples[i].0[k]
                                + contact_fit * (scale * axes[0] * q[0] + center[0]) * normal[k]
                                + (scale * axes[1] * q[1] + center[1]) * side[k]
                                + (scale * axes[2] * q[2] + center[2]) * tangent[k]
                        })
                        .collect()
                })
                .collect();
            curve.validate()?;
            curves.push(curve);
        }
        if closed {
            let first = curves[0].clone();
            *curves.last_mut().unwrap() = first;
        }
        Ok((curves, closed, residual))
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
        Ok(Level {
            patches,
            report: LevelReport {
                accepted: error <= self.options.max_deviation,
                sections: count,
                stations: fine.len(),
                sampled_control_deviation: error,
                budget: self.options.max_deviation,
                closed_path: closed,
                length_residual_upper,
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
