use super::*;

pub(super) fn budget_controls(count: usize) -> Result<()> {
    if count > 256 {
        Err(resource("The result exceeds 256 control points"))
    } else {
        Ok(())
    }
}

pub(super) fn compose_curve_with_piece(curve: &Curve, piece: &LegacyMapPiece) -> Result<Curve> {
    let [a, b] = curve.domain();
    check(
        (piece.range[0] - a).abs() <= 64. * f64::EPSILON
            && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
            || (piece.range[0] >= a && piece.range[1] <= b),
        "Map range must lie in the active curve domain",
    )?;
    let restricted = if (piece.range[0] - a).abs() <= 64. * f64::EPSILON
        && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
    {
        curve.clone()
    } else {
        curve.trim(piece.range[0], piece.range[1])?
    };
    // An affine map can retain every source knot directly. Do not restrict
    // this case to one Bezier span or increase its degree unnecessarily.
    if piece.values.len() == 2 && piece.weights[0] == piece.weights[1] {
        let mut result = restricted;
        for knot in &mut result.knots {
            *knot = piece.domain[0] + (*knot - piece.range[0])
                / (piece.range[1] - piece.range[0]) * (piece.domain[1] - piece.domain[0]);
        }
        result.validate()?;
        return Ok(result);
    }
    if curve.degree.saturating_mul(piece.values.len()-1) > 25 {
        return Err(crate::resource("Composed curve exceeds the degree-25 representation budget"));
    }
    let segments = restricted.decompose()?;
    check(
        segments.len() == 1,
        "Composition requires a single Bézier span after restriction",
    )?;
    let span = segments[0].definition().clone();
    compose_bezier_with_piece(&span, piece)
}

pub(super) fn compose_bezier_with_piece(span: &Curve, piece: &LegacyMapPiece) -> Result<Curve> {
    let dimension = span.control_points[0].len();
    let n = span.degree;
    let source_domain = span.domain();
    // Map values are normalized into the source span's domain. A bounded
    // knot preimage may put a tiny part outside that span; the whole-composition
    // retention gate must cover this polynomial continuation as well.

    let map_p: Vec<f64> = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(value, weight)| {
            ((value - source_domain[0]) / (source_domain[1] - source_domain[0])) * weight
        })
        .collect();
    let map_q = piece.weights.clone();
    // Cleared form uses P^i (Q-P)^{n-i}; for polynomial maps Q≡1 this is φ^i (1-φ)^{n-i}.
    let map_one_minus: Vec<f64> = map_q.iter().zip(&map_p).map(|(q, p)| q - p).collect();
    let mut p_powers = vec![vec![1.]];
    let mut one_powers = vec![vec![1.]];
    for _ in 1..=n {
        p_powers.push(bernstein_product(p_powers.last().unwrap(), &map_p));
        one_powers.push(bernstein_product(
            one_powers.last().unwrap(),
            &map_one_minus,
        ));
    }
    let mut homogeneous = vec![Vec::new(); dimension + 1];
    for axis in 0..=dimension {
        let coeffs: Vec<f64> = if axis == dimension {
            span.weights.clone()
        } else {
            span.control_points
                .iter()
                .zip(&span.weights)
                .map(|(p, w)| p[axis] * w)
                .collect()
        };
        let mut composed = vec![0.; n * (piece.values.len() - 1) + 1];
        for (i, coefficient) in coeffs.iter().enumerate() {
            let term = bernstein_product(&p_powers[i], &one_powers[n - i]);
            for (target, value) in composed.iter_mut().zip(term) {
                *target += value * binomial(n, i) * coefficient;
            }
        }
        homogeneous[axis] = composed;
    }
    let degree = homogeneous[0].len() - 1;
    check(degree <= 25, "Composed degree exceeds 25")?;
    budget_controls(degree + 1)?;
    let weights = homogeneous[dimension].clone();
    numeric(
        weights.iter().all(|w| *w >= 1e-12 && *w <= 1e12),
        "Composed weights left the positive window",
    )?;
    let control_points = (0..=degree)
        .map(|i| {
            (0..dimension)
                .map(|axis| homogeneous[axis][i] / weights[i])
                .collect()
        })
        .collect();
    let domain = piece.domain;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(domain[0], degree + 1)
            .chain(std::iter::repeat_n(domain[1], degree + 1))
            .collect(),
        control_points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

