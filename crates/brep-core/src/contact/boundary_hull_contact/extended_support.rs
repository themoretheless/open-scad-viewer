//! Exact axis-aligned hull contact restricted to common authored topology.
//! Only used after exact edge/lift agreement, simple trims and chart injectivity.
use crate::Model;
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub hull_intersection: [[f64; 2]; 3],
    pub edges: Vec<usize>,
    pub vertex: Option<usize>,
}
fn edge_ends(model: &Model, edge: usize) -> Option<[[f64; 3]; 2]> {
    let c = &model.edges[edge].curve;
    let n = c.control_points.len();
    if c.periodic
        || c.control_points.iter().any(|p| p.len() != 3)
        || c.knots[..=c.degree].iter().any(|k| *k != c.knots[c.degree])
        || c.knots[n..].iter().any(|k| *k != c.knots[n])
    {
        return None;
    }
    Some([
        std::array::from_fn(|k| c.control_points[0][k]),
        std::array::from_fn(|k| c.control_points[n - 1][k]),
    ])
}
// Bounded floating-point proposals for a strict supporting plane. The caller
// verifies the resulting binary64 anchors exactly against every original pole.
fn propose_vertex_plane(point: [f64; 3], poles: &[Vec<&Vec<f64>>; 2]) -> Option<[Vec<f64>; 2]> {
    let mut directions = Vec::with_capacity(poles[0].len() + poles[1].len());
    for (face, ps) in poles.iter().enumerate() {
        for p in ps {
            if p.as_slice() == point {
                continue;
            }
            let mut d: [f64; 3] = std::array::from_fn(|k| p[k] - point[k]);
            let length = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            if !length.is_finite() || length <= 0. {
                return None;
            }
            for x in &mut d {
                *x /= length;
                if face == 1 {
                    *x = -*x;
                }
            }
            directions.push(d);
        }
    }
    let mut normal = [0.; 3];
    for _ in 0..256 {
        let mut settled = true;
        for d in &directions {
            let dot = (0..3).map(|k| normal[k] * d[k]).sum::<f64>();
            if dot < 1. {
                settled = false;
                for k in 0..3 {
                    normal[k] += (1. - dot) * d[k];
                }
            }
        }
        if settled {
            break;
        }
    }
    let axis = (0..3).max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))?;
    if !normal[axis].is_finite() || normal[axis] == 0. {
        return None;
    }
    let free = match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    let anchors = std::array::from_fn(|i| {
        let mut p = point.to_vec();
        p[free[i]] += 1.;
        p[axis] -= normal[free[i]] / normal[axis];
        p
    });
    anchors
        .iter()
        .flatten()
        .all(|x| x.is_finite())
        .then_some(anchors)
}
// For a positive-weight single Bézier rectangle, all basis factors are
// positive in its interior. A supporting-plane equality can therefore use
// only a whole supported edge or an isolated supported corner. Interior
// zero-distance poles alone do not create additional surface contacts.
struct BezierSupport<'a> {
    poles: Vec<&'a Vec<f64>>,
    boundaries: Vec<&'static str>,
    corners_covered: bool,
    interior: bool,
}
fn bezier_support<'a>(
    s: &'a nurbs_core::surface::Surface,
    planes: &[(usize, f64)],
) -> Option<BezierSupport<'a>> {
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if s.periodic_u
        || s.periodic_v
        || nu != s.degree_u + 1
        || nv != s.degree_v + 1
        || s.weights
            .iter()
            .flatten()
            .any(|&w| !w.is_finite() || w <= 0.)
        || s.knots_u.len() != 2 * nu
        || s.knots_u[..nu].iter().any(|&k| k != s.knots_u[s.degree_u])
        || s.knots_u[nu..].iter().any(|&k| k != s.knots_u[nu])
        || s.knots_v.len() != 2 * nv
        || s.knots_v[..nv].iter().any(|&k| k != s.knots_v[s.degree_v])
        || s.knots_v[nv..].iter().any(|&k| k != s.knots_v[nv])
    {
        return None;
    }
    let on = |u: usize, v: usize| {
        planes
            .iter()
            .all(|&(k, value)| s.control_points[u][v][k] == value)
    };
    let interior = (0..nu).all(|u| (0..nv).all(|v| on(u, v)));
    let supported = [
        (0..nv).all(|v| on(0, v)),
        (0..nv).all(|v| on(nu - 1, v)),
        (0..nu).all(|u| on(u, 0)),
        (0..nu).all(|u| on(u, nv - 1)),
    ];
    let labels = ["uMin", "uMax", "vMin", "vMax"];
    let mut poles = Vec::with_capacity(nu * nv);
    let mut corners_covered = true;
    for u in 0..nu {
        for v in 0..nv {
            let boundary = supported[0] && u == 0
                || supported[1] && u == nu - 1
                || supported[2] && v == 0
                || supported[3] && v == nv - 1;
            let corner = (u == 0 || u == nu - 1) && (v == 0 || v == nv - 1) && on(u, v);
            if corner && !boundary {
                corners_covered = false;
            }
            if interior || boundary || corner {
                poles.push(&s.control_points[u][v]);
            }
        }
    }
    Some(BezierSupport {
        poles,
        boundaries: (0..4)
            .filter(|&i| supported[i])
            .map(|i| labels[i])
            .collect(),
        corners_covered,
        interior,
    })
}
/// Input must be structurally valid. Does not establish its prerequisites.
pub(crate) fn certify(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    let edges = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    });
    let hulls = faces.map(|f| {
        let mut h = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for p in model.faces[f].surface.control_points.iter().flatten() {
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
    for i in 0..2 {
        let planes = (0..3)
            .filter(|&k| {
                intersection[k][0] == intersection[k][1]
                    && (hulls[i][k][0] == intersection[k][0]
                        || hulls[i][k][1] == intersection[k][0])
            })
            .map(|k| (k, intersection[k][0]))
            .collect::<Vec<_>>();
        if planes.is_empty() {
            continue;
        }
        if let Some(support) = bezier_support(&model.faces[faces[i]].surface, &planes) {
            if !support.interior && support.corners_covered && !support.boundaries.is_empty() {
                let f = &model.faces[faces[i]];
                let mut shared = Vec::new();
                for label in &support.boundaries {
                    let edge = std::iter::once(f.outer)
                        .chain(f.holes.iter().copied())
                        .flat_map(|wire| model.loops[wire].coedges.iter())
                        .find(|c| {
                            edges[1 - i].contains(&c.edge)
                                && edge_ends(model, c.edge).is_some_and(|ends| {
                                    (0..2).all(|j| {
                                        ends[j]
                                            == model.vertices[model.edges[c.edge].vertices[j]].point
                                    })
                                })
                                && crate::sweep_smoothness::boundary(
                                    &f.surface,
                                    &c.pcurve,
                                    crate::sweep_smoothness::Axis::Profile,
                                )
                                .or_else(|| {
                                    crate::sweep_smoothness::boundary(
                                        &f.surface,
                                        &c.pcurve,
                                        crate::sweep_smoothness::Axis::Station,
                                    )
                                }) == Some(*label)
                        })
                        .map(|c| c.edge);
                    if let Some(edge) = edge {
                        shared.push(edge);
                    } else {
                        shared.clear();
                        break;
                    }
                }
                if !shared.is_empty() {
                    return Some(Certificate {
                        faces,
                        hull_intersection: intersection,
                        edges: shared,
                        vertex: None,
                    });
                }
            }
        }
    }
    // At an exact supporting plane, a positive rational convex combination
    // can use only poles on that plane. Intersecting several such planes
    // restricts support simultaneously, even where AABBs overlap along a line.
    // No tolerance or arithmetic displacement is used to select poles.
    for _ in 0..3 {
        let restricted = std::array::from_fn::<_, 2, _>(|i| {
            let f = faces[i];
            let support = (0..3)
                .filter(|&k| {
                    intersection[k][0] == intersection[k][1]
                        && (hulls[i][k][0] == intersection[k][0]
                            || hulls[i][k][1] == intersection[k][0])
                })
                .collect::<Vec<_>>();
            let mut h = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            let planes = support
                .iter()
                .map(|&k| (k, intersection[k][0]))
                .collect::<Vec<_>>();
            let points = bezier_support(&model.faces[f].surface, &planes)
                .map(|s| s.poles)
                .unwrap_or_else(|| {
                    model.faces[f]
                        .surface
                        .control_points
                        .iter()
                        .flatten()
                        .filter(|p| support.iter().all(|&k| p[k] == intersection[k][0]))
                        .collect()
                });
            for p in points {
                for k in 0..3 {
                    h[k][0] = h[k][0].min(p[k]);
                    h[k][1] = h[k][1].max(p[k]);
                }
            }
            h
        });
        let next = std::array::from_fn(|k| {
            [
                intersection[k][0]
                    .max(restricted[0][k][0])
                    .max(restricted[1][k][0]),
                intersection[k][1]
                    .min(restricted[0][k][1])
                    .min(restricted[1][k][1]),
            ]
        });
        if next.iter().any(|r| r[0] > r[1]) {
            return None;
        }
        if next == intersection {
            break;
        }
        intersection = next;
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
        // Generated/axis proposals keep the same fixed iteration and predicate limits.
        // Larger supported nets are checked against every original pole.
        if poles.iter().all(|ps| ps.len() <= 64) {
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
                // Quintic station walls have eighteen poles. For larger nets
                // retain only the bounded generated/axis proposals: enumerating
                // every pole pair would multiply exact predicate work.
                if poles.iter().all(|ps| ps.len() <= 16) {
                    candidates.extend(poles.iter().flat_map(|ps| ps.iter().map(|p| (*p).clone())));
                }
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
                                hull_intersection: point.map(|x| [x, x]),
                                edges: Vec::new(),
                                vertex: Some(vertex),
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
                // Generated/axis proposals keep the same fixed iteration and predicate limits.
                // Larger supported nets are checked against every original pole.
                if poles.iter().all(|ps| ps.len() <= 64) {
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
                                hull_intersection: point.map(|x| [x, x]),
                                edges: Vec::new(),
                                vertex: Some(vertex),
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
        hull_intersection: intersection,
        edges: Vec::new(),
        vertex: None,
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

#[cfg(test)]
#[path = "extended_support/tests/mod.rs"]
mod tests;

#[cfg(test)]
mod g2_support_regressions {
    use super::*;
    #[test]
    fn quintic_tangent_contact_is_confined_to_the_actual_shared_arc() {
        let built=crate::analytic::g2_annular::cylinder(8.,4.,16.,0.5,1e-8).unwrap();
        let source=built.model;
        let cap=certify(&source,[0,28]).expect("G2 corner meets its planar annulus only at its shared circular boundary");
        assert_eq!(cap.edges.len(),1);assert!(cap.vertex.is_none());
        let point=certify(&source,[0,5]).expect("G2 corner and neighboring wall quadrant share only their authored vertex");
        assert!(point.vertex.is_some());
        let mut crossed=source.clone();crossed.faces[0].surface.control_points[1][3][2]=-0.125;
        assert!(certify(&crossed,[0,28]).is_none(),"A pole crossing the support plane invalidates the sufficient certificate");
        let mut partial=source.clone();let wire=partial.faces[0].outer;
        partial.loops[wire].coedges[0].pcurve.control_points[1][0]=0.5;
        assert!(certify(&partial,[0,28]).is_none(),"A partial pcurve cannot claim the full supported boundary");
        let mut moved=source.clone();let vertex=moved.edges[cap.edges[0]].vertices[0];moved.vertices[vertex].point[0]=moved.vertices[vertex].point[0].next_up();
        assert!(certify(&moved,[0,28]).is_none(),"The supported shared edge must keep exact authored endpoints");
    }
}
