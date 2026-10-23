use super::*;

/// Homogeneous plane-UV numerator of the conic (x, y) = num / den.
pub(super) fn conic_num_uv(
    plane: &CanonicalPlane,
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    num: [[f64; 3]; 2],
    den: [f64; 3],
) -> [[f64; 3]; 2] {
    let u2 = plane.u_len * plane.u_len;
    let v2 = plane.v_len * plane.v_len;
    let au = dot(e1, plane.u) / u2;
    let bu = dot(e2, plane.u) / u2;
    let av = dot(e1, plane.v) / v2;
    let bv = dot(e2, plane.v) / v2;
    let u_q = dot(sub(q, plane.origin), plane.u) / u2;
    let v_q = dot(sub(q, plane.origin), plane.v) / v2;
    [
        std::array::from_fn(|i| u_q * den[i] + au * num[0][i] + bu * num[1][i]),
        std::array::from_fn(|i| v_q * den[i] + av * num[0][i] + bv * num[1][i]),
    ]
}

impl value_codec::Serialize for PlaneConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"planeUv":plane_uv,
                    "coneUv":cone_uv,"maxSampleResidual":max_sample_residual})
            }
            Self::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"ellipse","curve":curve,"center":center,
                "semiMajor":semi_major,"semiMinor":semi_minor,"major":major,"minor":minor,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Parabola {
                curve,
                vertex,
                direction,
                focal_length,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"parabola","curve":curve,"vertex":vertex,
                "direction":direction,"focalLength":focal_length,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
            Self::Hyperbola {
                curve,
                center,
                semi_transverse,
                semi_conjugate,
                transverse,
                conjugate,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"hyperbola","curve":curve,"center":center,
                "semiTransverse":semi_transverse,"semiConjugate":semi_conjugate,
                "transverse":transverse,"conjugate":conjugate,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
        }
    }
}
