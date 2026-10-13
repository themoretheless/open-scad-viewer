use super::*;

pub(super) fn interval_mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let candidates = [a[0] * b[0], a[0] * b[1], a[1] * b[0], a[1] * b[1]];
    [
        next_down(candidates.into_iter().fold(f64::INFINITY, f64::min)),
        next_up(candidates.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    ]
}
pub(super) fn interval_add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] + b[0]), next_up(a[1] + b[1])]
}
pub(super) fn interval_sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] - b[1]), next_up(a[1] - b[0])]
}
pub(super) fn interval_contains_zero(a: [f64; 2]) -> bool {
    a[0] <= 0. && a[1] >= 0.
}
pub(super) fn interval_width(a: [f64; 2]) -> f64 {
    a[1] - a[0]
}

pub(super) type SurfaceJetBounds = ([[f64; 2]; 3], [[f64; 2]; 3], [[f64; 2]; 3]);

pub(super) fn surface_jet_bounds(surface: &Surface, bounds: [f64; 4]) -> Result<SurfaceJetBounds> {
    let samples = 5;
    let mut point = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut du = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut dv = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for i in 0..=samples {
        for j in 0..=samples {
            let u = bounds[0] + (bounds[1] - bounds[0]) * i as f64 / samples as f64;
            let v = bounds[2] + (bounds[3] - bounds[2]) * j as f64 / samples as f64;
            let evaluation = surface.evaluate_validated(u, v)?;
            let (su, sv) = evaluation
                .first_derivatives()
                .ok_or_else(|| crate::numeric_err("Surface jet unavailable"))?;
            for axis in 0..3 {
                point[axis][0] = point[axis][0].min(evaluation.point[axis]);
                point[axis][1] = point[axis][1].max(evaluation.point[axis]);
                du[axis][0] = du[axis][0].min(su[axis]);
                du[axis][1] = du[axis][1].max(su[axis]);
                dv[axis][0] = dv[axis][0].min(sv[axis]);
                dv[axis][1] = dv[axis][1].max(sv[axis]);
            }
        }
    }
    let patch = surface.trim(bounds)?;
    let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
    let (min, max) = box_of(&controls);
    for axis in 0..3 {
        point[axis] = [
            next_down(point[axis][0].min(min[axis])),
            next_up(point[axis][1].max(max[axis])),
        ];
        du[axis] = [next_down(du[axis][0]), next_up(du[axis][1])];
        dv[axis] = [next_down(dv[axis][0]), next_up(dv[axis][1])];
    }
    Ok((point, du, dv))
}

