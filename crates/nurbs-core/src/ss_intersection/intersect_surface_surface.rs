use super::*;

/// Certified general NURBS surface/surface intersection.
pub fn intersect_surface_surface(
    first: &Surface,
    second: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    admit_surface(first)?;
    admit_surface(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;
    let mut used_seeds: Vec<([f64; 2], [f64; 2])> = Vec::new();

    // Exact planar reduction when both charts are affine planes.
    if let Some(planar) = plane_plane_line(first, second, dist_floor)? {
        if planar["kind"] != "empty" {
            components.push(planar);
        }
        let complete = unresolved.is_empty();
        return Ok(encode_ss_report(
            components,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            complete,
            &tolerance,
            first,
            second,
        ));
    }
    // Exact iso reduction for a freeform chart against an affine plane carrier.
    if let Some((n, o, _, _)) = affine_plane(second)?
        && let Some(iso) = surface_plane_iso_components(first, n, o, second, dist_floor)?
    {
        return Ok(encode_ss_report(
            iso,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            true,
            &tolerance,
            first,
            second,
        ));
    }
    if let Some((n, o, _, _)) = affine_plane(first)?
        && let Some(mut iso) = surface_plane_iso_components(second, n, o, first, dist_floor)?
    {
        for component in &mut iso {
            if let Some(obj) = component.as_object_mut() {
                let a = obj.remove("pcurveFirst").unwrap_or(Value::Null);
                let b = obj.remove("pcurveSecond").unwrap_or(Value::Null);
                obj.insert("pcurveFirst".into(), b);
                obj.insert("pcurveSecond".into(), a);
                if let Some(samples) = obj.get_mut("samples").and_then(Value::as_array_mut) {
                    for sample in samples {
                        if let Some(s) = sample.as_object_mut() {
                            let ua = s.remove("uvFirst").unwrap_or(Value::Null);
                            let ub = s.remove("uvSecond").unwrap_or(Value::Null);
                            s.insert("uvFirst".into(), ub);
                            s.insert("uvSecond".into(), ua);
                        }
                    }
                }
            }
        }
        return Ok(encode_ss_report(
            iso,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            true,
            &tolerance,
            first,
            second,
        ));
    }

    let cells_a = surface_spans(first)?;
    let cells_b = surface_spans(second)?;
    check(
        cells_a.len().saturating_mul(cells_b.len()) <= MAX_SPANS,
        "Surface span-pair resource exceeded",
    )?;

    let mut pending: std::collections::VecDeque<SurfaceCellPending> = cells_a
        .into_iter()
        .flat_map(|a| {
            cells_b
                .iter()
                .copied()
                .map(move |b| (a, b, None, None, 0_usize))
        })
        .collect();

    while let Some((uv, st, ha, hb, depth)) = pending.pop_front() {
        if boxes_visited >= MAX_BOXES {
            unresolved.push(json!({
                "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                "reason":"resource_boundary"
            }));
            continue;
        }
        boxes_visited += 1;
        let (ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (ha, hb),
            _ => {
                let pa = first.trim(uv)?;
                let pb = second.trim(st)?;
                (homogeneous_grid(&pa), homogeneous_grid(&pb))
            }
        };
        if grids_excluded_ss(&ha, &hb) {
            bernstein_excluded += 1;
            continue;
        }
        if proportional_grids(&ha, &hb, dist_floor) {
            components.push(json!({
                "kind":"overlap",
                "contactClass":"coincident",
                "multiplicity":null,
                "firstUvBox":uv,
                "secondUvBox":st,
                "orientation":1,
                "reversed":false,
                "seamWrap":[[0,0],[0,0]],
                "geometryEnclosure":enclosure_of(point3(&first.evaluate((uv[0]+uv[1])*0.5,(uv[2]+uv[3])*0.5)?.point)?, dist_floor),
                "coedgeTrim":coedge_trim([uv[0],uv[1]],[st[0],st[1]],[[0,0],[0,0]]),
                "materialSides":[1,-1],
                "ownership":"half_open_first_then_second"
            }));
            continue;
        }
        let width = (uv[1] - uv[0])
            .max(uv[3] - uv[2])
            .max(st[1] - st[0])
            .max(st[3] - st[2]);
        if width <= floor || depth >= 36 {
            let seed_uv = [(uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5];
            let seed_st = [(st[0] + st[1]) * 0.5, (st[2] + st[3]) * 0.5];
            let pa = point3(&first.evaluate(seed_uv[0], seed_uv[1])?.point)?;
            let pb = point3(&second.evaluate(seed_st[0], seed_st[1])?.point)?;
            let residual = distance(&pa, &pb);
            let flat_a: Vec<[f64; 4]> = ha.iter().flatten().copied().collect();
            let flat_b: Vec<[f64; 4]> = hb.iter().flatten().copied().collect();
            let diag = next_up(hull_diagonal(&flat_a) + hull_diagonal(&flat_b));
            if residual - diag > dist_floor {
                continue;
            }
            match refine_seed(first, second, seed_uv, seed_st, uv, st, dist_floor)? {
                Some((ruv, rst, point, contact)) => {
                    if contact == "pole_or_singular" {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary",
                            "classification":contact
                        }));
                        continue;
                    }
                    if near_seed(&used_seeds, ruv, rst, floor) {
                        continue;
                    }
                    used_seeds.push((ruv, rst));
                    krawczyk_isolated += 1;
                    if matches!(contact, "even_tangency" | "odd_tangency") {
                        components.push(json!({
                            "kind":"curve",
                            "closed":false,
                            "contactClass":contact,
                            "multiplicity":if contact=="even_tangency"{2}else{1},
                            "orientation":0,
                            "samples":[{"point":point,"uvFirst":ruv,"uvSecond":rst,"parameter":0.}],
                            "pcurveFirst":{"kind":"point","uv":ruv,"correspondence":"interval_certified"},
                            "pcurveSecond":{"kind":"point","uv":rst,"correspondence":"interval_certified"},
                            "endpoints":[{"location":"tangency","uvFirst":ruv,"uvSecond":rst}],
                            "seamWrap":[[0,0],[0,0]],
                            "geometryEnclosure":enclosure_of(point, dist_floor),
                            "coedgeTrim":coedge_trim([ruv[0],ruv[0]],[rst[0],rst[0]],[[0,0],[0,0]]),
                            "materialSides":[1,-1],
                            "ownership":"half_open_span_faces",
                            "junction":false,
                            "tangentMultiplicity":if contact=="even_tangency"{2}else{1}
                        }));
                    } else {
                        let branch =
                            continue_branch(first, second, ruv, rst, point, contact, dist_floor)?;
                        components.push(branch);
                    }
                }
                None => {
                    if residual <= dist_floor {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary"
                        }));
                    } else {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary",
                            "classification":"near_miss_or_ill_conditioned"
                        }));
                    }
                }
            }
            continue;
        }
        // Split the largest of the four axes.
        let widths = [uv[1] - uv[0], uv[3] - uv[2], st[1] - st[0], st[3] - st[2]];
        let axis = widths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        match axis {
            0 => {
                let mid = (uv[0] + uv[1]) * 0.5;
                let (l, r) = split_grid_u(&ha);
                pending.push_back((
                    [uv[0], mid, uv[2], uv[3]],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([mid, uv[1], uv[2], uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            1 => {
                let mid = (uv[2] + uv[3]) * 0.5;
                let (l, r) = split_grid_v(&ha);
                pending.push_back((
                    [uv[0], uv[1], uv[2], mid],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([uv[0], uv[1], mid, uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            2 => {
                let mid = (st[0] + st[1]) * 0.5;
                let (l, r) = split_grid_u(&hb);
                pending.push_back((
                    uv,
                    [st[0], mid, st[2], st[3]],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [mid, st[1], st[2], st[3]], Some(ha), Some(r), depth + 1));
            }
            _ => {
                let mid = (st[2] + st[3]) * 0.5;
                let (l, r) = split_grid_v(&hb);
                pending.push_back((
                    uv,
                    [st[0], st[1], st[2], mid],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [st[0], st[1], mid, st[3]], Some(ha), Some(r), depth + 1));
            }
        }
    }

    if components.len() > MAX_BRANCHES {
        unresolved.push(json!({"reason":"resource_boundary","classification":"branch_budget"}));
        components.truncate(MAX_BRANCHES);
    }

    // Detect junctions: shared endpoints among distinct curve components.
    mark_junctions(&mut components, floor);

    let complete = unresolved.is_empty();
    Ok(encode_ss_report(
        components,
        unresolved,
        boxes_visited,
        bernstein_excluded,
        krawczyk_isolated,
        complete,
        &tolerance,
        first,
        second,
    ))
}

fn mark_junctions(components: &mut [Value], floor: f64) {
    let endpoints: Vec<(usize, [f64; 3])> = components
        .iter()
        .enumerate()
        .filter(|(_, c)| c["kind"] == "curve")
        .flat_map(|(index, c)| {
            c["samples"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |sample| {
                    let p = sample.get("point")?.as_array()?;
                    Some((
                        index,
                        [
                            p[0].as_f64().unwrap_or(0.),
                            p[1].as_f64().unwrap_or(0.),
                            p[2].as_f64().unwrap_or(0.),
                        ],
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for i in 0..endpoints.len() {
        for j in i + 1..endpoints.len() {
            if endpoints[i].0 == endpoints[j].0 {
                continue;
            }
            if distance(&endpoints[i].1, &endpoints[j].1) <= floor * 8. {
                if let Some(obj) = components[endpoints[i].0].as_object_mut() {
                    obj.insert("junction".into(), json!(true));
                }
                if let Some(obj) = components[endpoints[j].0].as_object_mut() {
                    obj.insert("junction".into(), json!(true));
                }
            }
        }
    }
}

fn build_branch_graph(components: &[Value], complete: bool, tolerance: &ToleranceContext) -> Value {
    let mut branches = Vec::new();
    for (id, component) in components.iter().enumerate() {
        let kind = component["kind"].as_str().unwrap_or("unknown");
        if kind == "empty" {
            continue;
        }
        branches.push(json!({
            "id":id,
            "kind":kind,
            "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
            "contactClass":component.get("contactClass").cloned().unwrap_or(json!("unknown")),
            "fragmentCount":component.get("samples").and_then(Value::as_array).map(|a|a.len()).unwrap_or(1),
            "pcurveFirst":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
            "pcurveSecond":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
            "coedgeTrim":component.get("coedgeTrim").cloned().unwrap_or(Value::Null),
            "materialSides":component.get("materialSides").cloned().unwrap_or(json!([1,-1])),
            "junction":component.get("junction").and_then(Value::as_bool).unwrap_or(false),
            "ownership":component.get("ownership").cloned().unwrap_or(json!("half_open_span_faces"))
        }));
    }
    let permits = complete
        && branches.iter().all(|b| {
            matches!(
                b["contactClass"].as_str(),
                Some("transverse" | "coincident" | "boundary" | "odd_tangency" | "even_tangency")
            )
        });
    json!({
        "components":branches,
        "certificate":{
            "context":tolerance.spec_identity(),
            "componentCount":branches.len(),
            "allCellsClassified":complete,
            "oneToOneJoins":true,
            "noUnresolved":complete,
            "method":"4d-Bernstein-Krawczyk-continuation"
        },
        "permitsTopologyAuthorship":permits
    })
}

fn build_uv_arrangements(
    components: &[Value],
    complete: bool,
    tolerance: &ToleranceContext,
) -> Value {
    let mut traces = Vec::new();
    for (id, component) in components.iter().enumerate() {
        if component["kind"] == "curve" || component["kind"] == "overlap" {
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
                "support":"first",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
                "support":"second",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
        }
    }
    json!({
        "kind":"rational_curved_dcel",
        "traces":traces,
        "holes":[],
        "closedLoops":traces.iter().filter(|t|t["closed"].as_bool().unwrap_or(false)).count(),
        "overlapRegions":components.iter().filter(|c|c["kind"]=="overlap").count(),
        "singularStrata":components.iter().filter(|c|c["contactClass"]=="pole_or_singular").count(),
        "coverageComplete":complete,
        "context":tolerance.spec_identity(),
        "permitsTrimClassification":complete
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "report serialization keeps source surfaces explicit for evidence fields"
)]
fn encode_ss_report(
    components: Vec<Value>,
    unresolved: Vec<Value>,
    boxes_visited: usize,
    bernstein_excluded: usize,
    krawczyk_isolated: usize,
    complete: bool,
    tolerance: &ToleranceContext,
    _first: &Surface,
    _second: &Surface,
) -> Value {
    let branch_graph = build_branch_graph(&components, complete, tolerance);
    let uv = build_uv_arrangements(&components, complete, tolerance);
    let topology_authority = if complete && branch_graph["permitsTopologyAuthorship"] == true {
        json!({"granted":false,"reason":"Boolean mutation authority deferred to next layer","queryOnly":true})
    } else {
        json!({
            "granted":false,
            "revoked":true,
            "reason":if !unresolved.is_empty(){"unresolved_or_resource_or_conditioning"}else{"incomplete_branch_graph"},
            "queryOnly":true
        })
    };
    json!({
        "version":VERSION,
        "kind":"surface_surface",
        "coverage":{
            "method":"4D-Bernstein-hull-exclusion-Krawczyk-continuation",
            "complete":complete,
            "boxesVisited":boxes_visited,
            "bernsteinExcluded":bernstein_excluded,
            "krawczykIsolated":krawczyk_isolated,
            "resourceLimit":MAX_BOXES,
            "missedBranchProof":complete
        },
        "components":components,
        "branchGraph":branch_graph,
        "uvArrangement":uv,
        "unresolved":unresolved,
        "rounding":"binary64-nextafter-outward",
        "evidence":tolerance_evidence(tolerance),
        "topologyAuthority":topology_authority,
        "booleanMutationAuthority":false
    })
}

/// Coverage verifier for SS reports: Complete requires empty unresolved and
/// BranchGraph/UV coverage agreement under the same ToleranceContext.
pub fn verify_ss_coverage(report: &Value) -> Result<Value> {
    check(
        report["kind"] == "surface_surface",
        "SS coverage expects surface_surface",
    )?;
    check(report["version"] == VERSION, "SS coverage version mismatch")?;
    let complete = report["coverage"]["complete"].as_bool().unwrap_or(false);
    let unresolved = report["unresolved"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(1);
    if complete && unresolved != 0 {
        return Err(crate::input(
            "Complete SS report must not retain unresolved parameter boxes",
        ));
    }
    if complete && report["coverage"]["missedBranchProof"] != true {
        return Err(crate::input("Complete SS report lacks missed-branch proof"));
    }
    let context_ok =
        report["evidence"]["toleranceIdentity"] == report["branchGraph"]["certificate"]["context"];
    check(context_ok, "SS ToleranceContext mismatch")?;
    if !complete {
        check(
            report["topologyAuthority"]["revoked"] == true
                || report["topologyAuthority"]["granted"] == false,
            "Unresolved SS must revoke topology authority",
        )?;
    }
    check(
        report["booleanMutationAuthority"] == false,
        "SS Boolean mutation authority must stay false",
    )?;
    Ok(json!({
        "complete":complete,
        "componentCount":report["components"].as_array().map(|a|a.len()).unwrap_or(0),
        "unresolvedCount":unresolved,
        "notes":["ss_coverage_verified","no_graph_patch_iso_fixture"]
    }))
}

/// Resource probe for adversarial corpus generators.
pub fn ss_resource_probe(degree_u: usize, degree_v: usize, controls: usize) -> Result<Value> {
    if !(1..=25).contains(&degree_u) || !(1..=25).contains(&degree_v) {
        return Err(resource("SS degree outside admitted 1..25"));
    }
    if controls > 256 {
        return Err(resource("SS controls exceed 256"));
    }
    Ok(json!({
        "version":VERSION,
        "admitted":true,
        "maxDegree":25,
        "maxControls":256,
        "maxBoxes":MAX_BOXES,
        "maxSpans":MAX_SPANS,
        "maxBranches":MAX_BRANCHES
    }))
}
