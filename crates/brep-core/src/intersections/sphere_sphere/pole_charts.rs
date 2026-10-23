//! Nonrational UV lifts stay exact as constrained source-surface pullbacks.
//! The constraints describe sets, including empty chart subsets; they do not
//! pretend to enumerate rational arc components or authorize topology changes.
use super::*;
use nurbs_core::surface::Surface;
#[derive(Clone, Debug)]
pub struct WorldCircleClip {
    pub center: [f64; 3],
    pub cosine: [f64; 3],
    pub sine: [f64; 3],
    pub intervals: Vec<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct ImplicitSphereSection {
    pub source_surface: Surface,
    pub normal: [f64; 3],
    pub point: [f64; 3],
    pub clip: Option<WorldCircleClip>,
    pub recognition_error: f64,
}
/// The legacy recognizer remains stereo-only for Boolean UV consumers.
pub fn recognize_any(model: &Model) -> Result<Option<CanonicalSphere>> {
    if let Some(sphere) = recognize(model)? {
        return Ok(Some(sphere));
    }
    recognize_poles(model)
}
fn recognize_poles(model: &Model) -> Result<Option<CanonicalSphere>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 8
        || model.vertices.len() != 6
    {
        return Ok(None);
    }
    let first = model.faces[0]
        .surface
        .control_points
        .last()
        .and_then(|r| r.first());
    let Some(first) = first else { return Ok(None) };
    let Some(opposite) = model.vertices.iter().max_by(|a, b| {
        let d = |p: [f64; 3]| (0..3).map(|k| (p[k] - first[k]).powi(2)).sum::<f64>();
        d(a.point).total_cmp(&d(b.point))
    }) else {
        return Ok(None);
    };
    let center = std::array::from_fn(|k| first[k] * 0.5 + opposite.point[k] * 0.5);
    let d = sub(first.as_slice().try_into().unwrap(), center);
    let radius = d[0].hypot(d[1]).hypot(d[2]);
    if !radius.is_finite() || !(1e-5..=1e6).contains(&radius) {
        return Ok(None);
    }
    let mut error = 0_f64;
    let mut frames = Vec::new();
    let mut surfaces = Vec::new();
    let w = [1., 1., 2.];
    for (id, face) in model.faces.iter().enumerate() {
        let s = &face.surface;
        if s.degree_u != 2
            || s.degree_v != 2
            || s.periodic_u
            || s.periodic_v
            || s.knots_u != [0., 0., 0., 1., 1., 1.]
            || s.knots_v != s.knots_u
            || s.control_points.len() != 3
            || s.control_points.iter().any(|r| r.len() != 3)
            || !face.holes.is_empty()
            || model.loops[face.outer].coedges.len() != 4
            || crate::face_injectivity::declared_pole(model, id).is_none()
        {
            return Ok(None);
        }
        let expected_sides = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [1., 1.]),
            ([1., 1.], [0., 1.]),
            ([0., 1.], [0., 0.]),
        ];
        if !expected_sides.iter().all(|&(a, b)| {
            model.loops[face.outer]
                .coedges
                .iter()
                .any(|e| axis_line(&e.pcurve, a, b))
        }) {
            return Ok(None);
        }
        let direction = |i: usize, j: usize| {
            std::array::from_fn(|k| (s.control_points[i][j][k] - center[k]) / radius)
        };
        let (a, b, pole) = (direction(0, 0), direction(0, 2), direction(2, 0));
        if [(a, b), (a, pole), (b, pole)]
            .iter()
            .any(|&(a, b)| dot(a, b).abs() > RECOGNITION)
            || [a, b, pole]
                .iter()
                .any(|d| (dot(*d, *d) - 1.).abs() > RECOGNITION)
        {
            return Ok(None);
        }
        for i in 0..3 {
            for j in 0..3 {
                if s.weights[i][j] != w[i] * w[j] {
                    return Ok(None);
                }
                let longitude = match j {
                    0 => a,
                    1 => std::array::from_fn(|k| a[k] + b[k]),
                    _ => b,
                };
                let expected: [f64; 3] = std::array::from_fn(|k| {
                    center[k]
                        + radius * ([1., 1., 0.][i] * longitude[k] + [0., 1., 1.][i] * pole[k])
                });
                let d: [f64; 3] = std::array::from_fn(|k| s.control_points[i][j][k] - expected[k]);
                let deviation = d[0].hypot(d[1]).hypot(d[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * radius + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
        frames.push(PatchFrame { a, b, pole });
        surfaces.push(s.clone());
    }
    frames::finish(center, radius, error, frames, Some(surfaces))
}
pub(super) fn sections(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    point: [f64; 3],
    clip: Option<WorldCircleClip>,
) -> Vec<SpherePatchCircle> {
    sphere
        .pole_surfaces
        .as_ref()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(patch, s)| SpherePatchCircle {
            patch,
            arcs: Vec::new(),
            implicit: Some(ImplicitSphereSection {
                source_surface: s.clone(),
                normal,
                point,
                clip: clip.clone(),
                recognition_error: sphere.error,
            }),
        })
        .collect()
}
pub(super) fn clipped(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    point: [f64; 3],
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<SpherePatchCircle> {
    let intervals = arcs
        .iter()
        .filter(|(a, b)| a.is_finite() && b.is_finite() && a < b)
        .map(|&(a, b)| [a, b])
        .collect::<Vec<_>>();
    if intervals.is_empty() {
        return Vec::new();
    }
    let clip = WorldCircleClip {
        center: point,
        cosine: sub(point_at(0.), point),
        sine: sub(point_at(std::f64::consts::FRAC_PI_2), point),
        intervals,
    };
    sections(sphere, normal, point, Some(clip))
}
pub(super) fn invert(
    sphere: &CanonicalSphere,
    frame: &PatchFrame,
    point: [f64; 3],
) -> Option<[f64; 2]> {
    let d = sub(point, sphere.center);
    let (x, y, z) = (dot(d, frame.a), dot(d, frame.b), dot(d, frame.pole));
    let band = sphere.error + 64. * f64::EPSILON * (sphere.radius + 1.);
    if x < -band || y < -band || z < -band {
        return None;
    }
    let radial = x.hypot(y);
    // The complete pole row has no unique inverse longitude.
    if radial == 0. || radial + x <= 0. {
        return None;
    }
    let uv = [z / (sphere.radius + radial), y / (radial + x)];
    uv.iter()
        .all(|v| v.is_finite() && (-1e-9..=1. + 1e-9).contains(v))
        .then_some(uv)
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for ImplicitSphereSection {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"representation":"implicit-surface-plane-pullback","sourceSurface":self.source_surface,
            "normal":self.normal,"point":self.point,"domain":[0.,1.,0.,1.],"recognitionErrorUpper":self.recognition_error,
            "arcComponentsEnumerated":false,"permitsTopologyChange":false,
            "worldClip":self.clip.as_ref().map(|c|value_codec::json!({"center":c.center,"cosine":c.cosine,"sine":c.sine,"intervals":c.intervals}))})
    }
}
