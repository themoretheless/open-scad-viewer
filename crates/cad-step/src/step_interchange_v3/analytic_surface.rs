use super::*;

pub(super) fn analytic_surface(
    entities: &BTreeMap<usize, Entity>,
    ty: &str,
    a: &[Value],
    scale: f64,
) -> Result<Surface> {
    let expected = if matches!(ty, "TOROIDAL_SURFACE" | "CONICAL_SURFACE") {
        4
    } else {
        3
    };
    if a.len() != expected {
        return Err(refuse(format!("{ty} argument count mismatch")));
    }
    let (o, x, y) = axis2(
        entities,
        one_ref(&a[1], "analytic surface placement")?,
        3,
        scale,
    )?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let radius = number(&a[2], "analytic surface radius")? * scale;
    if !radius.is_finite() || radius <= 0. {
        return Err(refuse("Analytic surface radius must be positive"));
    }
    let angles = [
        0.,
        std::f64::consts::FRAC_PI_4,
        std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::PI,
        5. * std::f64::consts::FRAC_PI_4,
        3. * std::f64::consts::FRAC_PI_2,
        7. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::TAU,
    ];
    let wu = [
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
    ];
    let ku = vec![
        0.,
        0.,
        0.,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
        std::f64::consts::PI,
        3. * std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_2,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
    ];
    let (v_angles, wv, kv) = if matches!(ty, "SPHERICAL_SURFACE") {
        (
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_4,
                0.,
                std::f64::consts::FRAC_PI_4,
                std::f64::consts::FRAC_PI_2,
            ],
            vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                0.,
                0.,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
            ],
        )
    } else if ty == "TOROIDAL_SURFACE" {
        (angles.to_vec(), wu.to_vec(), ku.clone())
    } else {
        (vec![-1e6, 1e6], vec![1., 1.], vec![-1e6, -1e6, 1e6, 1e6])
    };
    let minor = if ty == "TOROIDAL_SURFACE" {
        let r = number(&a[3], "torus minor radius")? * scale;
        if !(r > 0. && r < radius) {
            return Err(refuse("TOROIDAL_SURFACE requires 0 < minor < major"));
        }
        r
    } else {
        radius
    };
    let semi = if ty == "CONICAL_SURFACE" {
        number(&a[3], "cone semi-angle")?
    } else {
        0.
    };
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    for (iu, &angle) in angles.iter().enumerate() {
        let radial = [
            x[0] * angle.cos() + y[0] * angle.sin(),
            x[1] * angle.cos() + y[1] * angle.sin(),
            x[2] * angle.cos() + y[2] * angle.sin(),
        ];
        let mut row = Vec::new();
        let mut wr = Vec::new();
        for (iv, &v) in v_angles.iter().enumerate() {
            let shoulder_scale = if iu % 2 == 1 { 1. / wu[iu] } else { 1. };
            let meridian_scale =
                if iv % 2 == 1 && matches!(ty, "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE") {
                    1. / wv[iv]
                } else {
                    1.
                };
            let (radial_distance, height) = match ty {
                "CYLINDRICAL_SURFACE" => (radius * shoulder_scale, v),
                "CONICAL_SURFACE" => ((radius + v * semi.tan()) * shoulder_scale, v),
                "SPHERICAL_SURFACE" => (
                    radius * v.cos() * shoulder_scale * meridian_scale,
                    radius * v.sin() * meridian_scale,
                ),
                "TOROIDAL_SURFACE" => (
                    (radius + minor * v.cos() * meridian_scale) * shoulder_scale,
                    minor * v.sin() * meridian_scale,
                ),
                _ => return Err(refuse("Unsupported analytic surface")),
            };
            row.push(
                (0..3)
                    .map(|j| o[j] + radial_distance * radial[j] + height * z[j])
                    .collect(),
            );
            wr.push(wu[iu] * wv[iv]);
        }
        cps.push(row);
        weights.push(wr);
    }
    // Full rational circles are clamped at an explicit seam. Periodic lifts
    // remain carried by CoedgeTrim; marking this net periodic would require
    // repeating `degree` control rows and would change the STEP angle domain.
    let s = Surface {
        degree_u: 2,
        degree_v: if v_angles.len() > 2 { 2 } else { 1 },
        knots_u: ku,
        knots_v: kv,
        control_points: cps,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}
