//! Exact linear factors and complete clipping in the ORIGINAL graph rectangle.
use super::*;
fn exact_sqrt(value: f64, work: &mut Work) -> Result<f64> {
    work.tick()?;
    let root = value.sqrt();
    check(
        root.is_finite() && root > 0.,
        "Need a positive finite factor root",
    )?;
    check(
        work.mul(root, root)? == value,
        "Factor root is not exactly representable",
    )?;
    Ok(root)
}
pub(super) fn crossing(data: &mut Data, work: &mut Work) -> Result<()> {
    let [a, b, k] = data.diagonal;
    check(
        k == 0.,
        "Indefinite graph requires an exact zero-level factorization",
    )?;
    check(
        data.center.iter().all(|c| 0. < *c && *c < 1.),
        "Crossing node must lie inside both original patches",
    )?;
    let ratio = work.div(a.abs(), b.abs())?;
    let slope = exact_sqrt(ratio, work)?;
    let center = data.center;
    data.kind = QuadraticGraphSSKind::CrossingLines;
    data.vertices.push(OriginalGraphSSVertex {
        point: OriginalGraphSSPoint::Normalized(center),
        singular: true,
    });
    for sign in [1., -1.] {
        let direction = line_clip::vector(data, [1., sign * slope], work)?;
        let endpoints = line_clip::clip(data.center, direction, work)?
            .ok_or_else(|| crate::input("Interior crossing factor lost its original rectangle"))?;
        check(
            endpoints[0] != endpoints[1],
            "Crossing factor clipped to an isolated boundary point",
        )?;
        let mut ids = [0; 2];
        for (end, point) in endpoints.iter().enumerate() {
            ids[end] = data.vertices.len();
            data.vertices.push(OriginalGraphSSVertex {
                point: OriginalGraphSSPoint::Normalized(*point),
                singular: false,
            });
        }
        let edge = data.edges.len();
        for (vertices, points) in [
            ([ids[0], 0], [endpoints[0], center]),
            ([0, ids[1]], [center, endpoints[1]]),
        ] {
            data.edges.push(OriginalGraphSSEdge {
                vertices,
                geometry: OriginalGraphSSEdgeGeometry::Segment(points),
                multiplicity: 1,
            });
        }
        data.branches.push(OriginalGraphSSBranch {
            edges: vec![edge, edge + 1],
            closed: false,
        });
    }
    Ok(())
}
pub(super) fn parallel(data: &mut Data, work: &mut Work) -> Result<()> {
    let axis = usize::from(data.diagonal[0] == 0.);
    let square = work.div(-data.diagonal[2], data.diagonal[axis])?;
    let radius = exact_sqrt(square, work)?;
    for sign in [-1., 1.] {
        let mut fixed = [0.; 2];
        fixed[axis] = sign * radius;
        let displacement = line_clip::vector(data, fixed, work)?;
        let mut anchor = [0.; 2];
        for i in 0..2 {
            anchor[i] = work.add(data.center[i], displacement[i])?;
        }
        let mut free = [0.; 2];
        free[1 - axis] = 1.;
        let direction = line_clip::vector(data, free, work)?;
        let Some(endpoints) = line_clip::clip(anchor, direction, work)? else {
            continue;
        };
        check(
            endpoints[0] != endpoints[1],
            "Simple parallel factor touches only an original rectangle corner",
        )?;
        segment(data, endpoints, 1);
        data.kind = QuadraticGraphSSKind::ParallelLineFactors;
    }
    Ok(())
}
