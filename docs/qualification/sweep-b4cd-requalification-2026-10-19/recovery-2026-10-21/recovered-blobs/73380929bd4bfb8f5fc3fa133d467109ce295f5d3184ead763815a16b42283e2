//! Topological BRep targets; tessellation density does not define feature edges.
use crate::{Model, Result};
use geometry_ops::keypoints::{
    Geometry, Interval, Kind, Segment, dot, length, sampled_circle_center,
};
use geometry_ops::keypoints::{cross, sub};
type P = [f64; 3];
fn point(p: &[f64]) -> Result<P> {
    p.try_into().map_err(|_| {
        nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Keypoints require three-dimensional geometry",
        )
    })
}
fn support(points: &[P]) -> Option<(P, P)> {
    let origin = *points.first()?;
    let mut normal = None;
    for i in 1..points.len() {
        for j in i + 1..points.len() {
            if normal.is_some() {
                break;
            }
            let n = cross(sub(points[i], origin), sub(points[j], origin));
            let l = length(n);
            if l > 1e-10 {
                normal = Some(n.map(|v| v / l));
            }
        }
        if normal.is_some() {
            break;
        }
    }
    let normal = normal?;
    points
        .iter()
        .all(|p| dot(sub(*p, origin), normal).abs() < 1e-6)
        .then_some((origin, normal))
}
pub fn matches_planar_face(model: &Model, center: P, normal: P) -> bool {
    model.faces.iter().any(|face| {
        face.surface.control_points.iter().flatten().all(|p| {
            point(p).is_ok_and(|p| {
                dot(sub(p, center), normal).abs() < 1e-6_f64.max(model.tolerance_mm * 10.)
            })
        })
    })
}
pub fn geometry(model: &Model) -> Result<Geometry> {
    let supports = model
        .faces
        .iter()
        .map(|face| {
            face.surface.validate()?;
            let points = face
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| point(p))
                .collect::<Result<Vec<_>>>()?;
            Ok(support(&points))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut adjacent = vec![Vec::new(); model.edges.len()];
    for (i, face) in model.faces.iter().enumerate() {
        for id in std::iter::once(&face.outer).chain(&face.holes) {
            let Some(loop_) = model.loops.get(*id) else {
                return Err(nurbs_core::Error::new(
                    "NURBS_INVALID_INPUT",
                    "Invalid keypoint loop",
                ));
            };
            for coedge in &loop_.coedges {
                let Some(faces) = adjacent.get_mut(coedge.edge) else {
                    return Err(nurbs_core::Error::new(
                        "NURBS_INVALID_INPUT",
                        "Invalid keypoint edge",
                    ));
                };
                faces.push(i);
            }
        }
    }
    let mut out = Geometry::default();
    for (index, edge) in model.edges.iter().enumerate() {
        let faces = &adjacent[index];
        if faces.len() >= 2 {
            if let Some((origin, normal)) = supports[faces[0]] {
                if faces.iter().all(|&i| {
                    supports[i].is_some_and(|(o, n)| {
                        (dot(n, normal).abs() - 1.).abs() < 1e-7
                            && dot(sub(o, origin), normal).abs() < 1e-6
                    })
                }) {
                    continue;
                }
            }
        }
        if edge.degenerate {
            continue;
        }
        let curve = &edge.curve;
        curve.validate()?;
        let [a, b] = curve.domain();
        let eval = |t: f64| {
            curve
                .evaluate(a + (b - a) * t)
                .and_then(|e| point(&e.point))
        };
        let first = eval(0.)?;
        let middle = eval(0.5)?;
        let last = eval(1.)?;
        out.add(first, Kind::Vertex);
        out.add(last, Kind::Vertex);
        out.add(middle, Kind::Midpoint);
        if curve.degree == 1 {
            out.segments.push(Segment {
                a: first,
                b: last,
                interval: None,
            });
        } else {
            let samples = (0..17)
                .map(|i| eval(i as f64 / 16.))
                .collect::<Result<Vec<_>>>()?;
            for i in 0..16 {
                out.segments.push(Segment {
                    a: samples[i],
                    b: samples[i + 1],
                    interval: Some(Interval {
                        curve: index,
                        start: a + (b - a) * i as f64 / 16.,
                        end: a + (b - a) * (i + 1) as f64 / 16.,
                    }),
                })
            }
            if curve.degree == 2 && curve.control_points.len() == 3 {
                if let Some(center) = sampled_circle_center(first, middle, last, &samples) {
                    out.add(center, Kind::Center)
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supporting_plane_requires_independent_and_coplanar_controls() {
        let controls = [[2., 3., 4.], [3., 3., 4.], [4., 3., 4.], [2., 5., 4.]];
        let (origin, normal) = support(&controls).unwrap();
        assert_eq!(origin, controls[0]);
        assert_eq!(normal, [0., 0., 1.]);
        assert!(support(&controls[..3]).is_none());
        let mut bent = controls.to_vec();
        bent.push([2., 3., 5.]);
        assert!(support(&bent).is_none());
        assert!(point(&[1., 2.]).is_err());
    }
}
