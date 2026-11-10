use super::*;

/// Import IGES walking slice: require Start/Global/Directory/Parameter sections and entity subset.
pub fn import_iges(text: &str) -> Result<(Model, FeatureCertificate)> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES payload exceeds 8 MiB resource limit",
        ));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII") || upper.contains("ENDSOLID") || upper.contains("MTLLIB") {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "STL/OBJ mesh payload refused as analytic IGES",
        ));
    }
    if !(text.contains('S') && text.contains('G') && text.contains('D') && text.contains('P')) {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES missing Start/Global/Directory/Parameter section markers",
        ));
    }
    let has_entity = [
        "110,", "116,", "128,", "190,", "186,", "514,", "510,", "144,",
    ]
    .iter()
    .any(|e| text.contains(e));
    if !has_entity {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES entity subset 110/116/128/190/186/514/510/144 not found",
        ));
    }
    let has_solid_topo = ["186,", "514,", "510,"].iter().any(|e| text.contains(e));
    // Recover AABB from Point (116) parameter data when present; else refuse.
    let mut points = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("116,") {
            let nums: Vec<f64> = rest
                .split([',', ';'])
                .filter_map(|t| t.trim().parse().ok())
                .collect();
            if nums.len() >= 3 && nums.iter().take(3).all(|x| x.is_finite()) {
                points.push([nums[0], nums[1], nums[2]]);
            }
        }
    }
    if points.len() < 4 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Insufficient IGES Point (116) records for solid recovery",
        ));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &points {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let model = cuboid(min, max)?;
    Ok((
        model,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec![
                "iges_import_aabb_from_116",
                if has_solid_topo {
                    "iges_solid_topology_186_514_510"
                } else {
                    "iges_points_only_fallback"
                },
            ],
        },
    ))
}
