//! boundary hull contact algorithm: certify_axis.
use super::*;
pub fn certify_axis(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    let edges = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    });
    let mut hulls = faces.map(|f| {
        let mut h = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for p in contact_controls(model, f) {
            for k in 0..3 {
                h[k][0] = h[k][0].min(p[k]);
                h[k][1] = h[k][1].max(p[k]);
            }
        }
        h
    });
    // Extrema are authored binary64 values: comparisons introduce no rounding.
    let mut intersection = std::array::from_fn::<_, 3, _>(|k| {
        [
            hulls[0][k][0].max(hulls[1][k][0]),
            hulls[0][k][1].min(hulls[1][k][1]),
        ]
    });
    if intersection.iter().any(|r| r[0] > r[1]) {
        return None;
    }
    // A positive rational convex combination can reach a supporting plane
    // only through controls lying on that plane. Simultaneous support-plane
    // constraints can isolate a corner even when the initial hulls overlap
    // along a larger interval. Never apply this to a plane cutting a hull.
    let mut previous = 0;
    for _ in 0..3 {
        let supports = (0..3)
            .filter(|&k| {
                intersection[k][0] == intersection[k][1]
                    && hulls
                        .iter()
                        .all(|h| intersection[k][0] == h[k][0] || intersection[k][0] == h[k][1])
            })
            .collect::<Vec<_>>();
        if supports.len() <= previous {
            break;
        }
        previous = supports.len();
        let restricted = faces.map(|f| {
            let mut h = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            for p in contact_controls(model, f) {
                if supports.iter().all(|&k| p[k] == intersection[k][0]) {
                    for k in 0..3 {
                        h[k][0] = h[k][0].min(p[k]);
                        h[k][1] = h[k][1].max(p[k]);
                    }
                }
            }
            h
        });
        intersection = std::array::from_fn(|k| {
            [
                restricted[0][k][0].max(restricted[1][k][0]),
                restricted[0][k][1].min(restricted[1][k][1]),
            ]
        });
        if intersection.iter().any(|r| r[0] > r[1]) {
            return None;
        }
        hulls = restricted;
    }
    let free = (0..3)
        .filter(|&k| intersection[k][0] < intersection[k][1])
        .collect::<Vec<_>>();
    if free.len() == 3 {
        // A 3D supporting plane may expose a single common topological vertex
        // even when all coordinate hull intervals overlap with positive width.
        let vertices = edges.clone().map(|set| {
            set.into_iter()
                .flat_map(|e| model.edges[e].vertices)
                .collect::<BTreeSet<_>>()
        });
        let poles = faces.map(|f| {
            model.faces[f]
                .surface
                .control_points
                .iter()
                .flatten()
                .collect::<Vec<_>>()
        });
        if poles.iter().all(|ps| ps.len() <= 16) {
            for &vertex in vertices[0].intersection(&vertices[1]) {
                let point = model.vertices[vertex].point;
                if !edges.iter().all(|set| {
                    set.iter().any(|&e| {
                        edge_ends(model, e).is_some_and(|ends| {
                            (0..2).any(|i| model.edges[e].vertices[i] == vertex && ends[i] == point)
                        })
                    })
                }) {
                    continue;
                }
                let mut candidates = vec![
                    vec![0., 0., 0.],
                    vec![1., 0., 0.],
                    vec![0., 1., 0.],
                    vec![0., 0., 1.],
                ];
                if let Some(anchors) = propose_vertex_plane(point, &poles) {
                    candidates.extend(anchors);
                }
                candidates.extend(poles.iter().flat_map(|ps| ps.iter().map(|p| (*p).clone())));
                for a in 0..candidates.len() {
                    for b in a + 1..candidates.len() {
                        let sides = poles.each_ref().map(|ps| {
                            let mut side = None;
                            for p in ps {
                                let sign = crate::shared_boundary::orient(
                                    &[&point, &candidates[a], &candidates[b], p],
                                    None,
                                )?;
                                if sign == cad_predicates::Sign::Zero {
                                    if p.as_slice() != point {
                                        return None;
                                    }
                                } else {
                                    if side.is_some_and(|s| s != sign) {
                                        return None;
                                    }
                                    side = Some(sign);
                                }
                            }
                            side
                        });
                        if matches!(
                            sides,
                            [
                                Some(cad_predicates::Sign::Positive),
                                Some(cad_predicates::Sign::Negative)
                            ] | [
                                Some(cad_predicates::Sign::Negative),
                                Some(cad_predicates::Sign::Positive)
                            ]
                        ) {
                            return Some(Certificate {
                                faces,
                                contact_enclosure: point.map(|x| [x, x]),
                                edges: Vec::new(),
                                vertex: Some(vertex),
                                joined_proof: None,
                                source_join: None,
                                polar_join: None,
                            });
                        }
                    }
                }
            }
        }
    }
    if !free.is_empty() && free.len() <= 2 {
        let projections = if free.len() == 2 {
            vec![[free[0], free[1]]]
        } else {
            (0..3)
                .filter(|&k| k != free[0])
                .map(|k| [free[0], k])
                .collect()
        };
        for axes in projections {
            // Shared station support can leave two transverse coordinates free.
            // An exact projected separator may restrict that contact to one
            // authored vertex even when its coordinate AABBs overlap in area.
            let vertices = edges.clone().map(|set| {
                set.into_iter()
                    .flat_map(|e| model.edges[e].vertices)
                    .collect::<BTreeSet<_>>()
            });
            for &vertex in vertices[0].intersection(&vertices[1]) {
                let point = model.vertices[vertex].point;
                if (0..3).any(|k| point[k] < intersection[k][0] || point[k] > intersection[k][1]) {
                    continue;
                }
                let poles = faces.map(|f| {
                    model.faces[f]
                        .surface
                        .control_points
                        .iter()
                        .flatten()
                        .filter(|p| {
                            (0..3).all(|k| {
                                intersection[k][0] != intersection[k][1]
                                    || (hulls[usize::from(f == faces[1])][k][0]
                                        != intersection[k][0]
                                        && hulls[usize::from(f == faces[1])][k][1]
                                            != intersection[k][0])
                                    || p[k] == intersection[k][0]
                            })
                        })
                        .collect::<Vec<_>>()
                });
                let mut candidates = vec![
                    vec![0., 0., 0.],
                    vec![1., 0., 0.],
                    vec![0., 1., 0.],
                    vec![0., 0., 1.],
                ];
                // Coordinate support already restricts the contact to this plane.
                // A rotated corner need not be separated by a world-axis proposal;
                // use bounded candidate generation, then verify all retained
                // source poles with the exact projected orientation predicate.
                if poles.iter().all(|ps| ps.len() <= 16) {
                    let mut projected = [0.; 2];
                    let mut directions = Vec::new();
                    for (face, ps) in poles.iter().enumerate() {
                        for p in ps {
                            let mut d = [p[axes[0]] - point[axes[0]], p[axes[1]] - point[axes[1]]];
                            let length = d[0].hypot(d[1]);
                            if length > 0. && length.is_finite() {
                                for x in &mut d {
                                    *x /= length;
                                    if face == 1 {
                                        *x = -*x;
                                    }
                                }
                                directions.push(d);
                            }
                        }
                    }
                    for _ in 0..256 {
                        for d in &directions {
                            let dot = projected[0] * d[0] + projected[1] * d[1];
                            if dot < 1. {
                                for k in 0..2 {
                                    projected[k] += (1. - dot) * d[k];
                                }
                            }
                        }
                    }
                    let length = projected[0].hypot(projected[1]);
                    if length > 0. && length.is_finite() {
                        let mut anchor = point.to_vec();
                        anchor[axes[0]] -= projected[1] / length;
                        anchor[axes[1]] += projected[0] / length;
                        candidates.push(anchor);
                    }
                }
                for candidate in candidates {
                    let sides = poles.each_ref().map(|ps| {
                        let mut side = None;
                        for p in ps {
                            let sign = crate::shared_boundary::orient(
                                &[&point, &candidate, p],
                                Some(axes),
                            )?;
                            if sign == cad_predicates::Sign::Zero {
                                if p.as_slice() != point {
                                    return None;
                                }
                            } else {
                                if side.is_some_and(|s| s != sign) {
                                    return None;
                                }
                                side = Some(sign);
                            }
                        }
                        side
                    });
                    if matches!(
                        sides,
                        [
                            Some(cad_predicates::Sign::Positive),
                            Some(cad_predicates::Sign::Negative)
                        ] | [
                            Some(cad_predicates::Sign::Negative),
                            Some(cad_predicates::Sign::Positive)
                        ]
                    ) {
                        // Each face must actually own an exact endpoint here.
                        if edges.iter().all(|set| {
                            set.iter().any(|&e| {
                                edge_ends(model, e).is_some_and(|ends| {
                                    (0..2).any(|i| {
                                        model.edges[e].vertices[i] == vertex && ends[i] == point
                                    })
                                })
                            })
                        }) {
                            return Some(Certificate {
                                faces,
                                contact_enclosure: point.map(|x| [x, x]),
                                edges: Vec::new(),
                                vertex: Some(vertex),
                                joined_proof: None,
                                source_join: None,
                                polar_join: None,
                            });
                        }
                    }
                }
            }
        }
    }
    if free.len() > 1 {
        return None;
    }
    let mut out = Certificate {
        faces,
        contact_enclosure: intersection,
        edges: Vec::new(),
        vertex: None,
        joined_proof: None,
        source_join: None,
        polar_join: None,
    };
    if free.is_empty() {
        let point = intersection.map(|r| r[0]);
        // Require the same topological vertex and an exact curve endpoint on
        // both faces. Merely nearby vertex coordinates do not authorize contact.
        let vertices = edges.map(|set| {
            set.into_iter()
                .flat_map(|e| {
                    let ends = edge_ends(model, e);
                    (0..2).filter_map(move |i| {
                        ends.filter(|p| p[i] == point)
                            .map(|_| model.edges[e].vertices[i])
                    })
                })
                .collect::<BTreeSet<_>>()
        });
        out.vertex = vertices[0]
            .intersection(&vertices[1])
            .copied()
            .find(|&v| model.vertices[v].point == point);
        return out.vertex.map(|_| out);
    }
    let axis = free[0];
    let mut segments = Vec::new();
    for &e in edges[0].intersection(&edges[1]) {
        let c = &model.edges[e].curve;
        let Some(ends) = edge_ends(model, e) else {
            continue;
        };
        if c.control_points
            .iter()
            .any(|p| (0..3).any(|k| k != axis && p[k] != intersection[k][0]))
        {
            continue;
        }
        // Continuity covers the whole interval between endpoints. Exact lift
        // agreement places that interval on both authored face boundaries.
        segments.push((
            ends[0][axis].min(ends[1][axis]),
            ends[0][axis].max(ends[1][axis]),
            e,
        ));
    }
    segments.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = intersection[axis][0];
    for (lo, hi, e) in segments {
        if lo > covered {
            break;
        }
        if hi > covered {
            covered = hi;
            out.edges.push(e);
        }
        if covered >= intersection[axis][1] {
            return Some(out);
        }
    }
    None
}
