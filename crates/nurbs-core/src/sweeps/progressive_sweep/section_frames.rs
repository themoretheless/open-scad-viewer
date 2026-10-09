use super::*;

impl<'a> Sweep<'a> {
/// Identity evidence belongs to the normals/correction actually used by
    /// this constructor, before twisting and the retained copied seam.
    pub(super) fn sections_with_frame_identity(&self,count:usize)->Result<(Vec<Curve>,bool,Option<f64>,bool)> {
        let (parameters, mut residual) = self.parameters(count)?;
        let samples = parameters
            .iter()
            .map(|&t| sample(self.path, t))
            .collect::<Result<Vec<_>>>()?;
        let (start, t0) = samples[0];
        let (end, t1) = samples[count - 1];
        // On an exactly proved source plane the Bishop basis is B, B x T.
        // Carry its coefficients directly: a zero second reflection at an
        // antiparallel endpoint step must not reverse the binormal. This is
        // constructor transport only, not retained-error/smoothness evidence.
        let corrected_plane = if self.options.orientation == Orientation::CorrectedFrenet
            && self.frame_laws.is_none() && self.orientation_guide.is_none()
            && (authored_error::original_planar_rmf(self.path,self.options.normal)
                || source_plane::certify(self.path,self.options.normal,1000000)?.proved)
        { Some(unit(self.options.normal)?) } else { None };
        let corrected_transport = |normal, previous_tangent, tangent, chord| {
            if let Some(binormal) = corrected_plane {
                let previous_in_plane = cross(binormal,previous_tangent);
                let in_plane = cross(binormal,tangent);
                let a = dot(normal,previous_in_plane);
                let b = dot(normal,binormal);
                project(std::array::from_fn(|k|a*in_plane[k]+b*binormal[k]),tangent)
            } else { transported_normal(normal,previous_tangent,tangent,chord) }
        };
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
            let Some(d2) = e.d2 else {
                // C1 rational joins may lack a common second derivative while
                // their principal direction agrees. Require a tight original
                // one-sided frame enclosure; never infer it from nearby samples.
                if !matches!(e.continuity,Some(0|1)) {return Ok(None);}
                let [a,b]=self.path.domain();let f=(parameters[i]-a)/(b-a);
                let zero=constant_vector_law([0.;3])?;
                let frame=crate::sweeps::progressive_miter::authored_frame_certificate::certify_frenet_path_values(
                    self.path,&zero,[f,f],10000,
                )?;
                let Some(n)=frame.transverse else {return Ok(None);};
                let Some(t)=frame.longitudinal else {return Ok(None);};
                let diameter=norm(n.map(|v|v[1]-v[0]));
                let tangent_diameter=norm(t.map(|v|v[1]-v[0]));
                if !diameter.is_finite() || diameter>1e-10 || !tangent_diameter.is_finite() || tangent_diameter>1e-10 {return Ok(None);}
                let midpoint=n.map(|v|v[0]+(v[1]-v[0])*0.5);
                return unit(midpoint).map(Some);
            };
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
            let (guide_parameters, guide_residual) = if let Spacing::ArcLength {tolerance,max_cells}=self.options.spacing {
                if self.contact_point.is_some() {
                    let phase=arc_guide::certify(self.path,guide,max_cells)?;
                    let shared_residual=phase.scale_upper.zip(residual).map(|(scale,r)|(scale*r).next_up());
                    if let Some(r)=shared_residual.filter(|r|*r<=tolerance) {
                        (parameters.clone(),Some(r))
                    }else{
                        self.curve_parameters_with_budget(guide,count,Some(max_cells-phase.cells),None)?
                    }
                }else{self.curve_parameters(guide,count)?}
            }else{self.curve_parameters(guide,count)?};
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
            let source_initial = if !closed && principal_normal(0)?.is_none() {
                if let Some(plane)=corrected_plane {
                    let (_,phase)=crate::sweeps::progressive_miter::authored_frame_certificate::certify_initial_planar_principal_phase(
                        self.path,self.options.normal,10000,
                    )?;
                    phase.map(|phase|cross(t0,plane).map(|x|x*phase)).map(|normal|project(normal,t0)).transpose()?
                }else{None}
            }else{None};
            if let Some(normal)=source_initial {normal}else{
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
                    normal = corrected_transport(normal, samples[i].1, samples[i - 1].1,
                        sub(samples[i - 1].0, samples[i].0))?;
                }
                normal
            } else { project(self.options.normal, t0)? }
            }
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
                        let transported = corrected_transport(normals[i - 1], previous_tangent, tangent, chord)?;
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
        let closed_planar_identity=if closed
            && self.options.orientation==Orientation::RotationMinimizing
            && self.frame_laws.is_none() && self.orientation_guide.is_none()
            && authored_error::original_planar_rmf(self.path,self.options.normal) {
            let axis=(0..3).find(|&k|self.options.normal[k]!=0.).unwrap();
            let mut expected=[0.;3];
            expected[axis]=if self.options.normal[axis]>0. {1.}else {-1.};
            // Both reflections act in the source plane and preserve this
            // exact axial normal. Check the actual numerical realization,
            // including every tangent/normal and the computed holonomy.
            correction==0. && normals.iter().all(|&n|n==expected)
                && samples.iter().all(|(p,t)|p[axis]==start[axis]&&t[axis]==0.)
        }else {false};
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
        Ok((curves, closed, residual,closed_planar_identity))
    }
}
