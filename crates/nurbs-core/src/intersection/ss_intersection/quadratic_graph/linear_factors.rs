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
        let delta = sign * slope;
        let upper = work.add(1., -center[0])?;
        let mut ids = [0; 2];
        let mut endpoints = [[0.; 2]; 2];
        for (end, t) in [-center[0], upper].into_iter().enumerate() {
            let displacement = work.mul(delta, t)?;
            let v = work.add(center[1], displacement)?;
            // Clip only when the U-boundary point leaves the V interval. An
            // unused V-boundary quotient may be non-dyadic even though both
            // actual clipped endpoints are exactly representable.
            let point = if !(0. ..=1.).contains(&v) {
                let boundary = if v < 0. { 0. } else { 1. };
                let numerator = work.add(boundary, -center[1])?;
                let clipped = work.div(numerator, delta)?;
                [work.add(center[0], clipped)?, boundary]
            } else {
                [work.add(center[0], t)?, v]
            };
            check(
                point.iter().all(|v| (0. ..=1.).contains(v)),
                "Exact factor endpoint leaves original graph rectangle",
            )?;
            ids[end] = data.vertices.len();
            endpoints[end] = point;
            data.vertices.push(OriginalGraphSSVertex {
                point: OriginalGraphSSPoint::Normalized(point),
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
        let value = work.add(data.center[axis], sign * radius)?;
        if !(0. ..=1.).contains(&value) {
            continue;
        }
        let mut endpoints = [[0.; 2]; 2];
        for end in 0..2 {
            endpoints[end][axis] = value;
            endpoints[end][1 - axis] = end as f64;
        }
        segment(data, endpoints, 1);
        data.kind = QuadraticGraphSSKind::ParallelLineFactors;
    }
    Ok(())
}
