use super::*;
pub(crate) fn rigid_frame(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<[[f64; 4]; 4]> {
    let (o, x, y) = axis2(entities, id, 3, scale)?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
pub(crate) fn mul(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}
pub(crate) fn inverse_rigid(m: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.; 4]; 4];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[j][i]
        }
        r[i][3] = -(0..3).map(|j| r[i][j] * m[j][3]).sum::<f64>();
    }
    r[3][3] = 1.;
    r
}
pub(crate) fn cartesian_operator_3d(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    scale: f64,
) -> Result<[[f64; 4]; 4]> {
    let offset = match args.len() {
        6 => 0,
        8 => 2,
        _ => {
            return Err(refuse(
                "CARTESIAN_TRANSFORMATION_OPERATOR_3D inherited/explicit attribute count mismatch",
            ));
        }
    };
    let factor = if matches!(args[4 + offset], Value::Omitted) {
        1.
    } else {
        number(&args[4 + offset], "transformation scale")?
    };
    if (factor - 1.).abs() > 1e-12 {
        return Err(refuse(
            "Non-rigid CARTESIAN_TRANSFORMATION_OPERATOR_3D scale is refused",
        ));
    }
    let axis = |index: usize, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(args[index], Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(&args[index], "transformation axis")?, 3)
        }
    };
    let x = axis(1 + offset, vec![1., 0., 0.])?;
    let y = axis(2 + offset, vec![0., 1., 0.])?;
    let z = axis(5 + offset, vec![0., 0., 1.])?;
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    if dot(&x, &y).abs() > 1e-10
        || dot(&x, &z).abs() > 1e-10
        || dot(&y, &z).abs() > 1e-10
        || cross
            .iter()
            .zip(&z)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
            > 1e-10
    {
        return Err(refuse(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D axes are non-rigid or reflected",
        ));
    }
    let o = point(
        entities,
        one_ref(&args[3 + offset], "transformation local origin")?,
        3,
        scale,
    )?;
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
pub(crate) fn cartesian_operator_affine(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    scale: f64,
    general: bool,
) -> Result<[[f64; 4]; 4]> {
    let parts = components(entities, id)?;
    let base = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR"))?;
    let spatial = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3D")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3D"))?;
    let nonuniform = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        .ok_or_else(|| {
            refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        })?;
    if !matches!(base.len(), 4 | 5) || spatial.len() != 1 || nonuniform.len() != 2 {
        return Err(refuse(
            "Malformed CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM complex",
        ));
    }
    let axis = |value: &Value, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(value, Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(value, "affine transformation axis")?, 3)
        }
    };
    let offset = base.len() - 4;
    let x = axis(&base[offset], vec![1., 0., 0.])?;
    let y = axis(&base[offset + 1], vec![0., 1., 0.])?;
    let z = axis(&spatial[0], vec![0., 0., 1.])?;
    let origin = point(
        entities,
        one_ref(&base[offset + 2], "affine transformation origin")?,
        3,
        scale,
    )?;
    if !general {
        let dot = |a: &[f64], b: &[f64]| {
            a.iter()
                .zip(b)
                .map(|(left, right)| left * right)
                .sum::<f64>()
        };
        let cross = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
        if dot(&x, &y).abs() > 1e-10
            || dot(&x, &z).abs() > 1e-10
            || dot(&y, &z).abs() > 1e-10
            || cross
                .iter()
                .zip(&z)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
                > 1e-10
        {
            return Err(refuse(
                "Affine transformation axes are non-orthogonal or reflected",
            ));
        }
    }
    let scales = [
        if matches!(base[offset + 3], Value::Omitted) {
            1.
        } else {
            number(&base[offset + 3], "scale")?
        },
        if matches!(nonuniform[0], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[0], "scale2")?
        },
        if matches!(nonuniform[1], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[1], "scale3")?
        },
    ];
    if scales
        .iter()
        .any(|value| (!general && *value <= 0.) || *value == 0. || !value.is_finite())
    {
        return Err(refuse(if general {
            "Affine scales must be finite and nonzero"
        } else {
            "Affine scales must be finite and positive"
        }));
    }
    let m = [
        [
            x[0] * scales[0],
            y[0] * scales[1],
            z[0] * scales[2],
            origin[0],
        ],
        [
            x[1] * scales[0],
            y[1] * scales[1],
            z[1] * scales[2],
            origin[1],
        ],
        [
            x[2] * scales[0],
            y[2] * scales[1],
            z[2] * scales[2],
            origin[2],
        ],
        [0., 0., 0., 1.],
    ];
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !determinant.is_finite() || determinant.abs() <= 1e-14 {
        return Err(refuse("Affine transformation is singular"));
    }
    Ok(m)
}
pub(crate) fn reachable_transform(
    entities: &BTreeMap<usize, Entity>,
    linked: &BTreeSet<usize>,
    scale: f64,
    strict: bool,
) -> Result<Option<[[f64; 4]; 4]>> {
    let mut transforms = Vec::new();
    for id in linked {
        let (ty, a) = call(entities, *id)?;
        if ty == "CARTESIAN_TRANSFORMATION_OPERATOR_3D" && strict {
            transforms.push(cartesian_operator_3d(entities, a, scale)?);
            continue;
        }
        if ty.starts_with("CARTESIAN_TRANSFORMATION_OPERATOR") {
            return Err(refuse(
                "Reachable generic transform is refused: reflection/nonuniform/singular transforms are outside /3",
            ));
        }
        if ty == "ITEM_DEFINED_TRANSFORMATION" {
            if a.len() != 4 {
                return Err(refuse(
                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                ));
            }
            let from = rigid_frame(entities, one_ref(&a[2], "transformation source")?, scale)?;
            let to = rigid_frame(entities, one_ref(&a[3], "transformation target")?, scale)?;
            transforms.push(mul(to, inverse_rigid(from)));
            if transforms.len() > 32 {
                return Err(refuse("Nested rigid transformation depth exceeds 32"));
            }
        }
    }
    if transforms.is_empty() {
        return Ok(None);
    }
    if strict && transforms.len() != 1 {
        return Err(refuse(
            "Ambiguous STEP placement: selected representation reaches multiple transforms",
        ));
    }
    let mut total = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    for t in transforms {
        total = mul(t, total);
    }
    Ok(Some(total))
}

