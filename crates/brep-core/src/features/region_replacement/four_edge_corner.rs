//! Bounded four-valent pyramid-corner proposals with different arm mid-radii.
//! G2 blend profiles have variable curvature near planar supports, not circular
//! arcs of constant curvature all the way to a plane.
use super::{EdgeRadius, RadiusNetwork, corner_patch_body::Build, plan_radius_network};
use crate::{Body, Curve, FaceUse, Model, Shell, Surface};
use nurbs_core::{Error, Result};
const H: f64 = 19.;
const A: f64 = 3.;
pub struct FourEdgeCorner {
    pub model: Model,
    pub central_face: usize,
    pub strip_faces: [usize; 4],
    pub retained_faces: [usize; 5],
    pub smooth_edges: Vec<usize>,
    pub radius_plan: RadiusNetwork,
    pub requested_mid_radii: [f64; 4],
    pub authored_handles: [f64; 4],
    pub mid_radius_intervals: [[f64; 2]; 4],
}
fn guard(ok: bool, s: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_FOUR_EDGE_CORNER_PROPOSAL", s))
    }
}
fn surface(cp: Vec<Vec<Vec<f64>>>, domains: [[f64; 2]; 2]) -> Surface {
    let p = cp.len() - 1;
    let q = cp[0].len() - 1;
    Surface {
        degree_u: p,
        degree_v: q,
        knots_u: [vec![domains[0][0]; p + 1], vec![domains[0][1]; p + 1]].concat(),
        knots_v: [vec![domains[1][0]; q + 1], vec![domains[1][1]; q + 1]].concat(),
        weights: vec![vec![1.; q + 1]; p + 1],
        control_points: cp,
        periodic_u: false,
        periodic_v: false,
    }
}
fn roof(sx: f64, sy: f64) -> Surface {
    surface(
        (0..2)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        vec![
                            sx * H * i as f64,
                            sy * H * j as f64,
                            H - H * i as f64 - H * j as f64,
                        ]
                    })
                    .collect()
            })
            .collect(),
        [[0., H]; 2],
    )
}
fn base() -> Surface {
    surface(
        (0..2)
            .map(|i| {
                (0..2)
                    .map(|j| vec![-H + 2. * H * i as f64, -H + 2. * H * j as f64, 0.])
                    .collect()
            })
            .collect(),
        [[-H, H]; 2],
    )
}
fn pc(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Result<Curve> {
    let mut c = Curve::from_polyline(points)?;
    c.knots = vec![domain[0], domain[0], domain[1], domain[1]];
    Ok(c)
}
fn segment(
    b: &Build,
    a: usize,
    z: usize,
    uv: [[f64; 2]; 2],
) -> Result<(usize, usize, Curve, Curve)> {
    let c = Curve::from_polyline(vec![
        b.m.vertices[a].point.to_vec(),
        b.m.vertices[z].point.to_vec(),
    ])?;
    Ok((
        a,
        z,
        c,
        pc(uv.iter().map(|p| p.to_vec()).collect(), [0., 1.])?,
    ))
}
fn finish(mut b: Build, reverse: Vec<bool>) -> Result<Model> {
    b.m.shells = vec![Shell {
        faces: reverse
            .into_iter()
            .enumerate()
            .map(|(face, reversed)| FaceUse { face, reversed })
            .collect(),
        closed: true,
    }];
    b.m.bodies = vec![Body {
        outer_shell: 0,
        inner_shells: vec![],
    }];
    b.m.rebuild_topology_ids();
    b.m.validate()?;
    Ok(b.m)
}
/// Canonical closed source with one real four-edge vertex. All retained source
/// planes use physical UV clocks; no triangle fitting or source remeshing.
pub fn four_edge_corner_stock() -> Result<Model> {
    let mut b = Build::empty(1e-7);
    let apex = b.node([0., 0., H]);
    let xs = [b.node([-H, 0., 0.]), b.node([H, 0., 0.])];
    let ys = [b.node([0., -H, 0.]), b.node([0., H, 0.])];
    let mut rev = Vec::new();
    for (ix, sx) in [-1., 1.].into_iter().enumerate() {
        for (iy, sy) in [-1., 1.].into_iter().enumerate() {
            b.face(
                roof(sx, sy),
                vec![
                    segment(&b, apex, xs[ix], [[0., 0.], [H, 0.]])?,
                    segment(&b, xs[ix], ys[iy], [[H, 0.], [0., H]])?,
                    segment(&b, ys[iy], apex, [[0., H], [0., 0.]])?,
                ],
            )?;
            rev.push(sx * sy < 0.);
        }
    }
    let path = [xs[1], ys[1], xs[0], ys[0]];
    let mut seg = Vec::new();
    for i in 0..4 {
        let a = path[i];
        let z = path[(i + 1) % 4];
        let p = b.m.vertices[a].point;
        let q = b.m.vertices[z].point;
        seg.push(segment(&b, a, z, [[p[0], p[1]], [q[0], q[1]]])?);
    }
    b.face(base(), seg)?;
    rev.push(true);
    finish(b, rev)
}
fn profile(h: f64) -> [f64; 7] {
    [A, 2., 1., h, 1., 2., A]
}
fn law(a: f64, b: f64) -> [f64; 7] {
    [a, a, a, (a + b) / 2., b, b, b]
}
/// Four mid-section normal radii, ordered Y-, X+, Y+, X-. The source must be
/// the unchanged canonical pyramid. Positive convex degree-six profiles match
/// the planes to G2. A central 6x6 tensor patch matches all four independently
/// prescribed profile jets. Handle quantisation is explicit (2^-32 mm).
/// This is a proposal family, not arbitrary high-valence NURBS corner admission.
pub fn propose_four_edge_corner(source: &Model, radii: [f64; 4]) -> Result<FourEdgeCorner> {
    source.validate()?;
    guard(
        source.0 == four_edge_corner_stock()?.0,
        "Need the canonical four-valent stock",
    )?;
    let mut handles = [0.; 4];
    let mut intervals = [[0.; 2]; 4];
    use nurbs_core::interval_eval::Interval as I;
    for i in 0..4 {
        guard(
            radii[i].is_finite() && radii[i] > 0.,
            "Need four positive finite mid-radii",
        )?;
        let h = A - 8. * A * A * 2f64.sqrt() / (15. * radii[i]);
        handles[i] = (h * 4294967296.).round() / 4294967296.;
        guard(
            handles[i] >= 0. && handles[i] <= A / 3.,
            "Mid-radius is outside the convex G2 profile family",
        )?;
        let sqrt_two = std::f64::consts::SQRT_2;
        let numerator =
            I::point(8. * A * A).mul(I::new(sqrt_two.next_down(), sqrt_two.next_up())?)?;
        let denominator = I::point(15.).mul(I::point(A).sub(I::point(handles[i]))?)?;
        let bound = numerator.div(denominator)?;
        guard(
            bound.lo >= radii[i] - 1e-7 && bound.hi <= radii[i] + 1e-7,
            "Quantised arm mid-radius not within 1e-7 mm",
        )?;
        intervals[i] = [bound.lo, bound.hi];
    }
    let apex = source
        .vertices
        .iter()
        .position(|v| v.point == [0., 0., H])
        .unwrap();
    let arm_ends = [[0., -H, 0.], [H, 0., 0.], [0., H, 0.], [-H, 0., 0.]];
    let requests = std::array::from_fn::<_, 4, _>(|i| EdgeRadius {
        edge: source
            .edges
            .iter()
            .position(|e| {
                e.vertices.contains(&apex)
                    && e.vertices
                        .iter()
                        .any(|v| source.vertices[*v].point == arm_ends[i])
            })
            .unwrap(),
        radius_mm: radii[i],
    });
    let plan = plan_radius_network(source, 0, &requests, 10000)?;
    guard(
        plan.corners
            .iter()
            .any(|c| c.vertex == apex && c.incident.len() == 4 && c.selected.len() == 4),
        "Source does not own a complete four-edge fan",
    )?;
    let mut b = Build::empty(source.tolerance_mm);
    let mut core = [[0; 2]; 2];
    let mut xe = [[0; 2]; 2];
    let mut ye = [[0; 2]; 2];
    for (i, sx) in [-1., 1.].into_iter().enumerate() {
        for (j, sy) in [-1., 1.].into_iter().enumerate() {
            core[i][j] = b.node([sx * A, sy * A, H - 2. * A]);
            xe[i][j] = b.node([sx * (H - A), sy * A, 0.]);
            ye[i][j] = b.node([sx * A, sy * (H - A), 0.]);
        }
    }
    let mut reverse = Vec::new();
    let mut retained = [0; 5];
    for (i, sx) in [-1., 1.].into_iter().enumerate() {
        for (j, sy) in [-1., 1.].into_iter().enumerate() {
            retained[2 * i + j] = b.face(
                source.faces[2 * i + j].surface.clone(),
                vec![
                    segment(&b, core[i][j], xe[i][j], [[A, A], [H - A, A]])?,
                    segment(&b, xe[i][j], ye[i][j], [[H - A, A], [A, H - A]])?,
                    segment(&b, ye[i][j], core[i][j], [[A, H - A], [A, A]])?,
                ],
            )?;
            reverse.push(sx * sy < 0.);
        }
    }
    // Each strip is ruled between its central profile and its base trace. The
    // whole natural rectangle is retained; every authored pole stays in stock.
    let coords = [-3., -2., -1., 0., 1., 2., 3.];
    let strips = std::array::from_fn::<_, 4, _>(|arm| {
        let f = profile(handles[arm]);
        let y_arm = arm % 2 == 0;
        let sign = if arm == 0 || arm == 3 { -1. } else { 1. };
        surface(
            (0..7)
                .map(|i| {
                    vec![
                        if y_arm {
                            vec![coords[i], sign * A, H - f[i] - A]
                        } else {
                            vec![sign * A, coords[i], H - f[i] - A]
                        },
                        if y_arm {
                            vec![coords[i], sign * (H - f[i]), 0.]
                        } else {
                            vec![sign * (H - f[i]), coords[i], 0.]
                        },
                    ]
                })
                .collect(),
            [[-A, A], [0., 1.]],
        )
    });
    // Base arcs in positive XY traversal; graph contour remains source-owned.
    let path = [
        xe[1][0], xe[1][1], ye[1][1], ye[0][1], xe[0][1], xe[0][0], ye[0][0], ye[1][0],
    ];
    let arc_arms = [
        Some((1, false)),
        None,
        Some((2, true)),
        None,
        Some((3, true)),
        None,
        Some((0, false)),
        None,
    ];
    let mut seg = Vec::new();
    for i in 0..8 {
        let a = path[i];
        let z = path[(i + 1) % 8];
        if let Some((arm, rev)) = arc_arms[i] {
            let mut c = strips[arm].iso(nurbs_core::surface::Axis::V, 1.)?;
            if rev {
                c = c.reverse()?;
            }
            let pc = Curve {
                control_points: c.control_points.iter().map(|p| vec![p[0], p[1]]).collect(),
                degree: c.degree,
                knots: c.knots.clone(),
                weights: c.weights.clone(),
                periodic: false,
            };
            seg.push((a, z, c, pc));
        } else {
            let p = b.m.vertices[a].point;
            let q = b.m.vertices[z].point;
            seg.push(segment(&b, a, z, [[p[0], p[1]], [q[0], q[1]]])?);
        }
    }
    retained[4] = b.face(source.faces[4].surface.clone(), seg)?;
    reverse.push(true);
    let mut strip_faces = [0; 4];
    for arm in 0..4 {
        let s = &strips[arm];
        let (lo, hi, farlo, farhi) = match arm {
            0 => (core[0][0], core[1][0], ye[0][0], ye[1][0]),
            1 => (core[1][0], core[1][1], xe[1][0], xe[1][1]),
            2 => (core[0][1], core[1][1], ye[0][1], ye[1][1]),
            _ => (core[0][0], core[0][1], xe[0][0], xe[0][1]),
        };
        let near = s.iso(nurbs_core::surface::Axis::V, 0.)?;
        let far = s.iso(nurbs_core::surface::Axis::V, 1.)?.reverse()?;
        let p = |a: [f64; 2], z: [f64; 2], d| pc(vec![a.to_vec(), z.to_vec()], d);
        strip_faces[arm] = b.face(
            s.clone(),
            vec![
                (lo, hi, near, p([-A, 0.], [A, 0.], [-A, A])?),
                (
                    hi,
                    farhi,
                    Curve::from_polyline(vec![
                        b.m.vertices[hi].point.to_vec(),
                        b.m.vertices[farhi].point.to_vec(),
                    ])?,
                    p([A, 0.], [A, 1.], [0., 1.])?,
                ),
                (farhi, farlo, far, p([A, 1.], [-A, 1.], [-A, A])?),
                (
                    farlo,
                    lo,
                    Curve::from_polyline(vec![
                        b.m.vertices[farlo].point.to_vec(),
                        b.m.vertices[lo].point.to_vec(),
                    ])?,
                    p([-A, 1.], [-A, 0.], [0., 1.])?,
                ),
            ],
        )?;
        reverse.push(arm == 0 || arm == 1);
    }
    let hx = law(handles[0], handles[2]);
    let hy = law(handles[3], handles[1]);
    let f = profile(0.);
    let center = surface(
        (0..7)
            .map(|i| {
                (0..7)
                    .map(|j| {
                        vec![
                            coords[i],
                            coords[j],
                            H - f[i]
                                - f[j]
                                - if i == 3 { hx[j] } else { 0. }
                                - if j == 3 { hy[i] } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        [[-A, A]; 2],
    );
    let near_nodes = [core[0][0], core[1][0], core[1][1], core[0][1]];
    let uv = [[-A, -A], [A, -A], [A, A], [-A, A]];
    let mut seg = Vec::new();
    for e in 0..4 {
        let c = match e {
            0 => center.iso(nurbs_core::surface::Axis::V, -A)?,
            1 => center.iso(nurbs_core::surface::Axis::U, A)?,
            2 => center.iso(nurbs_core::surface::Axis::V, A)?.reverse()?,
            _ => center.iso(nurbs_core::surface::Axis::U, -A)?.reverse()?,
        };
        seg.push((
            near_nodes[e],
            near_nodes[(e + 1) % 4],
            c,
            pc(vec![uv[e].to_vec(), uv[(e + 1) % 4].to_vec()], [-A, A])?,
        ));
    }
    let central_face = b.face(center, seg)?;
    reverse.push(false);
    let mut model = finish(b, reverse)?;
    model.inherit_topology_ids(&[source]);
    // The source role and unchanged original surface establish correspondence
    // for tilted trimmed faces; centroid/proximity matching is not evidence.
    for (old, &face) in retained.iter().enumerate() {
        let parent = source.1.faces[old];
        let child = model.1.faces[face];
        if parent != child
            && !model.1.lineage.iter().any(|r| {
                r.entity_kind == "face"
                    && r.parents.contains(&parent)
                    && r.children.contains(&child)
            })
        {
            model.1.lineage.push(crate::TopologyLineageRecord {
                operation: "persist".into(),
                entity_kind: "face".into(),
                parents: vec![parent],
                children: vec![child],
            });
        }
    }
    model.refresh_change_set(&[source]);
    let mut smooth = Vec::new();
    for edge in 0..model.edges.len() {
        let faces = model
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| model.loops[f.outer].coedges.iter().any(|c| c.edge == edge))
            .map(|(f, _)| f)
            .collect::<Vec<_>>();
        if faces.len() == 2 && !faces.contains(&retained[4]) {
            smooth.push(edge);
        }
    }
    guard(
        smooth.len() == 12,
        "Four-edge corner does not own twelve smooth interfaces",
    )?;
    let closure = crate::watertight::inspect_with_exact(&model, 1_000_000, 1_000_000)?;
    guard(
        closure.closure.status == crate::watertight::Closure::WithinTolerance,
        "Four-edge boundary agreement remains unresolved",
    )?;
    Ok(FourEdgeCorner {
        model,
        central_face,
        strip_faces,
        retained_faces: retained,
        smooth_edges: smooth,
        radius_plan: plan,
        requested_mid_radii: radii,
        authored_handles: handles,
        mid_radius_intervals: intervals,
    })
}