pub(super) fn surface(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if ty == "PLANE" {
        if a.len() != 2 {
            return Err(refuse("PLANE argument count mismatch"));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "PLANE placement")?, 3, scale)?;
        let p = |u: f64, v: f64| {
            (0..3)
                .map(|i| o[i] + u * x[i] + v * y[i])
                .collect::<Vec<_>>()
        };
        const LIMIT: f64 = 1e6;
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            knots_v: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            control_points: vec![
                vec![p(-LIMIT, -LIMIT), p(-LIMIT, LIMIT)],
                vec![p(LIMIT, -LIMIT), p(LIMIT, LIMIT)],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        s.validate()?;
        return Ok(s);
    }
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        return Err(refuse(format!(
            "{ty} periodic/angular parameterization is available only in step-interchange/4"
        )));
    }
    if ty != "B_SPLINE_SURFACE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable surface {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 16 {
        return Err(refuse(
            "B_SPLINE_SURFACE_WITH_KNOTS argument count mismatch",
        ));
    }
    let rows = list(&a[3], "surface control net")?;
    let cps = rows
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let flat = values(&a[10], "surface weights")?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|r| r.len() != nv) || flat.len() != nu * nv {
        return Err(refuse("Surface control/weight shape mismatch"));
    }
    let weights = flat.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: usize_value(&a[1], "surface U degree")?,
        degree_v: usize_value(&a[2], "surface V degree")?,
        knots_u: expand(
            &ints(&a[11], "surface U multiplicities")?,
            &values(&a[13], "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(&a[12], "surface V multiplicities")?,
            &values(&a[14], "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: boolean(&a[6], "surface U closed flag")?,
        periodic_v: boolean(&a[7], "surface V closed flag")?,
    };
    s.validate()?;
    Ok(s)
}
pub(super) fn surface_v4(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        analytic_surface(entities, ty, a, scale)
    } else {
        surface(entities, id, scale)
    }
}
pub(super) fn surface_v5(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_SURFACE_WITH_KNOTS") {
        return surface_v4(entities, id, scale);
    }
    let (du, dv, net, pu, pv, mu, mv, ku, kv, weight_value) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 13 {
            return Err(refuse(
                "AP242 B_SPLINE_SURFACE_WITH_KNOTS requires thirteen inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "surface U degree")?,
            usize_value(&a[2], "surface V degree")?,
            &a[3],
            boolean(&a[5], "surface U closed flag")?,
            boolean(&a[6], "surface V closed flag")?,
            &a[8],
            &a[9],
            &a[10],
            &a[11],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_SURFACE complex lacks B_SPLINE_SURFACE component")
        })?;
        let knots = component(&parts, "B_SPLINE_SURFACE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_SURFACE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("Complex B-spline surface lacks RATIONAL_B_SPLINE_SURFACE component")
        })?;
        if base.len() != 7 || knots.len() != 5 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline surface composition",
            ));
        }
        (
            usize_value(&base[0], "surface U degree")?,
            usize_value(&base[1], "surface V degree")?,
            &base[2],
            boolean(&base[4], "surface U closed flag")?,
            boolean(&base[5], "surface V closed flag")?,
            &knots[0],
            &knots[1],
            &knots[2],
            &knots[3],
            Some(&rational[0]),
        )
    };
    let cps = list(net, "surface control net")?
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|row| row.len() != nv) {
        return Err(refuse("Surface control net is empty or ragged"));
    }
    let weights = if let Some(value) = weight_value {
        let rows = list(value, "surface weights")?;
        if rows.len() != nu {
            return Err(refuse(
                "RATIONAL_B_SPLINE_SURFACE weight row count mismatch",
            ));
        }
        rows.iter()
            .map(|row| values(row, "surface weight row"))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![vec![1.; nv]; nu]
    };
    if weights.iter().any(|row| row.len() != nv) {
        return Err(refuse(
            "RATIONAL_B_SPLINE_SURFACE weight/control-net shape mismatch",
        ));
    }
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: expand(
            &ints(mu, "surface U multiplicities")?,
            &values(ku, "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(mv, "surface V multiplicities")?,
            &values(kv, "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: pu,
        periodic_v: pv,
    };
    s.validate()?;
    Ok(s)
}

pub(super) fn si_scale(args: &[Value]) -> Result<f64> {
    if !args
        .iter()
        .any(|v| matches!(v, Value::Enum(e) if e == "METRE"))
    {
        return Err(refuse("SI unit is not a length unit"));
    }
    let prefix = args.iter().find_map(|v| {
        if let Value::Enum(e) = v {
            (e != "METRE").then_some(e.as_str())
        } else {
            None
        }
    });
    Ok(match prefix {
        None => 1000.,
        Some("DECI") => 100.,
        Some("CENTI") => 10.,
        Some("MILLI") => 1.,
        Some("MICRO") => 1e-3,
        Some("NANO") => 1e-6,
        Some("KILO") => 1e6,
        _ => return Err(refuse("Unsupported SI length prefix")),
    })
}
pub(super) fn unit_scale(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if depth > 8 {
        return Err(refuse("CONVERSION_BASED_UNIT chain depth exceeds eight"));
    }
    if !active.insert(id) {
        return Err(refuse("Cycle in CONVERSION_BASED_UNIT chain"));
    }
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing unit reference"))?;
    let result = match &entity.value {
        Value::List(parts) => {
            let mut answer = None;
            for p in parts {
                if let Value::Call(n, a) = p {
                    if n == "SI_UNIT" {
                        answer = Some(si_scale(a)?)
                    } else if n == "CONVERSION_BASED_UNIT" {
                        answer = Some(conversion_scale(entities, a, depth, active)?)
                    }
                }
            }
            answer.ok_or_else(|| refuse("Complex unit has no admitted length component"))?
        }
        Value::Call(n, a) if n == "SI_UNIT" => si_scale(a)?,
        Value::Call(n, a) if n == "CONVERSION_BASED_UNIT" => {
            conversion_scale(entities, a, depth, active)?
        }
        _ => return Err(refuse("Unsupported length unit entity")),
    };
    active.remove(&id);
    Ok(result)
}
pub(super) fn conversion_scale(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if args.len() < 2 {
        return Err(refuse("CONVERSION_BASED_UNIT argument count mismatch"));
    }
    let factor_id = one_ref(args.last().unwrap(), "CONVERSION_BASED_UNIT factor")?;
    let (ty, a) = call(entities, factor_id)?;
    if !matches!(ty, "LENGTH_MEASURE_WITH_UNIT" | "MEASURE_WITH_UNIT") || a.len() != 2 {
        return Err(refuse("Conversion factor must be LENGTH_MEASURE_WITH_UNIT"));
    }
    let factor = match &a[0] {
        Value::Call(n, v) if n == "LENGTH_MEASURE" && v.len() == 1 => {
            number(&v[0], "LENGTH_MEASURE")?
        }
        v => number(v, "conversion factor")?,
    };
    if !(factor > 0. && factor <= 1e9) {
        return Err(refuse("Conversion factor must be positive and bounded"));
    }
    Ok(factor
        * unit_scale(
            entities,
            one_ref(&a[1], "conversion base unit")?,
            depth + 1,
            active,
        )?)
}
pub(super) fn length_scale(
    entities: &BTreeMap<usize, Entity>,
    reachable: &BTreeSet<usize>,
    allow_orphan_fallback: bool,
) -> Result<f64> {
    let mut unit_ids = Vec::new();
    for id in reachable {
        let entity = &entities[id];
        match &entity.value {
            Value::Call(n, a) if n == "GLOBAL_UNIT_ASSIGNED_CONTEXT" => {
                for v in a {
                    refs(v, &mut unit_ids);
                }
            }
            Value::List(parts) => {
                for part in parts {
                    if let Value::Call(n, a) = part
                        && n == "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                    {
                        for v in a {
                            refs(v, &mut unit_ids);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if unit_ids.is_empty() && allow_orphan_fallback {
        unit_ids.extend(entities.iter().filter_map(|(id, e)| match &e.value {
            Value::List(parts)
                if parts.iter().any(
                    |p| matches!(p,Value::Call(n,_)if n=="SI_UNIT"||n=="CONVERSION_BASED_UNIT"),
                ) =>
            {
                Some(*id)
            }
            Value::Call(n, _) if n == "SI_UNIT" || n == "CONVERSION_BASED_UNIT" => Some(*id),
            _ => None,
        }));
    }
    let mut found = Vec::new();
    for id in unit_ids {
        if let Ok(scale) = unit_scale(entities, id, 0, &mut BTreeSet::new()) {
            found.push(scale);
        }
    }
    if found.is_empty() {
        return Err(refuse("STEP /3 requires an SI length unit context"));
    }
    if found
        .iter()
        .any(|v| !v.is_finite() || (*v - found[0]).abs() > f64::EPSILON)
    {
        return Err(refuse("Unsupported or conflicting STEP length units"));
    }
    Ok(found[0])
}

pub(super) fn part21_views(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut clean = bytes.to_vec();
    let mut structural = bytes.to_vec();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        if !in_string && index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*'
        {
            clean[index] = b' ';
            clean[index + 1] = b' ';
            structural[index] = b' ';
            structural[index + 1] = b' ';
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] != b'\n' {
                    clean[index] = b' ';
                    structural[index] = b' ';
                }
                index += 1;
            }
            if index + 1 < bytes.len() {
                clean[index] = b' ';
                clean[index + 1] = b' ';
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2
            }
            continue;
        }
        if bytes[index] == b'\'' {
            if in_string && index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2;
                continue;
            }
            in_string = !in_string;
            structural[index] = b' ';
            index += 1;
            continue;
        }
        if in_string && bytes[index] != b'\n' {
            structural[index] = b' '
        }
        index += 1;
    }
    (
        String::from_utf8_lossy(&clean).to_ascii_uppercase(),
        String::from_utf8_lossy(&structural).to_ascii_uppercase(),
    )
}
