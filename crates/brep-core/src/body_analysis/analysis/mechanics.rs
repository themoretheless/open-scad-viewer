use super::*;
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn indeterminate(message: &str) -> Error {
    Error::new("BREP_ANALYSIS_INDETERMINATE", message)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisMoment {
    pub moment_mm5: f64,
    pub mass: f64,
}
/// Moment about an arbitrary axis through `point`, by Huygens–Steiner.
pub fn inertia_about_axis(
    p: &MassProperties,
    point: [f64; 3],
    axis: [f64; 3],
    density: f64,
) -> Result<AxisMoment> {
    if point.iter().chain(&axis).any(|v| !v.is_finite()) || !density.is_finite() || density < 0. {
        return Err(Error::new(
            "BREP_INVALID_ANALYSIS_OPTIONS",
            "Axis, point and nonnegative density must be finite",
        ));
    }
    let mass = p.signed_volume_mm3 * density;
    let moment =
        axial_moment(p.centroid, p.inertia_mm5, p.signed_volume_mm3, point, axis)? * density;
    if !moment.is_finite() {
        return Err(indeterminate("Non-finite axial inertia"));
    }
    Ok(AxisMoment {
        moment_mm5: moment,
        mass,
    })
}
#[derive(Clone, Debug, PartialEq)]
pub struct PrincipalInertia {
    pub moments_mm5: [f64; 3],
    /// Rows are right-handed unit axes, ordered by descending moment.
    pub axes: [[f64; 3]; 3],
}
/// Symmetric Jacobi eigensolver. Repeated moments have non-unique axes.
pub fn principal_inertia(tensor: [[f64; 3]; 3]) -> Result<PrincipalInertia> {
    if tensor.iter().flatten().any(|v| !v.is_finite()) {
        return Err(indeterminate("Non-finite inertia tensor"));
    }
    let mut a = tensor;
    let mut v = [[0.; 3]; 3];
    for i in 0..3 {
        v[i][i] = 1.;
    }
    for _ in 0..64 {
        let (p, q) = (0..3)
            .flat_map(|i| ((i + 1)..3).map(move |j| (i, j)))
            .max_by(|&(i, j), &(k, l)| a[i][j].abs().total_cmp(&a[k][l].abs()))
            .unwrap();
        if a[p][q].abs() <= 1e-14 * (a[0][0].abs() + a[1][1].abs() + a[2][2].abs()).max(1e-300) {
            break;
        }
        let tau = (a[q][q] - a[p][p]) / (2. * a[p][q]);
        let t = tau.signum() / (tau.abs() + (1. + tau * tau).sqrt());
        let c = 1. / (1. + t * t).sqrt();
        let s = t * c;
        for k in 0..3 {
            if k != p && k != q {
                let akp = a[k][p];
                let akq = a[k][q];
                a[k][p] = c * akp - s * akq;
                a[p][k] = a[k][p];
                a[k][q] = s * akp + c * akq;
                a[q][k] = a[k][q];
            }
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.;
        a[q][p] = 0.;
        for row in &mut v {
            let vp = row[p];
            let vq = row[q];
            row[p] = c * vp - s * vq;
            row[q] = s * vp + c * vq;
        }
    }
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]).reverse());
    let moments = std::array::from_fn(|i| a[order[i]][order[i]]);
    let mut axes: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|k| v[k][order[i]]));
    if dot(axes[0], cross(axes[1], axes[2])) < 0. {
        axes[2] = axes[2].map(|x| -x);
    }
    Ok(PrincipalInertia {
        moments_mm5: moments,
        axes,
    })
}
pub struct AssemblyComponent<'a> {
    pub properties: &'a MassProperties,
    /// Row-major affine map `[linear | translation]`.
    pub transform: [[f64; 4]; 3],
    pub density: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AssemblyMass {
    pub mass: f64,
    pub centroid: [f64; 3],
    pub inertia: [[f64; 3]; 3],
}
impl AssemblyMass {
    /// Place previously measured properties under a proper rigid motion.
    /// Rotates the central inertia tensor and maps the centre to world space.
    pub fn rigidly_placed(&self, matrix: [[f64; 4]; 4]) -> Result<Self> {
        if !crate::transform::is_proper_rigid_placement(matrix)
            || !self.mass.is_finite()
            || self.mass <= 0.
        {
            return Err(indeterminate(
                "Expected positive mass and a proper rigid placement",
            ));
        }
        let rotation: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| matrix[i][j]));
        let centroid = std::array::from_fn(|i| dot(rotation[i], self.centroid) + matrix[i][3]);
        let inertia: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| dot(rotation[i], self.inertia.map(|row| dot(row, rotation[j]))))
        });
        if centroid
            .iter()
            .chain(inertia.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err(indeterminate("Non-finite placed mass properties"));
        }
        Ok(Self {
            mass: self.mass,
            centroid,
            inertia,
        })
    }

    /// Transform a measured mass distribution at unchanged material density.
    /// The affine map may include shear, reflection and nonuniform scale.
    pub fn affine_at_constant_density(&self, matrix: [[f64; 4]; 4]) -> Result<Self> {
        if matrix[3] != [0., 0., 0., 1.] || matrix.iter().flatten().any(|v| !v.is_finite()) {
            return Err(indeterminate("Expected finite affine placement"));
        }
        let j = std::array::from_fn(|i| std::array::from_fn(|k| matrix[i][k]));
        let scale = dot(j[0], cross(j[1], j[2])).abs();
        let mass = self.mass * scale;
        let centroid = std::array::from_fn(|i| matrix[i][3] + dot(j[i], self.centroid));
        let inertia = affine_inertia(self.inertia, j, scale);
        if mass <= 0.
            || !mass.is_finite()
            || centroid
                .iter()
                .chain(inertia.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(indeterminate(
                "Affine mass properties singular or outside finite range",
            ));
        }
        Ok(Self {
            mass,
            centroid,
            inertia,
        })
    }

    /// Combine the assembly with a point mass at a world-space location.
    /// Intrinsic inertia of the added weight is assumed negligible.
    pub fn with_point_mass(&self, position: [f64; 3], added_mass: f64) -> Result<Self> {
        if !self.mass.is_finite()
            || self.mass <= 0.
            || !added_mass.is_finite()
            || added_mass < 0.
            || position
                .iter()
                .chain(&self.centroid)
                .chain(self.inertia.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(indeterminate("Invalid assembly or point mass"));
        }
        if added_mass == 0. {
            return Ok(self.clone());
        }
        let mass = self.mass + added_mass;
        let delta: [f64; 3] = std::array::from_fn(|i| position[i] - self.centroid[i]);
        let centroid =
            std::array::from_fn(|i| (added_mass / mass).mul_add(delta[i], self.centroid[i]));
        let reduced = self.mass.min(added_mass) * (self.mass.max(added_mass) / mass);
        let scaled = delta.map(|v| v * reduced.sqrt());
        let inertia = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let shift = if i == j {
                    scaled[(i + 1) % 3].powi(2) + scaled[(i + 2) % 3].powi(2)
                } else {
                    -scaled[i] * scaled[j]
                };
                self.inertia[i][j] + shift
            })
        });
        if !mass.is_finite()
            || centroid
                .iter()
                .chain(inertia.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(indeterminate("Non-finite point mass combination"));
        }
        Ok(Self {
            mass,
            centroid,
            inertia,
        })
    }

    /// Combine two extended bodies or subassemblies in the same world frame.
    /// Both inertia tensors are central and expressed along the world axes.
    pub fn combined_with(&self, other: &Self) -> Result<Self> {
        if other.mass <= 0. {
            return Err(indeterminate("Expected positive mass for the added body"));
        }
        let mut combined = self.with_point_mass(other.centroid, other.mass)?;
        for (row, addition) in combined.inertia.iter_mut().zip(other.inertia) {
            for (value, term) in row.iter_mut().zip(addition) {
                *value += term;
                if !value.is_finite() {
                    return Err(indeterminate("Non-finite composite inertia"));
                }
            }
        }
        Ok(combined)
    }

    /// Remove a known component expressed in the same world frame. The caller
    /// supplies component membership. Checks the physical principal-moment
    /// inequalities of the residual up to floating-point roundoff.
    pub fn without_component(&self, removed: &Self) -> Result<Self> {
        let mass = self.mass - removed.mass;
        if !mass.is_finite() || mass <= 0. || !removed.mass.is_finite() || removed.mass <= 0. {
            return Err(indeterminate(
                "Removal requires positive component and residual mass",
            ));
        }
        let ratio = removed.mass / mass;
        let delta: [f64; 3] = std::array::from_fn(|i| self.centroid[i] - removed.centroid[i]);
        let centroid = std::array::from_fn(|i| ratio.mul_add(delta[i], self.centroid[i]));
        // The two parallel-axis shifts reduce to M*m/(M-m) times
        // the squared offset from the original centre to the removed centre.
        let scaled = delta.map(|v| (v * removed.mass.sqrt()) * (self.mass / mass).sqrt());
        let inertia: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let shift = if i == j {
                    scaled[(i + 1) % 3].powi(2) + scaled[(i + 2) % 3].powi(2)
                } else {
                    -scaled[i] * scaled[j]
                };
                (self.inertia[i][j] - removed.inertia[i][j]) - shift
            })
        });
        if centroid
            .iter()
            .chain(inertia.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err(indeterminate("Non-finite residual mass properties"));
        }
        let scale = inertia.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
        if scale > 0. {
            let normalized = inertia.map(|row| row.map(|x| x / scale));
            let tol = 128. * f64::EPSILON;
            for i in 0..3 {
                for j in 0..i {
                    if (normalized[i][j] - normalized[j][i]).abs() > tol {
                        return Err(indeterminate("Asymmetric residual inertia"));
                    }
                }
            }
            let [a, b, c] = principal_inertia(normalized)?.moments_mm5;
            if c < -tol || a > b + c + tol {
                return Err(indeterminate("Nonphysical residual inertia"));
            }
        }
        Ok(Self {
            mass,
            centroid,
            inertia,
        })
    }

    /// Mass moment in kg mm² for a world-space axis when mass is in kg
    /// and positions are in mm. Axis direction need not be normalized.
    pub fn moment_about_axis(&self, point: [f64; 3], direction: [f64; 3]) -> Result<f64> {
        axial_moment(self.centroid, self.inertia, self.mass, point, direction)
    }

    /// Radial first mass moment in world coordinates, pointing toward the
    /// heavy side. With kg and mm inputs, components are in kg mm.
    pub fn static_unbalance(&self, point: [f64; 3], direction: [f64; 3]) -> Result<[f64; 3]> {
        let unit = unit_axis(point, direction)?;
        if !self.mass.is_finite() || self.mass <= 0. {
            return Err(indeterminate("Expected positive finite assembly mass"));
        }
        let offset = std::array::from_fn(|i| self.centroid[i] - point[i]);
        let radial = cross(unit, cross(offset, unit));
        let unbalance = radial.map(|v| v * self.mass);
        if unbalance.iter().any(|v| !v.is_finite()) {
            return Err(indeterminate("Non-finite static unbalance"));
        }
        Ok(unbalance)
    }

    /// Added mass-radius vectors (kg mm) at the two ordered planes normal to
    /// the axis. Plane offsets in mm are signed along the axis from point.
    pub fn two_plane_balance_corrections(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
        planes: [f64; 2],
    ) -> Result<[[f64; 3]; 2]> {
        let span = planes[1] - planes[0];
        if !span.is_finite() || span <= 0. {
            return Err(indeterminate(
                "Expected distinct increasing finite balance plane offsets",
            ));
        }
        let unit = unit_axis(point, direction)?;
        let unbalance = self.static_unbalance(point, direction)?;
        let axial_offset = dot(std::array::from_fn(|i| self.centroid[i] - point[i]), unit);
        // The transverse part of I*u is minus the central axial-radial mass product.
        let transverse = cross(unit, cross(self.inertia.map(|row| dot(row, unit)), unit));
        let first: [f64; 3] = std::array::from_fn(|i| {
            ((axial_offset - planes[1]).mul_add(unbalance[i], -transverse[i])) / span
        });
        let second = std::array::from_fn(|i| -unbalance[i] - first[i]);
        if first.iter().chain(&second).any(|v| !v.is_finite()) {
            return Err(indeterminate("Non-finite balance correction"));
        }
        Ok([first, second])
    }

    /// Added masses in kg for two fixed sites on each correction plane.
    /// Each site is a radial offset in mm from the axis in world coordinates.
    /// Rejects unresolved, non-transverse or negative-mass solutions.
    pub fn balance_at_fixed_sites(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
        planes: [f64; 2],
        sites: [[[f64; 3]; 2]; 2],
    ) -> Result<[[f64; 2]; 2]> {
        let axis = unit_axis(point, direction)?;
        let corrections = self.two_plane_balance_corrections(point, direction, planes)?;
        let mut masses = [[0.; 2]; 2];
        for ((sites, correction), result) in sites.into_iter().zip(corrections).zip(&mut masses) {
            let radii = sites.map(|r| r[0].hypot(r[1]).hypot(r[2]));
            if radii.iter().any(|v| !v.is_finite() || *v == 0.) {
                return Err(indeterminate("Invalid balance site radius"));
            }
            let [a, b] = std::array::from_fn(|i| sites[i].map(|v| v / radii[i]));
            let angular_tol = 64. * f64::EPSILON;
            let determinant = dot(axis, cross(a, b));
            if dot(axis, a).abs() > angular_tol
                || dot(axis, b).abs() > angular_tol
                || determinant.abs() <= angular_tol
            {
                return Err(indeterminate(
                    "Balance sites require two independent transverse directions",
                ));
            }
            *result = [
                dot(axis, cross(correction, b)) / determinant / radii[0],
                dot(axis, cross(a, correction)) / determinant / radii[1],
            ];
            if result.iter().any(|v| !v.is_finite() || *v < 0.) {
                return Err(indeterminate(
                    "Balance sites cannot supply the requested correction with positive masses",
                ));
            }
        }
        Ok(masses)
    }
}

