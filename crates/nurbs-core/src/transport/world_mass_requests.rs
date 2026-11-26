use super::*;
use crate::{
    analysis::{exact_planar_face_mass as mass, planar_trim_projection as wire},
    curve::Curve,
    surface::Surface,
};
use num_rational::BigRational;

fn matrix(m: [[BigRational; 3]; 3]) -> [[String; 3]; 3] {
    m.map(|row| row.map(|q| q.to_string()))
}

fn spectrum(m: mass::ExactPrincipalPlaneMoments) -> Value {
    json!({"midpoint":m.midpoint.to_string(),"halfGapSquared":m.half_gap_squared.to_string(),
        "areaScaleSquared":m.area_scale_squared.to_string(),"isotropic":m.is_isotropic()})
}

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let surface: Surface = field(v, "source")?;
    let loops: Vec<Vec<Curve>> = field(v, "loops")?;
    let budget: u64 = field(v, "max_work")?;
    let r = wire::project_coplanar_trim_loops(&[surface], &[loops], budget)?;
    let wire::PlanarTrimOutcome::Projected(faces) = r.outcome else {
        return Err(input(format!(
            "World mass projection refused: {:?}",
            r.outcome
        )));
    };
    let remaining = budget - r.work_used;
    if remaining == 0 {
        return Err(input("World mass work limit reached"));
    }
    let (mut report, used) = match op {
        "world_face_mass" => {
            let r = mass::exact_projected_face_mass(&faces[0], remaining)?;
            let mass::ExactWorldPlaneMassOutcome::Mass(m) = r.outcome else {
                return Err(input(format!("World mass refused: {:?}", r.outcome)));
            };
            (
                json!({"areaScaleSquared":m.area_scale_squared.to_string(),
                "signedAreaCoefficient":m.signed_area.to_string(),"firstMomentCoefficients":m.first_moments.map(|x|x.to_string()),
                "centroid":m.centroid.map(|p|p.map(|x|x.to_string())),
                "secondMomentCoefficients":matrix(m.second_moments),"centroidalSecondMomentCoefficients":m.centroidal_second_moments.map(matrix)}),
                r.work_used,
            )
        }
        "principal_face_moments" => {
            let r = mass::exact_projected_face_principal_moments(&faces[0], remaining)?;
            let mass::ExactPrincipalPlaneOutcome::Moments(m) = r.outcome else {
                return Err(input(format!("Principal moments refused: {:?}", r.outcome)));
            };
            (spectrum(m), r.work_used)
        }
        _ => {
            let r = mass::exact_projected_face_principal_directions(&faces[0], remaining)?;
            let mass::ExactPrincipalDirectionOutcome::Directions(d) = r.outcome else {
                return Err(input(format!(
                    "Principal directions refused: {:?}",
                    r.outcome
                )));
            };
            let mut report = spectrum(d.moments);
            report["planeProjector"] = json!(matrix(d.plane_projector));
            report["deviator"] = json!(d.deviator.map(matrix));
            (report, r.work_used)
        }
    };
    report["workUsed"] = json!(r.work_used + used);
    Ok(report)
}
