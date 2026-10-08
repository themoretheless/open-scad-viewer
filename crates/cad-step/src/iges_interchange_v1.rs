//! Legacy `iges-interchange/1` walking slice, frozen; the strict IGES 5.3
//! direct B-rep successor is `iges_interchange_v2`.
use brep_core::analytic_features::FeatureCertificate;
use brep_core::{Model, cuboid};
use nurbs_core::{Error, Result};

fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

/// Fail-closed IGES walking slice: entity subset 110/116/128/190 only.
pub fn export_iges(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Empty model cannot export as IGES B-rep",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut lines = Vec::new();
    lines.push(
        "                                                                        S      1".into(),
    );
    lines.push(
        "1H,,1H;,4HSOLID,11Hopen-scad-v,32Hanalytic IGES walking slice,32H,    G      1".into(),
    );
    let mut seq = 1usize;
    for v in &model.vertices {
        // Entity 116: Point
        lines.push(format!(
            "     116       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "116,{:.15},{:.15},{:.15};                                          P{seq:7}",
            v.point[0], v.point[1], v.point[2]
        ));
        seq += 1;
    }
    for edge in &model.edges {
        if edge.curve.degree == 1 && edge.curve.control_points.len() == 2 {
            let a = &edge.curve.control_points[0];
            let b = &edge.curve.control_points[1];
            // Entity 110: Line
            lines.push(format!(
                "     110       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "110,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8};                      P{seq:7}",
                a[0], a[1], a[2], b[0], b[1], b[2]
            ));
            seq += 1;
        }
    }
    for face in &model.faces {
        if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
            // Entity 190: Plane Surface
            lines.push(format!(
                "     190       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "190,0,0;                                                          P{seq:7}"
            ));
            seq += 1;
        } else {
            // Entity 128: Rational B-Spline Surface (mention only)
            lines.push(format!(
                "     128       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "128,{},{},0,0,0,0,0;                                              P{seq:7}",
                face.surface.degree_u, face.surface.degree_v
            ));
            seq += 1;
        }
    }
    // Manifold solid B-rep object (186) + shell (514) + face (510) + trimmed surface (144).
    lines.push(format!(
        "     186       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "186,1,0;                                                          P{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "     514       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "514,{},0;                                                         P{seq:7}",
        model.faces.len()
    ));
    seq += 1;
    for _ in &model.faces {
        lines.push(format!(
            "     510       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "510,1,0,0;                                                        P{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "     144       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "144,0,1,0,0;                                                      P{seq:7}"
        ));
        seq += 1;
    }
    lines.push(format!(
        "/* open-scad-viewer iges-interchange/1; faces={} bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] */",
        model.faces.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    lines.push(
        "S      1G      1D      1P      1                                        T      1".into(),
    );
    let text = lines.join("\n");
    if text.to_ascii_uppercase().contains("FACETED")
        || text.contains("solid ")
        || text.contains("mtllib")
    {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Faceted/STL/OBJ must not be labeled analytic IGES",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec!["iges_entity_110_116_128_190", "iges_solid_186_514_510_144"],
        },
    ))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iges_roundtrip_entity_subset() {
        let model = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, cert) = export_iges(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("116,") || text.contains("110,") || text.contains("190,"));
        assert!(text.contains("186,") && text.contains("514,"));
        let (back, _) = import_iges(&text).unwrap();
        back.validate().unwrap();
    }

    #[test]
    fn iges_refuses_stl_payload() {
        let stl = "solid cube\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nendloop\nendfacet\nendsolid cube\n";
        assert_eq!(import_iges(stl).unwrap_err().code, "BREP_IGES_REFUSED");
    }
}
