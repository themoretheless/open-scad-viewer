//! Typed indexed primitive expansion, independent of authored values and transport.
#[derive(Debug)]
pub enum FaceEvent {
    OutOfBounds {
        face: usize,
        entry: usize,
        index: f64,
    },
    InvalidPoint {
        index: usize,
    },
    IndexLimit {
        actual: usize,
    },
    PathLimit {
        actual: usize,
    },
}
#[derive(Debug)]
pub struct FaceExpansion<const D: usize = 3> {
    pub polygons: Vec<Vec<[f64; D]>>,
    pub events: Vec<FaceEvent>,
    pub aborted: bool,
    pub empty: bool,
}
pub fn unsigned_index(value: Option<f64>) -> f64 {
    let Some(value) = value else {
        return 0.;
    };
    if !value.is_finite() {
        return 9007199254740991.;
    }
    let value = value.trunc();
    if value < 0. || value > 9007199254740991. {
        9007199254740991.
    } else if value == 0. {
        0.
    } else {
        value
    }
}
fn point3(value: &[Option<f64>]) -> Option<[f64; 3]> {
    if value.len() != 2 && value.len() != 3 {
        return None;
    }
    let mut point = [0.; 3];
    for (index, value) in value.iter().enumerate() {
        point[index] = value.filter(|v| v.is_finite())?;
    }
    Some(point)
}
pub fn expand_faces(
    points: &[Vec<Option<f64>>],
    faces: &[Vec<Option<f64>>],
    maximum_indices: usize,
) -> FaceExpansion {
    let points: Vec<Option<[f64; 3]>> = points.iter().map(|p| point3(p)).collect();
    expand_indexed(&points, faces, maximum_indices)
}
fn expand_indexed<const D: usize>(
    points: &[Option<[f64; D]>],
    faces: &[Vec<Option<f64>>],
    maximum_indices: usize,
) -> FaceExpansion<D> {
    let mut polygons = Vec::new();
    let mut events = Vec::new();
    let mut count = 0usize;
    let mut aborted = false;
    'faces: for (face, entries) in faces.iter().enumerate() {
        polygons.push(Vec::new());
        let polygon = polygons.last_mut().unwrap();
        for (entry, value) in entries.iter().enumerate() {
            count += 1;
            if count > maximum_indices {
                events.push(FaceEvent::IndexLimit { actual: count });
                aborted = true;
                break 'faces;
            }
            let index = unsigned_index(*value);
            if index >= points.len() as f64 {
                events.push(FaceEvent::OutOfBounds { face, entry, index });
                continue;
            }
            let index = index as usize;
            let Some(point) = points[index] else {
                events.push(FaceEvent::InvalidPoint { index });
                aborted = true;
                break 'faces;
            };
            polygon.push(point);
        }
    }
    let empty = polygons.iter().all(|p| p.len() < 3);
    FaceExpansion {
        polygons,
        events,
        aborted,
        empty,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_faces_keep_order_and_stop_at_the_first_invalid_point() {
        let points = vec![vec![Some(0.), Some(0.)], vec![Some(1.), None]];
        let faces = vec![vec![Some(0.), Some(99.), Some(1.), Some(0.)]];
        let result = expand_faces(&points, &faces, 10);
        assert!(result.aborted);
        assert_eq!(result.polygons, vec![vec![[0.; 3]]]);
        assert!(matches!(
            result.events[0],
            FaceEvent::OutOfBounds { entry: 1, .. }
        ));
        assert!(matches!(
            result.events[1],
            FaceEvent::InvalidPoint { index: 1 }
        ));
        let result = expand_faces(&points, &faces, 1);
        assert!(matches!(
            result.events[0],
            FaceEvent::IndexLimit { actual: 2 }
        ));
    }
}