fn exact_boundary_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
) -> Result<bool> {
    if s.control_points.len() < 2 || s.control_points[0].len() < 2 {
        return Ok(false);
    }
    let d2 = pc.domain();
    let a = pc.evaluate(d2[0])?.point;
    let b = pc.evaluate(d2[1])?.point;
    if a.len() != 2 || b.len() != 2 {
        return Ok(false);
    }
    let du = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let dv = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-10;
    let mut expected = if close(a[0], b[0])
        && (close(a[0], du[0]) || close(a[0], du[1]))
        && ((close(a[1], dv[0]) && close(b[1], dv[1]))
            || (close(a[1], dv[1]) && close(b[1], dv[0])))
    {
        let row = if close(a[0], du[0]) {
            0
        } else {
            s.control_points.len() - 1
        };
        s.control_points[row]
            .iter()
            .zip(&s.weights[row])
            .map(|(point, weight)| (point.clone(), *weight))
            .collect::<Vec<_>>()
    } else if close(a[1], b[1])
        && (close(a[1], dv[0]) || close(a[1], dv[1]))
        && ((close(a[0], du[0]) && close(b[0], du[1]))
            || (close(a[0], du[1]) && close(b[0], du[0])))
    {
        let column = if close(a[1], dv[0]) {
            0
        } else {
            s.control_points[0].len() - 1
        };
        s.control_points
            .iter()
            .zip(&s.weights)
            .map(|(row, weights)| (row[column].clone(), weights[column]))
            .collect::<Vec<_>>()
    } else {
        return Ok(false);
    };
    let increasing = if close(a[0], b[0]) {
        b[1] > a[1]
    } else {
        b[0] > a[0]
    };
    if !increasing {
        expected.reverse()
    }
    if reversed {
        expected.reverse()
    }
    let constant = |points: &[(Vec<f64>, f64)]| {
        points.iter().all(|(point, _)| {
            point
                .iter()
                .zip(&points[0].0)
                .all(|(left, right)| close(*left, *right))
        })
    };
    let actual = c3
        .control_points
        .iter()
        .cloned()
        .zip(c3.weights.iter().copied())
        .collect::<Vec<_>>();
    if constant(&expected) && constant(&actual) {
        if !actual[0]
            .0
            .iter()
            .zip(&expected[0].0)
            .all(|(left, right)| close(*left, *right))
        {
            return Err(refuse("Whole-domain pole identity failed"));
        }
        return Ok(true);
    }
    if actual.len() != expected.len() {
        return Ok(false);
    }
    let scale = actual[0].1 / expected[0].1;
    if actual
        .iter()
        .zip(&expected)
        .any(|((point, weight), (wanted, wanted_weight))| {
            !point
                .iter()
                .zip(wanted)
                .all(|(left, right)| close(*left, *right))
                || !close(*weight, scale * wanted_weight)
        })
    {
        return Err(refuse("Whole-domain rational boundary identity failed"));
    }
    Ok(true)
}

pub(crate) fn verify_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
    whole_domain: bool,
    require_exact: bool,
) -> Result<CoedgeTrim> {
    let d3 = c3.domain();
    let d2 = pc.domain();
    let uv_start = pc.evaluate(d2[0])?.point;
    let uv_end = pc.evaluate(d2[1])?.point;
    let domain_u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let domain_v = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let lift = |uv: &[f64]| -> [i32; 2] {
        let component = |value: f64, domain: [f64; 2], periodic: bool| {
            if periodic {
                ((value - domain[0]) / (domain[1] - domain[0])).floor() as i32
            } else {
                0
            }
        };
        [
            component(uv[0], domain_u, s.periodic_u),
            component(uv[1], domain_v, s.periodic_v),
        ]
    };
    let exact = whole_domain && exact_boundary_correspondence(c3, pc, s, reversed)?;
    if require_exact && !exact {
        return Err(refuse(
            "Collapsed freeform boundary lacks an exact whole-domain rational control identity",
        ));
    }
    for q in 0..=if exact { 0 } else { 8 } {
        let t = q as f64 / 8.;
        let t3 = if reversed {
            d3[1] - t * (d3[1] - d3[0])
        } else {
            d3[0] + t * (d3[1] - d3[0])
        };
        let uv = pc.evaluate(d2[0] + t * (d2[1] - d2[0]))?.point;
        let p = c3.evaluate(t3)?.point;
        let sp = s.evaluate(uv[0], uv[1])?.point;
        let error = (p[0] - sp[0]).hypot(p[1] - sp[1]).hypot(p[2] - sp[2]);
        if !error.is_finite() || error > 1e-5 {
            return Err(refuse(format!(
                "Independent 3D curve↔pcurve correspondence failed ({error:.3e})"
            )));
        }
    }
    let trim = CoedgeTrim {
        curve_parameter: d3,
        pcurve_parameter: d2,
        periodic_lift: [lift(&uv_start), lift(&uv_end)],
    };
    trim.validate()?;
    Ok(trim)
}
