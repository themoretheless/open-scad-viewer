//! Independent actual-surface radius probes and convex source halfspace covers.
use brep_core::{Model, face_domain};
pub fn qualify(
    model: &Model,
    strips: [usize; 4],
    radii: [f64; 4],
    dir: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    use nurbs_core::interval_eval::Interval as I;
    let mut admitted = true;
    let mut probes = Vec::new();
    let mut material = Vec::new();
    for arm in 0..4 {
        for v in [0.125, 0.5, 0.875] {
            let s = &model.faces[strips[arm]].surface;
            let p = s.evaluate(0., v)?.point;
            let shape = nurbs_core::surface_shape_operator::at(
                s,
                [0., v],
                [nurbs_core::curve_differential::Side::Automatic; 2],
            )?;
            let tensor = shape.tensor.ok_or("Actual arm curvature unavailable")?;
            let k = if arm % 2 == 0 { 0 } else { 1 };
            let a = tensor[k][k];
            let lo = if a[0] > 0. { a[0] } else { -a[1] };
            let hi = a[0].abs().max(a[1].abs());
            if lo <= 0. {
                return Err("Mid-section normal curvature includes zero".into());
            }
            let r = I::point(1.).div(I::new(lo, hi)?)?;
            let good = r.lo >= radii[arm] - 1e-7 && r.hi <= radii[arm] + 1e-7;
            admitted &= good;
            probes.push(value_codec::json!({"arm":arm,"face":strips[arm],"uv":[0.,v],"point":p,"requestedRadius":radii[arm],"actualRadiusInterval":[r.lo,r.hi],"withinTolerance":good}));
        }
    }
    for check in 0..5 {
        let mapped = if check == 0 {
            model.clone()
        } else {
            let sx = if (check - 1) / 2 == 0 { -1. } else { 1. };
            let sy = if (check - 1) % 2 == 0 { -1. } else { 1. };
            brep_core::transform::affine(
                model,
                [
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [-sx, -sy, -1., 19.],
                    [0., 0., 0., 1.],
                ],
            )?
        };
        let target = [
            [-19.0000001, 19.0000001],
            [-19.0000001, 19.0000001],
            [-1e-7, if check == 0 { 19.0000001 } else { 76.0000001 }],
        ];
        for face in 0..model.faces.len() {
            let r = face_domain::inspect_box_containment(
                &mapped, face, target, 1e-8, 50_000, 1_000_000,
            )?;
            admitted &= r.decision == face_domain::BoxDecision::Within;
            material.push(value_codec::json!({"constraint":check,"face":face,"decision":format!("{:?}",r.decision),"cells":r.cells,"domainCells":r.domain_cells}));
        }
    }
    std::fs::write(
        dir.join("material-and-radius.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"qualified":admitted,"sourcePyramidHeight":19,"sourceStockVolumeMm3":2.*19f64.powi(3)/3.,"boundaryInConvexStock":material.iter().all(|x|x["decision"].as_str()==Some("Within")),"constraints":"Z>=0, and 19-Z-sx*X-sy*Y>=0 for sx,sy in {-1,+1}; all complete retained faces. Whole-body embedding is qualified separately.","fields":material,"independentNativeRadiusProbes":probes,"radiusScope":"Three original-surface tensor probes per arm independently check the constructor's analytic extruded midline-radius law. G2 profiles vary in curvature towards the planes; these are not constant-radius circular arcs."}),
        )?,
    )?;
    Ok(admitted)
}
