//! Adaptive display triangles on certified material rectangles.
//! Boundary bands remain explicit. This does not build a watertight mesh or
//! replace exact shell topology with approximate vertices.
use super::*;
use nurbs_core::{interval_eval::Interval as I, trim_domain::Location};
use std::collections::{HashMap, VecDeque};
type Domain = [[f64; 2]; 2];
type Bounds = [[f64; 2]; 3];
type Point = [f64; 3];
#[path = "source_profile_mesh_error.rs"]
mod error_bound;
use error_bound::{Derivatives, continuous_source, derivative_error_upper};

pub struct ExpressionMeshLimits {
    pub max_deviation_mm: f64,
    pub cells: usize,
    pub controls_per_enclosure: usize,
    pub spans: usize,
    pub tolerance_uv: f64,
    pub domain_cells: usize,
    pub domain_cells_per_cell: usize,
    pub max_depth: usize,
}
pub struct ExpressionMeshTile {
    pub face: usize,
    pub domain: Domain,
    pub corners: [Point; 4],
    pub deviation_upper_mm: f64,
}
pub struct ExpressionMeshPending {
    pub face: usize,
    pub domain: Domain,
    pub bounds: Option<Bounds>,
    pub reason: &'static str,
}
pub struct ExpressionMeshReport {
    pub tiles: Vec<ExpressionMeshTile>,
    pub pending: Vec<ExpressionMeshPending>,
    pub outside: Vec<(usize, Domain)>,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub max_deviation_upper_mm: f64,
    pub split_cells: usize,
}
struct Node {
    face: usize,
    domain: Domain,
    depth: usize,
    inherited_bounds: Option<Bounds>,
    inside: bool,
}
/// Both the true surface image and the affine display triangles lie in this
/// box. Its outward L1 diameter therefore bounds their pointwise Euclidean
/// distance, including the rounding of display vertices.
fn deviation_upper(bounds: Bounds) -> Result<f64> {
    let mut sum = I::point(0.);
    for [lo, hi] in bounds {
        I::new(lo, hi)?;
        sum = sum.add(I::point(hi).sub(I::point(lo))?)?;
    }
    Ok(sum.hi)
}
fn split(node: &Node, whole: Domain) -> Option<[Domain; 2]> {
    let width = |d: [f64; 2]| d[1] * 0.5 - d[0] * 0.5;
    let relative = |axis: usize| {
        let w = width(whole[axis]);
        if w > 0. {
            width(node.domain[axis]) / w
        } else {
            0.
        }
    };
    let axis = usize::from(relative(1) > relative(0));
    let [lo, hi] = node.domain[axis];
    let middle = lo * 0.5 + hi * 0.5;
    if middle <= lo || middle >= hi {
        return None;
    }
    let mut left = node.domain;
    let mut right = node.domain;
    left[axis][1] = middle;
    right[axis][0] = middle;
    Some([left, right])
}
fn pending(
    report: &mut ExpressionMeshReport,
    node: Node,
    bounds: Option<Bounds>,
    reason: &'static str,
) {
    report.pending.push(ExpressionMeshPending {
        face: node.face,
        domain: node.domain,
        bounds,
        reason,
    });
}
fn prepare(
    domains: Vec<Domain>,
    limits: ExpressionMeshLimits,
    mut classify: impl FnMut(usize, Domain, usize) -> Result<(Location, usize)>,
    mut enclose: impl FnMut(usize, Domain, usize) -> Result<(Option<Bounds>, usize)>,
    mut derivatives: impl FnMut(usize, Domain, usize) -> Result<(Option<Derivatives>, usize)>,
) -> Result<ExpressionMeshReport> {
    if !limits.max_deviation_mm.is_finite()
        || limits.max_deviation_mm <= 0.
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || limits.cells > 100_000
        || limits.controls_per_enclosure > 1_000_000
        || limits.spans > 100_000
        || limits.domain_cells > 100_000
        || limits.domain_cells_per_cell > 100_000
        || limits.max_depth > 32
        || domains.len() > 100_000
        || domains
            .iter()
            .flatten()
            .any(|r| !r.iter().all(|x| x.is_finite()) || r[0] > r[1])
    {
        return Err(Error::new(
            "BREP_PROFILE_MESH",
            "Bound mesh tolerance, domains and native work",
        ));
    }
    let mut report = ExpressionMeshReport {
        tiles: vec![],
        pending: vec![],
        outside: vec![],
        cells: 0,
        spans: 0,
        domain_cells: 0,
        max_deviation_upper_mm: 0.,
        split_cells: 0,
    };
    let mut queue = domains
        .iter()
        .enumerate()
        .map(|(face, &domain)| Node {
            face,
            domain,
            depth: 0,
            inherited_bounds: None,
            inside: false,
        })
        .collect::<VecDeque<_>>();
    // Coordinates shared by tiles on one face use identical rounded vertices.
    // Cross-face welding remains a separate exact-topology operation.
    let mut vertices = HashMap::<(usize, u64, u64), (Point, Bounds)>::new();
    while let Some(node) = queue.pop_front() {
        if report.cells == limits.cells
            || report.spans == limits.spans
            || limits.controls_per_enclosure == 0
        {
            let inherited = node.inherited_bounds;
            pending(&mut report, node, inherited, "mesh-work-limit");
            continue;
        }
        report.cells += 1;
        let available = limits
            .domain_cells_per_cell
            .min(limits.domain_cells - report.domain_cells);
        let (location, used) = if node.inside {
            (Location::Inside, 0)
        } else {
            classify(node.face, node.domain, available)?
        };
        if used > available {
            return Err(Error::new(
                "BREP_PROFILE_MESH",
                "Trim classifier exceeded its work budget",
            ));
        }
        report.domain_cells += used;
        if location == Location::Outside {
            report.outside.push((node.face, node.domain));
            continue;
        }
        let remaining = limits.spans - report.spans;
        let (bounds, used) = enclose(node.face, node.domain, remaining)?;
        if used > remaining {
            return Err(Error::new(
                "BREP_PROFILE_MESH",
                "Support enclosure exceeded its work budget",
            ));
        }
        report.spans += used;
        let Some(mut bounds) = bounds.or(node.inherited_bounds) else {
            pending(&mut report, node, None, "mesh-support-unproven");
            continue;
        };
        let mut reason = "mesh-trim-unproven";
        if location == Location::Inside {
            reason = "mesh-deviation-unproven";
            let (jet, used) = if deviation_upper(bounds)? > limits.max_deviation_mm {
                derivatives(node.face, node.domain, limits.spans - report.spans)?
            } else {
                (None, 0)
            };
            if used > limits.spans - report.spans {
                return Err(Error::new(
                    "BREP_PROFILE_MESH",
                    "Derivative enclosure exceeded its work budget",
                ));
            }
            report.spans += used;
            if jet.is_some() || deviation_upper(bounds)? <= limits.max_deviation_mm {
                let uv = [
                    [node.domain[0][0], node.domain[1][0]],
                    [node.domain[0][1], node.domain[1][0]],
                    [node.domain[0][1], node.domain[1][1]],
                    [node.domain[0][0], node.domain[1][1]],
                ];
                let mut corners = [[0.; 3]; 4];
                let mut anchor = [[0.; 2]; 3];
                let mut ready = true;
                for (i, [u, v]) in uv.into_iter().enumerate() {
                    let key = (node.face, u.to_bits(), v.to_bits());
                    if let Some(&(point, point_bounds)) = vertices.get(&key) {
                        corners[i] = point;
                        if i == 0 {
                            anchor = point_bounds;
                        }
                    } else {
                        let remaining = limits.spans - report.spans;
                        let (point, used) = enclose(node.face, [[u, u], [v, v]], remaining)?;
                        if used > remaining {
                            return Err(Error::new(
                                "BREP_PROFILE_MESH",
                                "Vertex enclosure exceeded its work budget",
                            ));
                        }
                        report.spans += used;
                        let Some(point) = point else {
                            ready = false;
                            break;
                        };
                        for [lo, hi] in point {
                            I::new(lo, hi)?;
                        }
                        corners[i] = point.map(|[lo, hi]| (lo * 0.5 + hi * 0.5).clamp(lo, hi));
                        if i == 0 {
                            anchor = point;
                        }
                        vertices.insert(key, (corners[i], point));
                    }
                    for (axis, &value) in corners[i].iter().enumerate() {
                        bounds[axis][0] = bounds[axis][0].min(value);
                        bounds[axis][1] = bounds[axis][1].max(value);
                    }
                }
                if !ready {
                    pending(&mut report, node, Some(bounds), "mesh-vertex-unproven");
                    continue;
                }
                let mut upper = deviation_upper(bounds)?;
                if let Some(jet) = jet {
                    if let Some(residual) =
                        derivative_error_upper(node.domain, corners, anchor, jet)?
                    {
                        upper = upper.min(residual);
                    }
                }
                if upper <= limits.max_deviation_mm {
                    report.max_deviation_upper_mm = report.max_deviation_upper_mm.max(upper);
                    report.tiles.push(ExpressionMeshTile {
                        face: node.face,
                        domain: node.domain,
                        corners,
                        deviation_upper_mm: upper,
                    });
                    continue;
                }
            }
        }
        if node.depth < limits.max_depth {
            if let Some(children) = split(&node, domains[node.face]) {
                report.split_cells += 1;
                for domain in children {
                    queue.push_back(Node {
                        face: node.face,
                        domain,
                        depth: node.depth + 1,
                        inherited_bounds: Some(bounds),
                        inside: location == Location::Inside,
                    });
                }
                continue;
            }
        }
        pending(&mut report, node, Some(bounds), reason);
    }
    Ok(report)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn prepare_display_mesh(
        &self,
        limits: ExpressionMeshLimits,
    ) -> Result<ExpressionMeshReport> {
        let domains = (0..self.faces().len())
            .map(|face| self.support_domain(face))
            .collect::<Result<Vec<_>>>()?;
        let tolerance = limits.tolerance_uv;
        let controls = limits.controls_per_enclosure;
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let original = root.restricted.proposal.body.geometry().shell();
        let continuous = self
            .faces()
            .iter()
            .map(|face| {
                let source = match face.kind() {
                    ShellFaceKind::Original(index) => {
                        original.faces()[index][0].edges()[0].surface()
                    }
                    _ => root
                        .restricted
                        .proposal
                        .profile(root.face)
                        .unwrap()
                        .source(),
                };
                continuous_source(source)
            })
            .collect::<Result<Vec<_>>>()?;
        prepare(
            domains,
            limits,
            |face, domain, cells| self.display_location(face, domain, tolerance, cells),
            |face, domain, spans| self.enclose_support(face, domain, controls, spans),
            |face, domain, spans| {
                if continuous[face] {
                    self.enclose_support_derivatives(face, domain, controls, spans)
                } else {
                    Ok((None, 0))
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> ExpressionMeshLimits {
        ExpressionMeshLimits {
            max_deviation_mm: 0.6,
            cells: 4096,
            controls_per_enclosure: 1000,
            spans: 100000,
            tolerance_uv: 1e-8,
            domain_cells: 10000,
            domain_cells_per_cell: 256,
            max_depth: 16,
        }
    }
    fn bounds(d: Domain, remaining: usize) -> Result<(Option<Bounds>, usize)> {
        if remaining == 0 {
            return Ok((None, 0));
        }
        let z = I::new(d[0][0], d[0][1])?.mul(I::new(d[1][0], d[1][1])?)?;
        Ok((Some([d[0], d[1], [z.lo, z.hi]]), 1))
    }
    fn area(d: Domain) -> f64 {
        (d[0][1] - d[0][0]) * (d[1][1] - d[1][0])
    }
    fn coverage(report: &ExpressionMeshReport) -> f64 {
        report.tiles.iter().map(|t| area(t.domain)).sum::<f64>()
            + report.pending.iter().map(|p| area(p.domain)).sum::<f64>()
            + report.outside.iter().map(|(_, d)| area(*d)).sum::<f64>()
    }
    #[test]
    fn curved_display_tiles_bound_pointwise_error_and_preserve_coverage() {
        let report = prepare(
            vec![[[0., 1.], [0., 1.]]],
            limits(),
            |_, _, _| Ok((Location::Inside, 1)),
            |_, d, n| bounds(d, n),
            |_, _, _| Ok((None, 0)),
        )
        .unwrap();
        assert!(report.pending.is_empty());
        assert!(report.tiles.len() > 1);
        assert_eq!(coverage(&report), 1.);
        assert!(report.max_deviation_upper_mm <= 0.6);
        // Sample both affine triangles of z=u*v. The algorithm's universal
        // error bound comes from interval arithmetic, not from these samples.
        let mut shared = HashMap::<(u64, u64), Point>::new();
        for tile in &report.tiles {
            assert!(tile.deviation_upper_mm <= 0.6);
            for (k, [u, v]) in [
                [tile.domain[0][0], tile.domain[1][0]],
                [tile.domain[0][1], tile.domain[1][0]],
                [tile.domain[0][1], tile.domain[1][1]],
                [tile.domain[0][0], tile.domain[1][1]],
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(previous) = shared.insert((u.to_bits(), v.to_bits()), tile.corners[k]) {
                    assert_eq!(
                        previous.map(f64::to_bits),
                        tile.corners[k].map(f64::to_bits)
                    );
                }
            }
            for t in [0., 0.25, 0.75, 1.] {
                for s in [0., 0.25, 0.75, 1.] {
                    let (indices, weights) = if t >= s {
                        ([0, 1, 2], [1. - t, t - s, s])
                    } else {
                        ([0, 2, 3], [1. - s, t, s - t])
                    };
                    let approximate: Point = std::array::from_fn(|a| {
                        (0..3)
                            .map(|k| tile.corners[indices[k]][a] * weights[k])
                            .sum()
                    });
                    let u = tile.domain[0][0] * (1. - t) + tile.domain[0][1] * t;
                    let v = tile.domain[1][0] * (1. - s) + tile.domain[1][1] * s;
                    let exact = [u, v, u * v];
                    let distance = (0..3)
                        .map(|a| (exact[a] - approximate[a]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(distance <= tile.deviation_upper_mm);
                }
            }
        }
    }
    #[test]
    fn derivative_residual_reaches_fine_tolerance_with_complete_coverage() {
        let mut l = limits();
        l.max_deviation_mm = 0.05;
        let report = prepare(
            vec![[[0., 1.], [0., 1.]]],
            l,
            |_, _, _| Ok((Location::Inside, 1)),
            |_, d, n| bounds(d, n),
            |_, d, n| {
                if n < 2 {
                    Ok((None, 0))
                } else {
                    Ok((
                        Some([[[1., 1.], [0., 0.], d[1]], [[0., 0.], [1., 1.], d[0]]]),
                        2,
                    ))
                }
            },
        )
        .unwrap();
        assert!(report.pending.is_empty());
        assert_eq!(coverage(&report), 1.);
        assert!(report.max_deviation_upper_mm <= 0.05);
        assert!(
            report.tiles.len() < 128,
            "A fine residual bound avoids a box-diameter grid"
        );
        for tile in &report.tiles {
            for t in [0., 0.25, 0.75, 1.] {
                for s in [0., 0.25, 0.75, 1.] {
                    let (indices, weights) = if t >= s {
                        ([0, 1, 2], [1. - t, t - s, s])
                    } else {
                        ([0, 2, 3], [1. - s, t, s - t])
                    };
                    let approximate: Point = std::array::from_fn(|a| {
                        (0..3)
                            .map(|k| tile.corners[indices[k]][a] * weights[k])
                            .sum()
                    });
                    let u = tile.domain[0][0] * (1. - t) + tile.domain[0][1] * t;
                    let v = tile.domain[1][0] * (1. - s) + tile.domain[1][1] * s;
                    let distance = ((u - approximate[0]).powi(2)
                        + (v - approximate[1]).powi(2)
                        + (u * v - approximate[2]).powi(2))
                    .sqrt();
                    assert!(distance <= tile.deviation_upper_mm);
                }
            }
        }
    }
    #[test]
    fn exhausted_budgets_and_depth_keep_the_entire_domain_pending() {
        for setting in 0..3 {
            let mut l = limits();
            match setting {
                0 => l.cells = 0,
                1 => l.spans = 0,
                _ => l.controls_per_enclosure = 0,
            }
            let report = prepare(
                vec![[[0., 1.], [0., 1.]], [[0., 1.], [0., 1.]]],
                l,
                |_, _, _| panic!("No trim work is authorized"),
                |_, _, _| panic!("No enclosure work is authorized"),
                |_, _, _| panic!("No derivative work is authorized"),
            )
            .unwrap();
            assert!(report.tiles.is_empty());
            assert_eq!(report.pending.len(), 2);
            assert_eq!(coverage(&report), 2.);
            assert_eq!(report.cells, 0);
        }
        let mut l = limits();
        l.max_depth = 0;
        let report = prepare(
            vec![[[0., 1.], [0., 1.]]],
            l,
            |_, _, _| Ok((Location::Inside, 1)),
            |_, d, n| bounds(d, n),
            |_, _, _| Ok((None, 0)),
        )
        .unwrap();
        assert!(report.tiles.is_empty());
        assert_eq!(report.pending[0].reason, "mesh-deviation-unproven");
        assert_eq!(coverage(&report), 1.);
        let mut l = limits();
        l.cells = 3;
        let report = prepare(
            vec![[[0., 1.], [0., 1.]]],
            l,
            |_, _, _| Ok((Location::Inside, 1)),
            |_, d, n| bounds(d, n),
            |_, _, _| Ok((None, 0)),
        )
        .unwrap();
        assert_eq!(report.cells, 3);
        assert_eq!(coverage(&report), 1.);
        assert!(report.pending.iter().all(|p| p.reason == "mesh-work-limit"));
    }
    #[test]
    fn unresolved_trim_bands_are_not_filled_with_triangles() {
        let mut l = limits();
        l.max_depth = 8;
        let report = prepare(
            vec![[[0., 1.], [0., 1.]]],
            l,
            |_, d, _| {
                Ok((
                    if d[0][1] < 0.5 {
                        Location::Outside
                    } else if d[0][0] > 0.5 {
                        Location::Inside
                    } else {
                        Location::Unresolved
                    },
                    1,
                ))
            },
            |_, d, n| bounds(d, n),
            |_, _, _| Ok((None, 0)),
        )
        .unwrap();
        assert!(
            !report.tiles.is_empty() && !report.outside.is_empty() && !report.pending.is_empty()
        );
        assert_eq!(coverage(&report), 1.);
        assert!(report.tiles.iter().all(|t| t.domain[0][0] > 0.5));
        assert!(
            report
                .pending
                .iter()
                .all(|p| p.reason == "mesh-trim-unproven")
        );
    }
}
