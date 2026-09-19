//! Ruled loft with exact bilinear side patches and trimmed planar caps.
use super::*;
type P = [f64; 3];
fn sub(a: P, b: P) -> P {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: P, b: P) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(p: P) -> P {
    let length = p[0].hypot(p[1]).hypot(p[2]);
    p.map(|x| x / length)
}
fn side(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn difference(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn surface(points: Vec<Vec<Vec<f64>>>) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: points,
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn positive_support(coefficients: [f64; 3], remaining: &mut usize) -> Result<()> {
    let failure = || {
        Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Ruled loft correspondence does not bound convex intermediate sections",
        )
    };
    let mut pending = vec![(coefficients, 0u8)];
    while let Some((c, depth)) = pending.pop() {
        *remaining = remaining.checked_sub(1).ok_or_else(|| {
            Error::new(
                "BREP_RESOURCE_LIMIT",
                "Ruled loft support refinement budget exceeded",
            )
        })?;
        if c.iter().any(|x| !x.is_finite()) || c[0] <= 1e-7 || c[2] <= 1e-7 {
            return Err(failure());
        }
        if c[1] > 1e-7 {
            continue;
        }
        if depth >= 24 {
            return Err(Error::new(
                "BREP_UNRESOLVED_LOFT",
                "Ruled loft intermediate convexity remains unresolved",
            ));
        }
        // De Casteljau subdivision tightens the quadratic Bernstein hull without
        // replacing continuous interval admission with a finite sampling heuristic.
        let left = c[0] * 0.5 + c[1] * 0.5;
        let right = c[1] * 0.5 + c[2] * 0.5;
        let middle = left * 0.5 + right * 0.5;
        if middle <= 1e-7 {
            return Err(failure());
        }
        pending.push(([middle, right, c[2]], depth + 1));
        pending.push(([c[0], left, middle], depth + 1));
    }
    Ok(())
}

fn section_frame(section: &[P]) -> Result<(P, P, P, Vec<[f64; 2]>)> {
    let origin = section[0];
    let u = unit(sub(section[1], origin));
    let n = unit(cross(u, sub(section[2], origin)));
    if u.iter().chain(n.iter()).any(|value| !value.is_finite()) {
        return Err(Error::new(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "Section frame is singular",
        ));
    }
    let v = cross(n, u);
    let mut profile = Vec::with_capacity(section.len());
    for point in section {
        let delta = sub(*point, origin);
        if dot(delta, n).abs() > 1e-7 {
            return Err(Error::new(
                "BREP_LOFT_SECTION_TOPOLOGY_REFUSED",
                "Every admitted section must be planar",
            ));
        }
        profile.push([dot(delta, u), dot(delta, v)]);
    }
    let mut sign = 0.;
    for i in 0..profile.len() {
        let a = profile[i];
        let b = profile[(i + 1) % profile.len()];
        let c = profile[(i + 2) % profile.len()];
        let turn = side(difference(b, a), difference(c, b));
        if turn <= 1e-9 {
            return Err(Error::new(
                "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
                "Section must be simple, strictly convex, and consistently indexed",
            ));
        }
        sign += turn;
    }
    if sign <= 1e-8 {
        return Err(Error::new(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "Section has no certified positive area",
        ));
    }
    Ok((origin, u, v, profile))
}

/// Exact bounded multi-section solid with one bilinear rational Bezier patch
/// per corresponding edge and span. Unlike `ruled_loft`, section planes may
/// turn between spans. Every section is a degree-1 rational NURBS loop and all
/// joins share authored edges, so adjacent side patches are exactly C0.
pub(crate) fn piecewise_ruled_loft(sections: &[Vec<P>]) -> Result<Model> {
    if sections.len() < 2 || sections.len() > 16 {
        return Err(Error::new(
            "BREP_LOFT_RESOURCE_REFUSED",
            "Piecewise ruled loft admits 2..16 sections",
        ));
    }
    let count = sections[0].len();
    if !(3..=16).contains(&count) || sections.iter().any(|section| section.len() != count) {
        return Err(Error::new(
            "BREP_LOFT_CORRESPONDENCE_REFUSED",
            "Sections require one exact, equal 3..16 vertex correspondence",
        ));
    }
    if sections
        .iter()
        .flatten()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Loft coordinates must be finite and within 1000000 mm",
        ));
    }
    let frames = sections
        .iter()
        .map(|section| section_frame(section))
        .collect::<Result<Vec<_>>>()?;
    for pair in sections.windows(2) {
        for i in 0..count {
            let j = (i + 1) % count;
            let edge0 = sub(pair[0][j], pair[0][i]);
            let edge1 = sub(pair[1][j], pair[1][i]);
            let ruling0 = sub(pair[1][i], pair[0][i]);
            let ruling1 = sub(pair[1][j], pair[0][j]);
            let bounds = [
                cross(edge0, ruling0),
                cross(edge0, ruling1),
                cross(edge1, ruling0),
                cross(edge1, ruling1),
            ];
            if bounds.iter().any(|normal| dot(*normal, *normal) <= 1e-16) {
                return Err(Error::new(
                    "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
                    "A ruled side has an unresolved zero-Jacobian Bernstein bound",
                ));
            }
        }
    }

    let mut build = Builder::new();
    let ids: Vec<Vec<usize>> = sections
        .iter()
        .map(|section| {
            section
                .iter()
                .map(|&point| {
                    let id = build.model.vertices.len();
                    build.model.vertices.push(Vertex { point });
                    id
                })
                .collect()
        })
        .collect();
    for layer in 0..sections.len() - 1 {
        for i in 0..count {
            let j = (i + 1) % count;
            let a = sections[layer][i];
            let b = sections[layer][j];
            let c = sections[layer + 1][j];
            let d = sections[layer + 1][i];
            build.rectangular_patch(
                surface(vec![
                    vec![a.to_vec(), d.to_vec()],
                    vec![b.to_vec(), c.to_vec()],
                ]),
                [
                    ids[layer][i],
                    ids[layer][j],
                    ids[layer + 1][j],
                    ids[layer + 1][i],
                ],
                [
                    line(a.to_vec(), b.to_vec()),
                    line(b.to_vec(), c.to_vec()),
                    line(c.to_vec(), d.to_vec()),
                    line(d.to_vec(), a.to_vec()),
                ],
            );
        }
    }
    for layer in [0, sections.len() - 1] {
        let (origin, u, v, profile) = &frames[layer];
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for point in profile {
            for axis in 0..2 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        let point = |x: f64, y: f64| -> Vec<f64> {
            (0..3)
                .map(|axis| origin[axis] + x * u[axis] + y * v[axis])
                .collect()
        };
        let uv = |p: [f64; 2]| {
            vec![
                (p[0] - min[0]) / (max[0] - min[0]),
                (p[1] - min[1]) / (max[1] - min[1]),
            ]
        };
        let mut edges = Vec::with_capacity(count);
        for i in 0..count {
            let j = (i + 1) % count;
            edges.push(build.coedge(
                ids[layer][i],
                ids[layer][j],
                line(sections[layer][i].to_vec(), sections[layer][j].to_vec()),
                line(uv(profile[i]), uv(profile[j])),
            ));
        }
        let wire = build.wire(edges);
        build.face(
            surface(vec![
                vec![point(min[0], min[1]), point(min[0], max[1])],
                vec![point(max[0], min[1]), point(max[0], max[1])],
            ]),
            wire,
            vec![],
            layer == 0,
        );
    }
    build.finish()
}
/// Matching vertices define a straight ruling between each adjacent section.
/// Subdivided Bernstein support bounds admit convex intermediate profiles.
/// This bounded numerical admission is sufficient, not a certified or complete
/// test of all valid correspondences. Surface joins across stations are C0.
pub fn ruled_loft(sections: &[Vec<P>]) -> Result<Model> {
    crate::operations::admit_loft_sections(sections)?;
    let count = sections[0].len();
    if (sections.len() - 1) * count + 2 > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Ruled loft exceeds 256 faces",
        ));
    }
    let origin = sections[0][0];
    let u = unit(sub(sections[0][1], origin));
    let n = unit(cross(u, sub(sections[0][2], origin)));
    let v = cross(n, u);
    let projected: Vec<Vec<[f64; 2]>> = sections
        .iter()
        .map(|s| {
            s.iter()
                .map(|p| {
                    let p = sub(*p, origin);
                    [dot(p, u), dot(p, v)]
                })
                .collect()
        })
        .collect();
    let mut remaining = 1_000_000usize;
    for pair in projected.windows(2) {
        for i in 0..count {
            let j = (i + 1) % count;
            let e0 = difference(pair[0][j], pair[0][i]);
            let e1 = difference(pair[1][j], pair[1][i]);
            for k in 0..count {
                if k == i || k == j {
                    continue;
                }
                let p0 = difference(pair[0][k], pair[0][i]);
                let p1 = difference(pair[1][k], pair[1][i]);
                let coefficients = [
                    side(e0, p0),
                    0.5 * (side(e0, p1) + side(e1, p0)),
                    side(e1, p1),
                ];
                positive_support(coefficients, &mut remaining)?;
            }
        }
    }
    let mut build = Builder::new();
    let ids: Vec<Vec<usize>> = sections
        .iter()
        .map(|s| {
            s.iter()
                .map(|&point| {
                    let id = build.model.vertices.len();
                    build.model.vertices.push(Vertex { point });
                    id
                })
                .collect()
        })
        .collect();
    for layer in 0..sections.len() - 1 {
        for i in 0..count {
            let j = (i + 1) % count;
            let a = sections[layer][i];
            let b = sections[layer][j];
            let c = sections[layer + 1][j];
            let d = sections[layer + 1][i];
            build.rectangular_patch(
                surface(vec![
                    vec![a.to_vec(), d.to_vec()],
                    vec![b.to_vec(), c.to_vec()],
                ]),
                [
                    ids[layer][i],
                    ids[layer][j],
                    ids[layer + 1][j],
                    ids[layer + 1][i],
                ],
                [
                    line(a.to_vec(), b.to_vec()),
                    line(b.to_vec(), c.to_vec()),
                    line(c.to_vec(), d.to_vec()),
                    line(d.to_vec(), a.to_vec()),
                ],
            );
        }
    }
    for layer in [0, sections.len() - 1] {
        let profile = &projected[layer];
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for p in profile {
            for k in 0..2 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        let height = dot(sub(sections[layer][0], origin), n);
        let point = |x: f64, y: f64| -> Vec<f64> {
            (0..3)
                .map(|k| origin[k] + x * u[k] + y * v[k] + height * n[k])
                .collect()
        };
        let uv = |p: [f64; 2]| {
            vec![
                (p[0] - min[0]) / (max[0] - min[0]),
                (p[1] - min[1]) / (max[1] - min[1]),
            ]
        };
        let mut edges = Vec::with_capacity(count);
        for i in 0..count {
            let j = (i + 1) % count;
            edges.push(build.coedge(
                ids[layer][i],
                ids[layer][j],
                line(sections[layer][i].to_vec(), sections[layer][j].to_vec()),
                line(uv(profile[i]), uv(profile[j])),
            ));
        }
        let wire = build.wire(edges);
        build.face(
            surface(vec![
                vec![point(min[0], min[1]), point(min[0], max[1])],
                vec![point(max[0], min[1]), point(max[0], max[1])],
            ]),
            wire,
            vec![],
            layer == 0,
        );
    }
    build.finish()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_twisted_profiles_are_admitted_after_bound_refinement() {
        let base = vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]];
        for angle in [90_f64, 120., 150.] {
            let (s, c) = angle.to_radians().sin_cos();
            let top = base
                .iter()
                .map(|p| [c * p[0] - s * p[1], s * p[0] + c * p[1], 3.])
                .collect();
            ruled_loft(&[base.clone(), top])
                .unwrap()
                .validate()
                .unwrap();
        }
    }
    #[test]
    fn support_refinement_has_typed_budget_refusal() {
        assert_eq!(
            positive_support([1., -0.5, 1.], &mut 0).unwrap_err().code,
            "BREP_RESOURCE_LIMIT"
        );
        assert!(positive_support([1., -1., 1.], &mut 100).is_err());
        assert!(positive_support([1., -0.5, 1.], &mut 100).is_ok());
    }
    #[test]
    fn ruled_side_patches_keep_four_edges_and_reject_twisted_collapse() {
        let base = vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]];
        let c = 0.5_f64.sqrt();
        let top = base
            .iter()
            .map(|p| [c * (p[0] - p[1]), c * (p[0] + p[1]), 3.])
            .collect();
        let model = ruled_loft(&[base.clone(), top]).unwrap();
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.edges.len(), 12);
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert!(model.loops.iter().all(|l| l.coedges.len() == 4));
        let twisted = base.iter().map(|p| [-p[0], -p[1], 3.]).collect();
        assert!(ruled_loft(&[base, twisted]).is_err());
    }
}