pub(super) fn materialize_pieces(curve: &Curve, pieces: &[LegacyMapPiece]) -> Result<(Curve, usize)> {
    let mut composed = Vec::new();
    let mut degree_growth = 0_usize;
    for piece in pieces {
        let part = compose_curve_with_piece(curve, piece)?;
        degree_growth = degree_growth.max(part.degree.saturating_sub(curve.degree));
        composed.push(part);
    }
    let degree = composed.iter().map(|c| c.degree).max().unwrap();
    let mut composed = composed.into_iter().map(|c| c.elevate(degree)).collect::<Result<Vec<_>>>()?.into_iter();
    let mut result = composed.next().unwrap();
    for mut next in composed {
        let left = result.evaluate(result.domain()[1])?.point;
        let right = next.evaluate(next.domain()[0])?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Composed pieces are not C0 joinable",
        )?;
        // A shared endpoint must have the same homogeneous scale. Replacing
        // only the next endpoint weight changes the entire next rational span.
        check(result.control_points.last() == next.control_points.first(),
            "Composed pieces require identical endpoint controls")?;
        let factor = result.weights.last().unwrap() / next.weights[0];
        check(factor.is_finite() && factor > 0., "Composition weight scale overflow")?;
        for weight in &mut next.weights {
            *weight *= factor;
        }
        next.validate()?;
        let mut knots = result.knots[..result.knots.len() - 1].to_vec();
        knots.extend(next.knots[next.degree + 1..].iter().copied());
        let mut controls = result.control_points.clone();
        controls.extend(next.control_points[1..].iter().cloned());
        let mut weights = result.weights.clone();
        weights.extend(next.weights[1..].iter().copied());
        budget_controls(controls.len())?;
        result = Curve {
            degree: result.degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        result.validate()?;
    }
    Ok((result, degree_growth))
}

pub(super) fn materialize_mapping_tree(
    curve: &Curve,
    mapping: &Value,
    depth: usize,
) -> Result<(Curve, usize, Value)> {
    check(depth <= 8, "Nested composition depth exceeds 8")?;
    if let Some(parts) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !parts.is_empty() && parts.len() <= 8,
            "Nested composition needs 1..8 maps",
        )?;
        let mut current = curve.clone();
        let mut growth = 0_usize;
        let mut children = Vec::new();
        // Evaluation applies authored factors first-to-last. Materializing
        // C(map(u)) pulls them through the curve in the opposite order.
        for part in parts.iter().rev() {
            let (next, part_growth, child) = materialize_mapping_tree(&current, part, depth + 1)?;
            growth = growth.saturating_add(part_growth);
            children.push(child);
            current = next;
        }
        return Ok((
            current,
            growth,
            json!({"kind":"nested_composition","depth":depth,"children":children}),
        ));
    }
    let pieces = map_pieces(mapping)?;
    let (result, growth) = materialize_pieces(curve, &pieces)?;
    Ok((
        result,
        growth,
        json!({"kind":"piecewise","pieceCount":pieces.len()}),
    ))
}

pub fn materialize_reparameterized_curve(
    curve: &Curve,
    mapping: &Value,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    let certificate = certify_reparameterization(mapping, None)?;
    let (source, periodic_cover) = if curve.periodic {
        let [a, b] = curve.domain();
        let mut open = curve.trim(a, b)?;
        open.periodic = false;
        (open, true)
    } else {
        (curve.clone(), false)
    };
    let (mut result, degree_growth, tree) = materialize_mapping_tree(&source, mapping, 0)?;
    if periodic_cover {
        // Close the fundamental cover when endpoints match to restore periodic storage.
        let [a, b] = result.domain();
        let left = result.evaluate(a)?.point;
        let right = result.evaluate(b)?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Periodic composition cover is not closed",
        )?;
        result.periodic = true;
        result.validate()?;
    }
    let tolerance = context(tolerance);
    Ok(json!({"curve":result,"certificate":{
        "version":"nurbs-foundation/5","operation":"materialize-nonlinear-reparameterization",
        "exact":true,"fittedToExactPromotion":false,"degreeGrowth":degree_growth,
        "periodicCoverMaterialized":periodic_cover,
        "compositionTree":tree,
        "controlCount":result.control_points.len(),"resource":{"maxControls":256,"maxDegree":25,"maxNesting":8},
        "method":"homogeneous-Bernstein-clear-denominator-composition",
        "mapCertificate":certificate,"evidence":tolerance_evidence(&tolerance)}}))
}
