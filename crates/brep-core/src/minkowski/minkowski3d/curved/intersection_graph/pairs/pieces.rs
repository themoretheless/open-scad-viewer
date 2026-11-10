//! Constancy of trim membership between ALL proved boundary events.
use super::*;
fn anchor(
    build: &mut Build<'_>,
    faces: [usize; 2],
    owner: &Physical,
    branch: usize,
    t: f64,
) -> Result<([[f64; 2]; 4], [bool; 2])> {
    let mut point = owner.native.station(branch, t, build.rectangles()?)?;
    build.work.rectangles += point.report().rectangles;
    let domains = [
        crate::face_domain::FaceDomain::new(build.models[0], faces[0], 1e-8)?,
        crate::face_domain::FaceDomain::new(build.models[1], faces[1], 1e-8)?,
    ];
    let mut last = String::new();
    for _ in 0..8 {
        let raw = image(&point);
        let b = owner.canonical(raw);
        let mut labels = [None; 2];
        for side in 0..2 {
            let r = domains[side].classify([b[2 * side], b[2 * side + 1]], build.cells()?)?;
            build.work.cells += r.cells;
            labels[side] = match (r.location, r.winding) {
                (Location::Inside, Some(1)) => Some(true),
                (Location::Outside, Some(0)) => Some(false),
                _ => None,
            };
        }
        last = format!(
            "faces={faces:?} branch={branch} clock={:?} station={t} labels={labels:?} box={b:?}",
            point.free_axis()
        );
        if let [Some(a), Some(c)] = labels {
            return Ok((b, [a, c]));
        }
        let free = axis(point.free_axis());
        // The actual root is already in `b`. Retain a cushion from the old
        // box so an exact coordinate/ulp-wide image does not destroy numerical
        // conditioning. This is only a proposal; the fresh operator must pass.
        let old = point.domain();
        let proposed = std::array::from_fn(|k| {
            if k == free {
                return raw[k];
            }
            let pad = (old[k][1] - old[k][0]) * 0.125;
            [
                (raw[k][0] - pad).max(old[k][0]),
                (raw[k][1] + pad).min(old[k][1]),
            ]
        });
        if (0..4).any(|k| k != free && proposed[k][0] >= proposed[k][1]) {
            break;
        }
        let refined = point.source().certify(
            proposed,
            point.free_axis(),
            ss::FiberLimits {
                max_rectangles: build.rectangles()?,
                preconditioner: None,
            },
        )?;
        build.work.rectangles += refined.report().rectangles;
        let Some(next) = refined.into_fiber() else {
            break;
        };
        point = next;
    }
    Err(fail(&format!(
        "whole root-box trim anchor classification remains unproved: {last}"
    )))
}
pub(super) fn partition(
    build: &mut Build<'_>,
    faces: [usize; 2],
    owner: &Physical,
    events: &[OriginalGraphEvent],
) -> Result<Vec<OriginalGraphPiece>> {
    let mut pieces = Vec::new();
    for (branch, meta) in owner.branches().iter().enumerate() {
        let mut ordered = events
            .iter()
            .filter(|e| e.branch == branch)
            .collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.station[0].total_cmp(&b.station[0]));
        if ordered
            .windows(2)
            .any(|p| p[0].station[1] >= p[1].station[0])
        {
            return Err(fail("trim boundary station order is not strictly proved"));
        }
        let mut nodes = vec![(
            GraphEndpoint::Natural { branch, end: 0 },
            [meta.domain[0]; 2],
        )];
        let mut finish = (
            GraphEndpoint::Natural { branch, end: 1 },
            [meta.domain[1]; 2],
        );
        for e in ordered {
            let node = (GraphEndpoint::Boundary(e.key.clone()), e.station);
            if e.station == [meta.domain[0]; 2] {
                nodes[0] = node;
            } else if e.station == [meta.domain[1]; 2] {
                finish = node;
            } else if meta.domain[0] < e.station[0] && e.station[1] < meta.domain[1] {
                nodes.push(node);
            } else {
                return Err(fail(
                    "trim event is not proved interior or exactly at an original branch endpoint",
                ));
            }
        }
        nodes.push(finish);
        for ends in nodes.windows(2) {
            let lo = ends[0].1[1];
            let hi = ends[1].1[0];
            let t = lo * 0.5 + hi * 0.5;
            if !(lo < t && t < hi) {
                return Err(fail(
                    "no strictly proved branch station between trim events",
                ));
            }
            let (b, inside) = anchor(build, faces, owner, branch, t)?;
            // All boundary queries are complete, both supports injective, and
            // this physical branch is a continuous graph in the same clock.
            // Membership cannot change on the EXACT open interval between
            // ordered algebraic events without another original boundary root.
            // Thus one strict whole-root-box anchor proves the WHOLE interval,
            // including the parts inside the endpoint enclosure bands.
            if inside == [true, true]
                && ends
                    .iter()
                    .any(|n| matches!(n.0, GraphEndpoint::Natural { .. }))
            {
                return Err(fail(
                    "active branch end lacks a proved original trim-edge identity",
                ));
            }
            if inside == [true, true] {
                for end in ends {
                    if let GraphEndpoint::Boundary(key) = &end.0 {
                        if !events.iter().any(|e| e.key == *key && e.inside_other) {
                            return Err(fail(
                                "active arc endpoint is not strictly inside the other original face",
                            ));
                        }
                    }
                }
            }
            pieces.push(OriginalGraphPiece {
                branch,
                endpoints: [ends[0].0.clone(), ends[1].0.clone()],
                anchor: t,
                anchor_box: b,
                inside,
            });
        }
    }
    Ok(pieces)
}
