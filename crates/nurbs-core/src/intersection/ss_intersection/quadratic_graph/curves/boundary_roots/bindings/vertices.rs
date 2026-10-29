//! Exact curve-carrier identities restrict roots to owned source vertices.
use super::*;
fn exact<T>(b: &mut Arithmetic, f: impl FnOnce(&mut Work) -> Result<T>) -> Result<Option<T>> {
    let remaining = (b.max - b.used).min(4096);
    if remaining == 0 {
        return Err(crate::resource(
            "Exact original vertex incidence work exhausted",
        ));
    }
    let mut w = Work::new(remaining)?;
    let result = f(&mut w);
    b.tick(w.used)?;
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.code == crate::RESOURCE_LIMIT => Err(e),
        Err(_) => Ok(None),
    }
}
fn controls(
    census: &QualifiedGraphBoundaryRoots,
    b: &mut Arithmetic,
) -> Result<Option<Vec<[f64; 2]>>> {
    exact(b, |w| {
        census
            .boundary()
            .control_points
            .iter()
            .map(|point| {
                let mut p = [0.; 2];
                for a in 0..2 {
                    let d = census.original().data.domains[census.side()][a];
                    if point[a] == d[0] {
                        p[a] = 0.;
                        continue;
                    }
                    if point[a] == d[1] {
                        p[a] = 1.;
                        continue;
                    }
                    let span = w.add(d[1], -d[0])?;
                    let delta = w.add(point[a], -d[0])?;
                    p[a] = w.div(delta, span)?;
                }
                Ok(p)
            })
            .collect()
    })
}
fn diagonal_controls(
    original: &QualifiedQuadraticGraphSS,
    controls: &[[f64; 2]],
    b: &mut Arithmetic,
) -> Result<Option<Vec<[f64; 2]>>> {
    exact(b, |w| {
        let a = original.data.basis;
        let left = w.mul(a[0][0], a[1][1])?;
        let right = w.mul(a[0][1], a[1][0])?;
        let det = w.add(left, -right)?;
        controls
            .iter()
            .map(|p| {
                let x = w.add(p[0], -original.data.center[0])?;
                let y = w.add(p[1], -original.data.center[1])?;
                let left = w.mul(x, a[1][1])?;
                let right = w.mul(y, a[0][1])?;
                let n0 = w.add(left, -right)?;
                let left = w.mul(y, a[0][0])?;
                let right = w.mul(x, a[1][0])?;
                let n1 = w.add(left, -right)?;
                Ok([w.div(n0, det)?, w.div(n1, det)?])
            })
            .collect()
    })
}
fn location(
    original: &QualifiedQuadraticGraphSS,
    vertex: usize,
    b: &mut Arithmetic,
) -> Result<GraphRootLocation> {
    let mut incidences = Vec::new();
    for (edge, e) in original.edges().iter().enumerate() {
        b.tick(1)?;
        for end in 0..2 {
            if e.vertices[end] == vertex {
                incidences.push(GraphRootEdgeClock {
                    original_edge: edge,
                    original_branch: edge_branch(original, edge)?,
                    clock: [end as f64; 2],
                });
            }
        }
    }
    check(
        !incidences.is_empty(),
        "Original root vertex has no owned edge ports",
    )?;
    Ok(GraphRootLocation::Vertex {
        original_vertex: vertex,
        incidences,
    })
}
pub(super) fn bind(
    census: &QualifiedGraphBoundaryRoots,
    root: usize,
    b: &mut Arithmetic,
) -> Result<Option<GraphRootLocation>> {
    let original = census.original();
    let uv = census.roots()[root].normalized_uv;
    if original.kind() == QuadraticGraphSSKind::ClosedEllipse {
        let Some(controls) = controls(census, b)? else {
            return Ok(None);
        };
        let Some(diag) = diagonal_controls(original, &controls, b)? else {
            return Ok(None);
        };
        for fixed in 0..2 {
            b.tick(1)?;
            if !diag.iter().all(|p| p[fixed] == 0.) {
                continue;
            }
            // Positive rational weights and exact affine control identities
            // place the ENTIRE actual query on one original diagonal carrier.
            // The original conic equation has only its two cardinal vertices
            // on this carrier. Strict sign identifies one algebraic vertex;
            // no rounded radius, parameter representative or point snap exists.
            let ranges = diagonal(
                original,
                [I::new(uv[0][0], uv[0][1])?, I::new(uv[1][0], uv[1][1])?],
                b,
            )?;
            let varying = 1 - fixed;
            let quarter = if ranges[varying].lo > 0. {
                varying
            } else if ranges[varying].hi < 0. {
                varying + 2
            } else {
                continue;
            };
            let vertices: Vec<_> = original
                .vertices()
                .iter()
                .enumerate()
                .filter(|(_, v)| v.point == OriginalGraphSSPoint::EllipseCardinal(quarter))
                .map(|(i, _)| i)
                .collect();
            check(
                vertices.len() == 1,
                "Original cardinal vertex is not uniquely owned",
            )?;
            return Ok(Some(location(original, vertices[0], b)?));
        }
    } else {
        for axis in 0..2 {
            for end in [0., 1.] {
                b.tick(1)?;
                let native = original.data.domains[census.side()][axis][end as usize];
                if !census
                    .boundary()
                    .control_points
                    .iter()
                    .all(|p| p[axis] == native)
                {
                    continue;
                }
                // On an exact natural rectangle rail, the complete original
                // clipped factor graph places every finite root at an owned end
                // vertex. Coincident carriers and singular multiple query roots
                // cannot have passed the prerequisite simple-root census.
                let vertices: Vec<_> = original
                    .vertices()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| {
                        let OriginalGraphSSPoint::Normalized(p) = v.point else {
                            return None;
                        };
                        (p[axis] == end && (0..2).all(|a| uv[a][0] <= p[a] && p[a] <= uv[a][1]))
                            .then_some(i)
                    })
                    .collect();
                if vertices.len() == 1 {
                    return Ok(Some(location(original, vertices[0], b)?));
                }
                check(
                    vertices.len() <= 1,
                    "Distinct original rail vertices remain unresolved",
                )?;
            }
        }
    }
    Ok(None)
}