fn unit_axis(point: [f64; 3], axis: [f64; 3]) -> Result<[f64; 3]> {
    if axis.iter().chain(&point).any(|v| !v.is_finite()) {
        return Err(indeterminate("Non-finite inertia axis"));
    }
    let scale = axis.iter().map(|v| v.abs()).fold(0., f64::max);
    if scale == 0. {
        return Err(indeterminate("Zero inertia axis"));
    }
    let scaled = axis.map(|v| v / scale);
    let norm = dot(scaled, scaled).sqrt();
    Ok(scaled.map(|v| v / norm))
}

fn axial_moment(
    center: [f64; 3],
    tensor: [[f64; 3]; 3],
    mass: f64,
    point: [f64; 3],
    axis: [f64; 3],
) -> Result<f64> {
    let unit = unit_axis(point, axis)?;
    let offset = std::array::from_fn(|i| center[i] - point[i]);
    let perpendicular = cross(offset, unit);
    let moment =
        dot(unit, tensor.map(|row| dot(row, unit))) + mass * dot(perpendicular, perpendicular);
    if !moment.is_finite() {
        return Err(indeterminate("Non-finite axial inertia"));
    }
    Ok(moment)
}

fn affine_inertia(local: [[f64; 3]; 3], j: [[f64; 3]; 3], density_scale: f64) -> [[f64; 3]; 3] {
    // Second moments transform by J C Jᵀ. Convert to and from the
    // inertia tensor I = trace(C) Id - C around the affine map.
    let half_trace = (local[0][0] + local[1][1] + local[2][2]) * 0.5;
    let second: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|k| {
            let mut value = 0.;
            for p in 0..3 {
                for q in 0..3 {
                    let entry = (if p == q { half_trace } else { 0. }) - local[p][q];
                    value += j[i][p] * entry * j[k][q];
                }
            }
            density_scale * value
        })
    });
    let inertia: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|k| {
            if i == k {
                second[(i + 1) % 3][(i + 1) % 3] + second[(i + 2) % 3][(i + 2) % 3]
            } else {
                -second[i][k]
            }
        })
    });
    inertia
}

