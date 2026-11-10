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
    let n = dot(axis, axis);
    if n <= 1e-28 {
        return Err(indeterminate("Inertia axis is numerically zero"));
    }
    let a = axis.map(|v| v / n.sqrt());
    let d: [f64; 3] = std::array::from_fn(|i| p.centroid[i] - point[i]);
    let d2 = dot(d, d) - dot(d, a).powi(2);
    let central = dot(a, std::array::from_fn(|i| dot(p.inertia_mm5[i], a)));
    let mass = p.signed_volume_mm3 * density;
    let moment = (central + p.signed_volume_mm3 * d2) * density;
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
        let inertia: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                let mut value = 0.;
                for p in 0..3 {
                    for q in 0..3 {
                        value += j[i][p] * c.properties.inertia_mm5[p][q] * j[k][q];
                    }
                }
                scale * c.density * value
            })
        });
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
