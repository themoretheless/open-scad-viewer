use super::*;
use crate::analysis::{
    cloud_normals::orient_normals_consistent,
    cloud_segmentation::{SegmentationOptions, segment_cloud},
    ransac_primitives::{PrimitiveParams, RansacOptions, detect_primitives},
};

fn parameters(value: PrimitiveParams) -> Value {
    match value {
        PrimitiveParams::Plane { normal, offset } => json!({"normal":normal,"offsetMm":offset}),
        PrimitiveParams::Sphere { center, radius } => json!({"centerMm":center,"radiusMm":radius}),
        PrimitiveParams::Cylinder { point_on_axis, direction, radius } => {
            json!({"axisPointMm":point_on_axis,"axisDirection":direction,"radiusMm":radius})
        }
        PrimitiveParams::Cone { vertex, axis, half_angle } => {
            json!({"vertexMm":vertex,"axisDirection":axis,"halfAngleRad":half_angle})
        }
    }
}

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let points: Vec<[f64; 3]> = field(v, "points")?;
    let normals: Vec<[f64; 3]> = field(v, "normals")?;
    check((4..=4096).contains(&points.len()), "Cloud diagnostics require 4..4096 sites")?;
    match op {
        "detect_cloud_primitives" => {
            let kind = field::<String>(v, "primitive_kind")?;
            let all = kind == "all";
            let options = RansacOptions {
                threshold: field(v, "tolerance_mm")?,
                min_inliers: field(v, "min_inliers")?,
                max_iterations: field(v, "max_iterations")?,
                max_primitives: field(v, "max_primitives")?,
                detect_plane: all || kind == "plane",
                detect_sphere: all || kind == "sphere",
                detect_cylinder: all || kind == "cylinder",
                detect_cone: all || kind == "cone",
                ..Default::default()
            };
            check((1..=16).contains(&options.max_primitives)
                && (1..=1000).contains(&options.max_iterations),
                "Cloud primitive inspection exceeds the editor work limit")?;
            let found = detect_primitives(&points, (!normals.is_empty()).then_some(normals.as_slice()), &options)?;
            let primitives: Vec<_> = found.into_iter().map(|item| {
                json!({"kind":format!("{:?}",item.kind),"parameters":parameters(item.params),
                    "inlierIndices":item.inlier_indices,"rmsMm":item.rms})
            }).collect();
            Ok(json!({"siteCount":points.len(),"primitives":primitives,"sampled":true}))
        }
        "orient_cloud_normals" => {
            let result = orient_normals_consistent(&points, &normals)?;
            Ok(json!({"normals":result.normals,"flipped":result.flipped,
                "components":result.components,"sampled":true}))
        }
        "segment_point_cloud" => {
            let options = SegmentationOptions {
                normal_angle_threshold: field::<f64>(v, "normal_angle_deg")?.to_radians(),
                curvature_threshold: field(v, "curvature_threshold")?,
                min_region_size: field(v, "min_region_size")?,
                neighbor_count: field(v, "neighbor_count")?,
            };
            check(options.neighbor_count <= 64, "At most 64 neighbors per cloud site")?;
            let regions: Vec<_> = segment_cloud(&points, &normals, options)?.into_iter().map(|region| {
                json!({"kind":format!("{:?}",region.approx_kind),"pointIndices":region.point_indices,
                    "meanNormal":region.mean_normal})
            }).collect();
            Ok(json!({"siteCount":points.len(),"regions":regions,"sampled":true}))
        }
        _ => Err(input("Unknown cloud diagnostic")),
    }
}
