//! Scalar sweep compatibility constructors. Geometry and budgets stay native.
use super::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
use crate::{
    Result, check,
    curve::Curve,
    numeric, resource,
    surface::{Surface, loft},
};

fn positive_scale(scale: &Curve) -> Result<()> {
    scale.validate()?;
    check(
        scale.control_points[0].len() == 3
            && scale
                .control_points
                .iter()
                .all(|p| p[0] > 0. && p[1] == 0. && p[2] == 0.),
        "Sweep scale must be a positive scalar [value,0,0] law",
    )
}
fn normalized(curve: &Curve) -> Result<Curve> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 3,
        "Scaled sweep requires 3D curves",
    )?;
    // Materialize one authored period. No periodic surface claim is inferred.
    let mut source = curve.clone();
    source.periodic = false;
    let [a, b] = source.domain();
    let mut result = source.trim(a, b)?;
    for k in &mut result.knots {
        *k = (*k - a) / (b - a);
    }
    result.validate()?;
    Ok(result)
}
fn elevate(mut controls: Vec<f64>, degree: usize) -> Vec<f64> {
    while controls.len() <= degree {
        let n = controls.len();
        let mut next = vec![controls[0]; n + 1];
        next[n] = controls[n - 1];
        for i in 1..n {
            let t = i as f64 / n as f64;
            next[i] = t * controls[i - 1] + (1. - t) * controls[i];
        }
        controls = next;
    }
    controls
}
/// origin + C(v)-C(0) + r(v)*(P(u)-origin), with independent normalized
/// path/law domains. Rational Bernstein products, not sampled fitting. Arithmetic
/// rounding, regularity, injectivity and solid topology remain uncertified.
pub fn scaled_sweep(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    origin: [f64; 3],
) -> Result<Surface> {
    profile.validate()?;
    positive_scale(scale)?;
    check(
        profile.control_points[0].len() == 3 && origin.iter().all(|x| x.is_finite()),
        "Scaled sweep needs a 3D profile and finite origin",
    )?;
    let path = normalized(path)?;
    let scale = normalized(scale)?;
    let start = path.evaluate(0.)?.point;
    let cells = crate::gordon::denominators::prepare(&[path, scale])?;
    let degree = cells
        .iter()
        .flat_map(|c| c.numerators.iter().map(|n| n.len() - 1))
        .max()
        .unwrap();
    if degree > 25 || profile.control_points.len() > 32 || cells.len() * degree + 1 > 32 {
        return Err(resource(
            "Scaled sweep exceeds degree 25 or 32 controls per surface axis",
        ));
    }
    let mut points: Vec<Vec<Vec<f64>>> = vec![Vec::new(); profile.control_points.len()];
    let mut weights = points.iter().map(|_| Vec::new()).collect::<Vec<Vec<f64>>>();
    let mut knots = vec![0.; degree + 1];
    let mut previous_weight = None;
    for (cell_index, cell) in cells.iter().enumerate() {
        let denominator = elevate(cell.denominator.clone(), degree);
        let numerators = cell
            .numerators
            .iter()
            .map(|n| {
                (0..3)
                    .map(|axis| elevate(n.iter().map(|p| p[axis]).collect(), degree))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let gauge = previous_weight.map_or(1., |w: f64| w / denominator[0]);
        numeric(
            gauge.is_finite() && gauge > 0.,
            "Scaled sweep denominator gauge overflow",
        )?;
        for (i, p) in profile.control_points.iter().enumerate() {
            let row = (0..=degree)
                .map(|j| {
                    (0..3)
                        .map(|axis| {
                            numerators[0][axis][j] / denominator[j] + origin[axis] - start[axis]
                                + (p[axis] - origin[axis]) * numerators[1][0][j] / denominator[j]
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if cell_index > 0 {
                let last = points[i].last().unwrap();
                numeric(
                    (0..3).all(|axis| {
                        (last[axis] - row[0][axis]).abs()
                            <= 128. * f64::EPSILON * (1. + last[axis].abs().max(row[0][axis].abs()))
                    }),
                    "Scaled sweep cell seam mismatch",
                )?;
            }
            let first = usize::from(cell_index > 0);
            points[i].extend(row.into_iter().skip(first));
            weights[i].extend(
                denominator
                    .iter()
                    .skip(first)
                    .map(|w| profile.weights[i] * w * gauge),
            );
        }
        previous_weight = Some(denominator[degree] * gauge);
        if cell_index > 0 {
            knots.extend(std::iter::repeat_n(cell.domain[0], degree));
        }
    }
    knots.extend(vec![1.; degree + 1]);
    let surface = Surface {
        degree_u: profile.degree,
        degree_v: degree,
        knots_u: profile.knots.clone(),
        knots_v: knots,
        control_points: points,
        weights,
        periodic_u: profile.periodic,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}
/// A retained RMF loft admitted only by a whole-domain, rounding-inclusive
/// deviation certificate. The fourfold comparison remains a separate diagnostic.
pub fn checked_profile_sweep(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
) -> Result<value_codec::Value> {
    checked_profile_sweep_with_cells(profile, path, scale, normal, sections, budget, 16384)
}
pub fn checked_profile_sweep_with_cells(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
    max_cells: usize,
) -> Result<value_codec::Value> {
    positive_scale(scale)?;
    check(
        (2..=32).contains(&sections),
        "Profile sweep requires 2..32 sections",
    )?;
    check(
        budget.is_finite() && budget >= 0.,
        "Sweep deviation budget must be finite and nonnegative",
    )?;
    let twist = constant_vector_law([0.; 3])?;
    let sweep = Sweep::new(
        profile,
        path,
        scale,
        &twist,
        Options {
            normal,
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: sections,
            max_sections: sections,
            max_deviation: budget.max(f64::MIN_POSITIVE),
        },
    )?;
    let level = sweep.preview_at(sections)?;
    let retained_sections = sweep.sections_at(sections)?;
    let mut seam_certificate = value_codec::Value::Null;
    let candidate = {
        let mut surface = if level.report.closed_path && sections >= 4 {
            let (surface, proof) = super::profile_seam::build(&retained_sections)?;
            seam_certificate = proof;
            surface
        } else {
            loft(&retained_sections)?
        };
        if level.report.closed_path && surface.degree_v == 1 {
            surface.periodic_v = true;
            surface.knots_v = (0..sections + 2)
                .map(|i| (i as f64 - 1.) / (sections - 1) as f64)
                .collect();
        } else if !level.report.closed_path {
            for k in &mut surface.knots_v {
                *k /= (sections - 1) as f64;
            }
        }
        surface.validate()?;
        surface
    };
    let certificate = super::profile_certificate::certify(
        profile, path, scale, normal, &candidate, budget, max_cells,
    )?;
    let geometry_certificate=if certificate.within_budget {
        super::profile_geometry::inspect(&candidate,max_cells).unwrap_or_else(|_|value_codec::json!({
            "certified":false,"reason":"geometry-proof-unresolved","scope":"single-untrimmed-surface",
            "solidTopologyCertified":false,"pairwiseFaceContactsCertified":false}))
    } else {value_codec::json!({"certified":false,"reason":"deviation-unproved", "scope":"single-untrimmed-surface",
        "solidTopologyCertified":false,"pairwiseFaceContactsCertified":false})};
    let continuous = certificate.error_upper.is_some();
    let accepted = certificate.within_budget;
    let surface = if accepted { Some(candidate) } else { None };
    let seam = if !level.report.closed_path {
        "open"
    } else {
        match seam_certificate["order"].as_u64() {
            Some(2) => "G2",
            Some(1) => "G1",
            _ => "C0",
        }
    };
    Ok(
        value_codec::json!({"surface":surface,"report":{"accepted":accepted,"sampledControlDeviation":level.report.sampled_control_deviation,"budget":budget,"stations":level.report.stations,"sections":sections,"closedPath":level.report.closed_path,"seamContinuity":seam,"seamCertificate":seam_certificate,"geometryCertificate":geometry_certificate,"continuousBound":continuous,"method":"double-reflection-fourfold-section-refinement","continuousCertificate":{"errorUpper":certificate.error_upper,"withinBudget":certificate.within_budget,"cells":certificate.cells,"maxCells":max_cells,"method":certificate.method,"reason":certificate.reason,"scope":"matched-parameter-profile-deviation","regularityCertified":false,"globalEmbeddingCertified":false,"seamSmoothnessCertified":false}}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve(points: &[[f64; 3]], weights: &[f64], domain: [f64; 2]) -> Curve {
        Curve {
            degree: points.len() - 1,
            knots: [vec![domain[0]; points.len()], vec![domain[1]; points.len()]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: weights.to_vec(),
            periodic: false,
        }
    }
    #[test]
    fn rational_scaled_surface_preserves_formula_jets_and_source_data() {
        let profile = curve(
            &[[2., 1., 0.], [3., 2., 1.], [4., -1., 2.]],
            &[1., 2., 3.],
            [2., 5.],
        );
        let path = curve(
            &[[4., -2., 3.], [6., 8., 5.], [9., 2., 10.]],
            &[2., 0.5, 3.],
            [-3., 7.],
        );
        let scale = curve(&[[0.5, 0., 0.], [2., 0., 0.]], &[3., 1.], [10., 14.]);
        let origin = [1., 2., -1.];
        let before = value_codec::to_string(&value_codec::json!([profile, path, scale])).unwrap();
        let s = scaled_sweep(&profile, &path, &scale, origin).unwrap();
        for i in 0..=12 {
            for j in 0..=12 {
                let u = 2. + 3. * i as f64 / 12.;
                let v = j as f64 / 12.;
                let p = profile.evaluate(u).unwrap();
                let c = path.evaluate(-3. + 10. * v).unwrap();
                let r = scale.evaluate(10. + 4. * v).unwrap();
                let e = s.evaluate(u, v).unwrap();
                for axis in 0..3 {
                    let expected = origin[axis] + c.point[axis] - path.control_points[0][axis]
                        + r.point[0] * (p.point[axis] - origin[axis]);
                    assert!(
                        (e.point[axis] - expected).abs() < 2e-12,
                        "u={u} v={v} axis={axis} actual={} expected={expected}",
                        e.point[axis]
                    );
                    assert!(
                        (e.first_derivatives().unwrap().0[axis]
                            - r.point[0] * p.d1.as_ref().unwrap()[axis])
                            .abs()
                            < 2e-11
                    );
                    let dv = 10. * c.d1.as_ref().unwrap()[axis]
                        + 4. * r.d1.as_ref().unwrap()[0] * (p.point[axis] - origin[axis]);
                    assert!((e.first_derivatives().unwrap().1[axis] - dv).abs() < 2e-10);
                }
            }
        }
        assert_eq!(
            before,
            value_codec::to_string(&value_codec::json!([profile, path, scale])).unwrap()
        );
    }
    #[test]
    fn differing_internal_knots_preserve_both_laws() {
        let profile = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 2.], [0., 1.]);
        let path = Curve {
            degree: 1,
            knots: vec![2., 2., 3., 6., 6.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 2.], vec![0., 1., 5.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let scale = Curve {
            degree: 1,
            knots: vec![-2., -2., 0., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.], vec![0.5, 0., 0.]],
            weights: vec![3., 1., 2.],
            periodic: false,
        };
        let s = scaled_sweep(&profile, &path, &scale, [0.; 3]).unwrap();
        for j in 0..=96 {
            let v = j as f64 / 96.;
            let p = profile.evaluate(0.3).unwrap().point;
            let c = path.evaluate(2. + 4. * v).unwrap().point;
            let r = scale.evaluate(-2. + 3. * v).unwrap().point[0];
            let q = s.evaluate(0.3, v).unwrap().point;
            for k in 0..3 {
                assert!((q[k] - c[k] - r * p[k]).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn profile_sweep_scale_and_sampled_refusal() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 2.], [0., 1.]);
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[1., 1.], [2., 7.]);
        let scale = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [-4., 4.]);
        let result = checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 5, 1e-12).unwrap();
        assert_eq!(result["report"]["accepted"], true);
        let s: Surface = value_codec::from_value(result["surface"].clone()).unwrap();
        for j in 0..=20 {
            let v = j as f64 / 20.;
            let e = s.evaluate(0.3, v).unwrap();
            assert!((e.point[0] - (1. + v) * p.evaluate(0.3).unwrap().point[0]).abs() < 1e-12);
            assert!((e.point[2] - 5. * v).abs() < 1e-12);
        }
        let arc = curve(
            &[[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            &[1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            [0., 1.],
        );
        let coarse = checked_profile_sweep(&p, &arc, &scale, [1., 0., 0.], 3, 1e-6).unwrap();
        assert_eq!(coarse["report"]["accepted"], false);
        assert!(coarse["surface"].is_null());
        assert_eq!(coarse["report"]["continuousBound"], false);
    }
    #[test]
    fn invalid_laws_and_resource_limits_refuse() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[1., 1.], [0., 1.]);
        let mut scale = constant_vector_law([1., 0., 0.]).unwrap();
        for bad in [0., -1.] {
            scale.control_points[0][0] = bad;
            assert!(scaled_sweep(&p, &path, &scale, [0.; 3]).is_err());
            assert!(checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 5, 1.).is_err());
        }
        scale.control_points[0][0] = 1.;
        assert!(checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 33, 1.).is_err());
        assert!(checked_profile_sweep(&p, &path, &scale, [0., 0., 1.], 5, 1.).is_err());
        assert!(checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 5, -1.).is_err());
        let mut large = path.clone();
        large.knots = vec![0., 0.];
        large.knots.extend((1..33).map(|i| i as f64 / 33.));
        large.knots.extend([1., 1.]);
        large.control_points = (0..34).map(|i| vec![0., 0., i as f64]).collect();
        large.weights = vec![1.; 34];
        assert!(scaled_sweep(&p, &large, &scale, [0.; 3]).is_err());
    }
    #[test]
    fn closed_rmf_seam_requires_matching_scale() {
        let path = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![-1., 1., 0.],
                vec![-1., 0., 0.],
                vec![-1., -1., 0.],
                vec![0., -1., 0.],
                vec![1., -1., 0.],
                vec![1., 0., 0.],
            ],
            weights: (0..9)
                .map(|i| {
                    if i % 2 == 0 {
                        1.
                    } else {
                        std::f64::consts::FRAC_1_SQRT_2
                    }
                })
                .collect(),
            periodic: false,
        };
        let p = curve(&[[1., 0., 0.], [1.2, 0., 0.]], &[1., 1.], [0., 1.]);
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let before = value_codec::to_string(&value_codec::json!([p, path, scale])).unwrap();
        let r = checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 17, 0.01).unwrap();
        assert_eq!(r["report"]["closedPath"], true);
        assert_eq!(r["report"]["seamContinuity"], "G2", "{r}");
        assert_eq!(r["report"]["accepted"], true, "{r}");
        assert_eq!(r["report"]["continuousBound"], true);
        assert!(r["report"]["sampledControlDeviation"].as_f64().unwrap() > 0.01);
        let s: Surface = value_codec::from_value(r["surface"].clone()).unwrap();
        assert!(s.periodic_v);
        assert_eq!(r["report"]["geometryCertificate"]["certified"],true,"{r}");
        assert_eq!(s.degree_v, 3);
        for row in &s.control_points {
            assert_eq!(&row[..3], &row[row.len() - 3..]);
        }
        for count in [4,6,7,10,12,15,20,24,31,32] {
            let alternate=checked_profile_sweep(&p,&path,&scale,[1.,0.,0.],count,1.).unwrap();
            assert_eq!(alternate["report"]["accepted"],true,"count={count}: {alternate}");
            assert_eq!(alternate["report"]["seamContinuity"],if count==4 {"G1"}else{"G2"},"count={count}: {alternate}");
            let surface:Surface=value_codec::from_value(alternate["surface"].clone()).unwrap();
            let a=surface.evaluate(0.37,0.).unwrap();let b=surface.evaluate(0.37,1.).unwrap();
            for (a,b) in a.point.iter().zip(&b.point){assert!((a-b).abs()<1e-12);}
            for (a,b) in a.first_derivatives().unwrap().1.iter().zip(b.first_derivatives().unwrap().1){assert!((a-b).abs()<1e-10);}
            if count>4 {for (a,b) in a.second_derivatives().unwrap().2.iter().zip(b.second_derivatives().unwrap().2){assert!((a-b).abs()<1e-9);}}
        }
        let bound = r["report"]["continuousCertificate"]["errorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=200 {
            for u in [0., 0.37, 1.] {
                let v = i as f64 / 200.;
                let c = path.evaluate(v).unwrap().point;
                let radius = c[0].hypot(c[1]);
                let q = s.evaluate(u, v).unwrap().point;
                let offset = p.evaluate(u).unwrap().point[0] - 1.;
                let expected = [
                    c[0] + offset * c[0] / radius,
                    c[1] + offset * c[1] / radius,
                    0.,
                ];
                assert!(
                    (0..3)
                        .map(|k| (q[k] - expected[k]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                        <= bound
                );
            }
        }
        // Inspect the represented active Bezier strips independently of the
        // constructor's report, then prove sensitivity to a changed tangent.
        let mut a = s.clone();
        let mut b = s.clone();
        for surface in [&mut a, &mut b] {
            surface.periodic_v = false;
            surface.knots_v = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        }
        for (surface,last) in [(&mut a,false),(&mut b,true)] {
            for row in &mut surface.control_points {
                let start=if last {row.len()-4} else {0};
                let q=&row[start..start+4];let mut controls=vec![vec![0.;3];4];
                for k in 0..3 {
                    controls[0][k]=(q[0][k]+4.*q[1][k]+q[2][k])/6.;
                    controls[1][k]=(2.*q[1][k]+q[2][k])/3.;
                    controls[2][k]=(q[1][k]+2.*q[2][k])/3.;
                    controls[3][k]=(q[1][k]+4.*q[2][k]+q[3][k])/6.;
                }
                *row=controls;
            }
            for row in &mut surface.weights {*row=vec![row[0];4];}
        }
        assert!(
            crate::continuity::inspect_surface_exact_strip_jets(
                &b, &a, "vMax", "vMin", 2, 1., 1000000
            )
            .unwrap()
            .certified
        );
        a.control_points[0][1][1] += 1e-6;
        assert!(
            !crate::continuity::inspect_surface_exact_strip_jets(
                &b, &a, "vMax", "vMin", 2, 1., 1000000
            )
            .unwrap()
            .exact_identity
        );
        assert_eq!(
            before,
            value_codec::to_string(&value_codec::json!([p, path, scale])).unwrap()
        );
        let tight = checked_profile_sweep(&p, &path, &scale, [1., 0., 0.], 17, 0.001).unwrap();
        assert_eq!(tight["report"]["accepted"], false);
        assert!(tight["surface"].is_null());
        let bad = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        assert!(checked_profile_sweep(&p, &path, &bad, [1., 0., 0.], 17, 1.).is_err());
    }
}