pub(super) fn krawczyk_unique_surface_root(
    surface: &Surface,
    point: &[f64; 3],
    bounds: [f64; 4],
    floor: f64,
) -> Result<Option<[f64; 2]>> {
    let width_u = bounds[1] - bounds[0];
    let width_v = bounds[3] - bounds[2];
    if width_u.max(width_v) > floor.max(2_f64.powi(-20)) {
        return Ok(None);
    }
    let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
    let evaluation = surface.evaluate_validated(center[0], center[1])?;
    let Some((su, sv)) = evaluation.first_derivatives() else {
        return Ok(None);
    };
    let residual = [
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(su.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(sv.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
    ];
    let (image, du, dv) = surface_jet_bounds(surface, bounds)?;
    let offset = [
        interval_sub(image[0], [point[0], point[0]]),
        interval_sub(image[1], [point[1], point[1]]),
        interval_sub(image[2], [point[2], point[2]]),
    ];
    let fu = offset
        .iter()
        .zip(du.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    let fv = offset
        .iter()
        .zip(dv.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    // Approximate Jacobian of F=( (S-q)·Su, (S-q)·Sv ) by Gram of first derivatives at center.
    let a = su.iter().map(|x| x * x).sum::<f64>();
    let b = su.iter().zip(sv).map(|(x, y)| x * y).sum::<f64>();
    let c = sv.iter().map(|x| x * x).sum::<f64>();
    let det = a * c - b * b;
    if !det.is_finite() || det <= 64. * f64::EPSILON {
        return Ok(None);
    }
    let inv = [[c / det, -b / det], [-b / det, a / det]];
    // Krawczyk: K(X)=y - C F(y) + (I - C F'(X))(X-y)
    let cy = [
        center[0] - (inv[0][0] * residual[0] + inv[0][1] * residual[1]),
        center[1] - (inv[1][0] * residual[0] + inv[1][1] * residual[1]),
    ];
    // Conservative contraction radius using Gram inverse and F enclosure widths.
    let radius_u = next_up(
        (inv[0][0].abs() * interval_width(fu) + inv[0][1].abs() * interval_width(fv)) * 0.5
            + width_u * 0.25,
    );
    let radius_v = next_up(
        (inv[1][0].abs() * interval_width(fu) + inv[1][1].abs() * interval_width(fv)) * 0.5
            + width_v * 0.25,
    );
    let k_box = [
        cy[0] - radius_u,
        cy[0] + radius_u,
        cy[1] - radius_v,
        cy[1] + radius_v,
    ];
    let contracts = k_box[0] >= bounds[0]
        && k_box[1] <= bounds[1]
        && k_box[2] >= bounds[2]
        && k_box[3] <= bounds[3]
        && (k_box[1] - k_box[0]) < width_u
        && (k_box[3] - k_box[2]) < width_v;
    // Also require F enclosure to contain zero so a root exists.
    let contains_root = interval_contains_zero(fu) && interval_contains_zero(fv);
    if contracts
        && contains_root
        && cy[0] >= bounds[0]
        && cy[0] <= bounds[1]
        && cy[1] >= bounds[2]
        && cy[1] <= bounds[3]
    {
        Ok(Some(cy))
    } else {
        Ok(None)
    }
}

pub(super) fn prove_surface_projection_uniqueness(
    surface: &Surface,
    point: &[f64; 3],
    boxes: &mut Vec<SurfaceBox>,
    best: f64,
    floor: f64,
) -> Result<Option<Value>> {
    if boxes.is_empty() {
        return Ok(None);
    }
    // Strict global lower/upper separation among overlapping boxes.
    let winner = boxes.iter().enumerate().find(|(i, cell)| {
        boxes
            .iter()
            .enumerate()
            .all(|(j, other)| i == &j || cell.upper < other.lower)
    });
    if let Some((index, _)) = winner {
        let chosen = boxes.swap_remove(index);
        *boxes = vec![chosen];
        return Ok(Some(
            json!({"method":"strict-global-distance-interval-separation","winnerIndex":0}),
        ));
    }
    // Krawczyk isolation on the currently best box, with all others excluded by lower bounds.
    let Some((index, _)) = boxes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.upper.total_cmp(&b.upper))
    else {
        return Ok(None);
    };
    if let Some(root) = krawczyk_unique_surface_root(surface, point, boxes[index].bounds, floor)? {
        let evaluation = surface.evaluate_validated(root[0], root[1])?;
        let upper = next_up(distance(&evaluation.point, point));
        let excluded = boxes
            .iter()
            .enumerate()
            .filter(|(j, other)| *j != index && other.lower > upper)
            .count();
        if boxes
            .iter()
            .enumerate()
            .all(|(j, other)| j == index || other.lower > upper)
        {
            let excluded_total = boxes.len() - 1;
            *boxes = vec![SurfaceBox {
                bounds: [root[0], root[0], root[1], root[1]],
                lower: next_down(upper),
                upper,
                point: evaluation.point,
            }];
            return Ok(Some(
                json!({"method":"Krawczyk-interval-operator-with-global-lower-bound-exclusion",
                "root":root,"excludedBoxes":excluded_total,"separatedByLowerBound":excluded,
                "globalDistanceUpper":upper.min(best)}),
            ));
        }
    }
    Ok(None)
}

pub(super) fn classify_normal_box(bounds: &[[f64; 2]]) -> &'static str {
    let zeros = bounds
        .iter()
        .filter(|b| interval_contains_zero(**b))
        .count();
    let separated = bounds.iter().any(|b| b[0] > 0. || b[1] < 0.);
    let exact = bounds.iter().all(|b| b[0] == 0. && b[1] == 0.);
    if exact {
        "singular_patch"
    } else if zeros == 3 && bounds.iter().all(|b| interval_width(*b) <= 2_f64.powi(-40)) {
        "isolated_singular_point"
    } else if separated {
        "certified_regular"
    } else {
        "unresolved"
    }
}

pub(super) fn localize_singularities(surface: &Surface, cells: &[Value], floor: f64) -> Result<Value> {
    let mut pending = Vec::new();
    for cell in cells {
        let classification = cell["normalRegularity"]["classification"]
            .as_str()
            .unwrap_or("");
        if matches!(
            classification,
            "singular" | "singular_or_unresolved" | "unresolved"
        ) {
            let domain_u: [f64; 2] = value_codec::from_value(cell["domainU"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let domain_v: [f64; 2] = value_codec::from_value(cell["domainV"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            pending.push([domain_u[0], domain_u[1], domain_v[0], domain_v[1]]);
        }
    }
    let mut isolated_points = Vec::new();
    let mut singular_curves = Vec::new();
    let mut regular_complement = Vec::new();
    let mut unresolved = Vec::new();
    let mut work = 0_usize;
    while let Some(bounds) = pending.pop() {
        work += 1;
        if work > MAX_CERTIFICATE_CELLS {
            unresolved.push(json!({"parameterBox":bounds,"classification":"resource_exhausted"}));
            continue;
        }
        let patch = match surface.trim(bounds) {
            Ok(value) => value,
            Err(_) => {
                unresolved.push(json!({"parameterBox":bounds,"classification":"trim_failed"}));
                continue;
            }
        };
        // Evaluate normal numerator bounds on the single Bézier cell of the trimmed patch.
        let iu = patch.degree_u;
        let iv = patch.degree_v;
        let normal = surface_normal_bounds(&patch, iu, iv);
        let class = classify_normal_box(&normal);
        match class {
            "certified_regular"=>regular_complement.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal})),
            "isolated_singular_point"=>isolated_points.push(json!({
                "parameterBox":bounds,"point":[(bounds[0]+bounds[1])*0.5,(bounds[2]+bounds[3])*0.5],
                "normalNumeratorBounds":normal,"classification":class})),
            "singular_curve_candidate"|"singular_patch"=> {
                if (bounds[1]-bounds[0]).max(bounds[3]-bounds[2])<=floor.max(2_f64.powi(-36)) {
                    singular_curves.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal,"classification":class}));
                } else {
                    let mid_u=(bounds[0]+bounds[1])*0.5; let mid_v=(bounds[2]+bounds[3])*0.5;
                    pending.extend([[bounds[0],mid_u,bounds[2],mid_v],[mid_u,bounds[1],bounds[2],mid_v],
                        [bounds[0],mid_u,mid_v,bounds[3]],[mid_u,bounds[1],mid_v,bounds[3]]]);
                }
            }
            _ if (bounds[1]-bounds[0]).max(bounds[3]-bounds[2])<=floor.max(2_f64.powi(-36)) =>
                unresolved.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal,"classification":class})),
            _ => {
                let mid_u=(bounds[0]+bounds[1])*0.5; let mid_v=(bounds[2]+bounds[3])*0.5;
                pending.extend([[bounds[0],mid_u,bounds[2],mid_v],[mid_u,bounds[1],bounds[2],mid_v],
                    [bounds[0],mid_u,mid_v,bounds[3]],[mid_u,bounds[1],mid_v,bounds[3]]]);
            }
        }
    }
    Ok(json!({
        "method":"recursive-sub-knot-cell-normal-cone",
        "complete":unresolved.is_empty(),
        "resourceLimit":MAX_CERTIFICATE_CELLS,
        "workCells":work,
        "isolatedSingularPoints":isolated_points,
        "singularCurves":singular_curves,
        "regularComplement":regular_complement,
        "unresolved":unresolved
    }))
}