#[derive(Debug)]
pub struct PolygonExpansion {
    pub points: Vec<[f64; 2]>,
    pub outlines: Vec<Vec<[f64; 2]>>,
    pub events: Vec<FaceEvent>,
    pub implicit: bool,
    pub aborted: bool,
    pub empty: bool,
}
pub fn expand_polygon(
    authored_points: &[Vec<Option<f64>>],
    paths: &[Vec<Option<f64>>],
    path_count: usize,
    maximum_paths: usize,
    maximum_indices: usize,
) -> PolygonExpansion {
    let mut points = Vec::new();
    for (index, point) in authored_points.iter().enumerate() {
        let converted = if let [Some(x), Some(y)] = point.as_slice() {
            if x.is_infinite() || y.is_infinite() {
                None
            } else {
                Some([*x, *y])
            }
        } else {
            None
        };
        let Some(point) = converted else {
            return PolygonExpansion {
                points,
                outlines: vec![],
                events: vec![FaceEvent::InvalidPoint { index }],
                implicit: true,
                aborted: true,
                empty: true,
            };
        };
        points.push(point);
    }
    let implicit = path_count == 0 && points.len() > 2;
    if path_count > maximum_paths {
        return PolygonExpansion {
            points,
            outlines: vec![],
            events: vec![FaceEvent::PathLimit { actual: path_count }],
            implicit,
            aborted: true,
            empty: true,
        };
    }
    if implicit {
        return PolygonExpansion {
            outlines: vec![points.clone()],
            points,
            events: vec![],
            implicit: true,
            aborted: false,
            empty: false,
        };
    }
    let available: Vec<Option<[f64; 2]>> = points.iter().copied().map(Some).collect();
    let expanded = expand_indexed(&available, paths, maximum_indices);
    PolygonExpansion {
        points,
        outlines: expanded.polygons,
        events: expanded.events,
        implicit: false,
        aborted: expanded.aborted,
        empty: expanded.empty,
    }
}

#[cfg(test)]
mod polygon_tests {
    use super::*;
    #[test]
    fn polygon_nan_is_numeric_and_point_failure_precedes_path_limit() {
        let points = vec![
            vec![Some(f64::NAN), Some(0.)],
            vec![Some(1.), Some(0.)],
            vec![Some(0.), Some(1.)],
        ];
        let result = expand_polygon(&points, &[], 0, 0, 0);
        assert!(result.implicit);
        assert!(!result.empty);
        assert!(result.points[0][0].is_nan());
        let result = expand_polygon(&[vec![None, Some(0.)]], &[], 10, 0, 0);
        assert!(matches!(
            result.events[0],
            FaceEvent::InvalidPoint { index: 0 }
        ));
        let result = expand_polygon(&points, &[], 10, 0, 0);
        assert!(matches!(
            result.events[0],
            FaceEvent::PathLimit { actual: 10 }
        ));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AuthoredLimit {
    Missing,
    Number(f64),
    Invalid,
}
#[derive(Debug)]
pub struct IndexedPolicy {
    pub limits: [f64; 3],
    pub convexity: f64,
}
/// Limits are admitted in points/faces/indices order; omitted limits remain unbounded.
pub fn indexed_policy(
    limits: [AuthoredLimit; 3],
    convexity: Option<f64>,
) -> Result<IndexedPolicy, &'static str> {
    let names = ["maximumPoints", "maximumFacesOrPaths", "maximumIndices"];
    let mut normalized = [f64::INFINITY; 3];
    for (index, value) in limits.into_iter().enumerate() {
        match value {
            AuthoredLimit::Missing => {}
            AuthoredLimit::Number(v)
                if v.is_finite() && v >= 0. && v <= 9007199254740991. && v.fract() == 0. =>
            {
                normalized[index] = v
            }
            _ => return Err(names[index]),
        }
    }
    let convexity = convexity
        .filter(|v| v.is_finite())
        .map(|v| v.trunc().max(1.))
        .unwrap_or(1.);
    Ok(IndexedPolicy {
        limits: normalized,
        convexity,
    })
}
#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn ordered_admission_and_convexity_default() {
        let p = indexed_policy([AuthoredLimit::Missing; 3], Some(f64::NAN)).unwrap();
        assert!(p.limits[0].is_infinite());
        assert_eq!(p.convexity, 1.);
        assert_eq!(
            indexed_policy([AuthoredLimit::Invalid; 3], None).unwrap_err(),
            "maximumPoints"
        );
        assert_eq!(
            indexed_policy(
                [
                    AuthoredLimit::Missing,
                    AuthoredLimit::Number(-1.),
                    AuthoredLimit::Invalid
                ],
                None
            )
            .unwrap_err(),
            "maximumFacesOrPaths"
        );
        assert_eq!(
            indexed_policy([AuthoredLimit::Number(0.); 3], Some(2.9))
                .unwrap()
                .convexity,
            2.
        );
    }
}