/// Composite properties of instances under nonsingular affine maps.
pub fn assembly_mass(components: &[AssemblyComponent<'_>]) -> Result<AssemblyMass> {
    if components.is_empty() || components.len() > 4096 {
        return Err(Error::new(
            "BREP_INVALID_ANALYSIS_OPTIONS",
            "Assembly requires 1..4096 components",
        ));
    }
    let mut mass = 0.;
    let mut first = [0.; 3];
    let mut placed = Vec::with_capacity(components.len());
    for c in components {
        if c.transform.iter().flatten().any(|v| !v.is_finite())
            || !c.density.is_finite()
            || c.density < 0.
        {
            return Err(Error::new(
                "BREP_INVALID_ANALYSIS_OPTIONS",
                "Invalid component transform or density",
            ));
        }
        let j: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|k| c.transform[i][k]));
        let det = dot(j[0], cross(j[1], j[2]));
        if det.abs() <= 1e-28 {
            return Err(indeterminate("Singular assembly transform"));
        }
        let scale = det.abs();
        let m = c.properties.signed_volume_mm3 * scale * c.density;
        let center: [f64; 3] =
            std::array::from_fn(|i| c.transform[i][3] + dot(j[i], c.properties.centroid));
        let inertia = affine_inertia(c.properties.inertia_mm5, j, scale * c.density);
        mass += m;
        for k in 0..3 {
            first[k] += m * center[k];
        }
        placed.push((m, center, inertia));
    }
    if mass <= 0. || !mass.is_finite() {
        return Err(indeterminate("Assembly mass must be positive and finite"));
    }
    let centroid = first.map(|x| x / mass);
    let mut inertia = [[0.; 3]; 3];
    for (m, c, local) in placed {
        let d = std::array::from_fn(|k| c[k] - centroid[k]);
        let d2 = dot(d, d);
        for i in 0..3 {
            for k in 0..3 {
                inertia[i][k] += local[i][k] + m * (if i == k { d2 } else { 0. } - d[i] * d[k]);
            }
        }
    }
    if inertia.iter().flatten().any(|v| !v.is_finite()) {
        return Err(indeterminate("Non-finite assembly inertia"));
    }
    Ok(AssemblyMass {
        mass,
        centroid,
        inertia,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-7 * b.abs().max(1.), "{a} != {b}");
    }
    fn box_properties() -> MassProperties {
        mass_properties(&cuboid([0.; 3], [2., 4., 6.]).unwrap(), 1e-8, 200000).unwrap()
    }
    #[test]
    fn reflected_scaled_box_matches_axial_moment_of_world_box() {
        let properties = box_properties();
        let assembly = assembly_mass(&[AssemblyComponent {
            properties: &properties,
            density: 0.5,
            transform: [[0., -3., 0., 0.], [2., 0., 0., 0.], [0., 0., -1., 0.]],
        }])
        .unwrap();
        let expected = 144. * (144. + 16.) / 12. + 144. * 25.;
        for dir in [1e-300, -1e300] {
            near(
                assembly
                    .moment_about_axis([-3., 6., 1e15], [0., 0., dir])
                    .unwrap(),
                expected,
            );
        }
        assert!(assembly.moment_about_axis([0.; 3], [0.; 3]).is_err());
    }

    #[test]
    fn central_box_tensor_matches_independent_dimensions_formula() {
        let m = box_properties();
        near(m.signed_volume_mm3, 48.);
        assert_eq!(m.centroid, [1., 2., 3.]);
        near(m.inertia_mm5[0][0], 48. * (4. * 4. + 6. * 6.) / 12.);
        near(m.inertia_mm5[1][1], 48. * (2. * 2. + 6. * 6.) / 12.);
        near(m.inertia_mm5[2][2], 48. * (2. * 2. + 4. * 4.) / 12.);
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    near(m.inertia_mm5[i][j], 0.);
                }
            }
        }
    }
    #[test]
    fn arbitrary_axis_matches_box_formula() {
        let m = box_properties();
        // This non-unit Z axis passes through the COM: no Huygens term.
        let central = inertia_about_axis(&m, [1., 2., 0.], [0., 0., 2.], 1.).unwrap();
        near(central.moment_mm5, 48. * (2. * 2. + 4. * 4.) / 12.);
        near(central.mass, 48.);
    }
    #[test]
    fn shifted_axis_and_density_scale_are_independent() {
        let m = box_properties();
        // COM=[1,2,3], so the Z axis through the origin has squared distance 5.
        let shifted = inertia_about_axis(&m, [0.; 3], [0., 0., 1.], 2.5).unwrap();
        let unit_density = 48. * 20. / 12.;
        near(shifted.moment_mm5, 2.5 * (unit_density + 48. * 5.));
        near(shifted.mass, 2.5 * 48.);
    }
    #[test]
    fn principal_axes_and_composite_huygens() {
        let a = box_properties();
        let p = principal_inertia(a.inertia_mm5).unwrap();
        near(p.moments_mm5[0], 48. * (16. + 36.) / 12.);
        near(p.moments_mm5[1], 48. * (4. + 36.) / 12.);
        near(p.moments_mm5[2], 48. * (4. + 16.) / 12.);
        let b = AssemblyComponent {
            properties: &a,
            transform: [[1., 0., 0., 10.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
            density: 2.,
        };
        let assembly = assembly_mass(&[
            AssemblyComponent {
                properties: &a,
                transform: [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
                density: 2.,
            },
            b,
        ])
        .unwrap();
        near(assembly.mass, 192.);
        assert_eq!(assembly.centroid, [6., 2., 3.]);
        // Both 96-mass instances are five units from the assembly COM.
        near(assembly.inertia[2][2], 2. * (2. * 80.) + 2. * (96. * 25.));
    }
    #[test]
    fn transformed_single_instance_uses_affine_tensor_congruence() {
        let a = box_properties();
        // Rotate +90 degrees about Z, then translate. X/Y inertia swap;
        // translation cannot affect a single-component central tensor.
        let assembly = assembly_mass(&[AssemblyComponent {
            properties: &a,
            transform: [[0., -1., 0., 10.], [1., 0., 0., 20.], [0., 0., 1., 0.]],
            density: 3.,
        }])
        .unwrap();
        near(assembly.mass, 144.);
        assert_eq!(assembly.centroid, [8., 21., 3.]);
        near(assembly.inertia[0][0], 3. * 160.);
        near(assembly.inertia[1][1], 3. * 208.);
        near(assembly.inertia[2][2], 3. * 80.);
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    near(assembly.inertia[i][j], 0.);
                }
            }
        }
    }
}
