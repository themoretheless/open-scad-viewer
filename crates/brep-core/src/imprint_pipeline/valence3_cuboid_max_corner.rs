use super::loft_curve_loops::ARC_WEIGHT;
use super::*;

/// Equal-radius valence-3 blend at the axis-aligned cuboid max corner:
/// three rational quarter-cylinders plus one stereographic spherical octant.
pub(crate) fn valence3_cuboid_max_corner(
    source: &Model,
    min: [f64; 3],
    max: [f64; 3],
    radius: f64,
) -> Result<Model> {
    if !(radius.is_finite()
        && radius > 0.
        && 2. * radius < max[0] - min[0]
        && 2. * radius < max[1] - min[1]
        && 2. * radius < max[2] - min[2])
    {
        return Err(refuse(
            "Valence-3 cuboid corner requires a fit radius inside all three extents",
        ));
    }
    let [lx, ly, lz] = min;
    let [hx, hy, hz] = max;
    let (cx, cy, cz) = (hx - radius, hy - radius, hz - radius);
    let r = radius;
    // Vertex roles (13):
    // 0 (lx,ly,lz) 1 (hx,ly,lz) 2 (lx,hy,lz) 3 (lx,ly,hz)
    // 4 (hx,cy,lz) 5 (cx,hy,lz) 6 (hx,cy,cz) 7 (cx,hy,cz) 8 (cx,cy,hz)
    // 9 (hx,ly,cz) 10 (cx,ly,hz) 11 (lx,hy,cz) 12 (lx,cy,hz)
    let pts = [
        [lx, ly, lz],
        [hx, ly, lz],
        [lx, hy, lz],
        [lx, ly, hz],
        [hx, cy, lz],
        [cx, hy, lz],
        [hx, cy, cz],
        [cx, hy, cz],
        [cx, cy, hz],
        [hx, ly, cz],
        [cx, ly, hz],
        [lx, hy, cz],
        [lx, cy, hz],
    ];
    let vertices: Vec<Vertex> = pts.iter().map(|p| Vertex { point: *p }).collect();

    let line3 = |a: usize, b: usize| -> Curve { crate::line(pts[a].to_vec(), pts[b].to_vec()) };
    let arc_xy = |c: [f64; 2], z: f64, start: f64, sweep: f64| -> Curve {
        arc_curve(
            CircleArc {
                center: c,
                radius: r,
                start,
                sweep,
            },
            z,
        )
    };
    let arc_xz = |y: f64| -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![cx + r, y, cz],
                vec![cx + r, y, cz + r],
                vec![cx, y, cz + r],
            ],
            // Reverse of stereographic meridian [1,1,2] pole→equator.
            weights: vec![2., 1., 1.],
            periodic: false,
        }
    };
    let arc_yz = |x: f64| -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![x, cy + r, cz],
                vec![x, cy + r, cz + r],
                vec![x, cy, cz + r],
            ],
            weights: vec![2., 1., 1.],
            periodic: false,
        }
    };

    // Sphere-cylinder seams (quarter circles).
    let seam_z = arc_xy([cx, cy], cz, 0., std::f64::consts::FRAC_PI_2); // 6→7
    let seam_y = arc_xz(cy); // 6→8
    let seam_x = arc_yz(cx); // 7→8
    let bottom_z = arc_xy([cx, cy], lz, 0., std::f64::consts::FRAC_PI_2); // 4→5
    let remote_y = arc_xz(ly); // 9→10
    let remote_x = arc_yz(lx); // 11→12

    let mut edges: Vec<Edge> = Vec::new();
    let mut edge_of: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut add = |a: usize, b: usize, curve: Curve| -> usize {
        let key = (a.min(b), a.max(b));
        if let Some(&id) = edge_of.get(&key) {
            return id;
        }
        let id = edges.len();
        let stored = if a <= b {
            curve
        } else {
            let mut c = curve;
            c.control_points.reverse();
            c.weights.reverse();
            c
        };
        edges.push(Edge {
            degenerate: false,
            vertices: [key.0, key.1],
            curve: stored,
        });
        edge_of.insert(key, id);
        id
    };

    // Straight skeleton.
    for (a, b) in [
        (0, 1),
        (1, 4),
        (5, 2),
        (2, 0),
        (0, 3),
        (1, 9),
        (9, 6),
        (4, 6),
        (5, 7),
        (2, 11),
        (11, 7),
        (3, 10),
        (10, 8),
        (3, 12),
        (12, 8),
    ] {
        add(a, b, line3(a, b));
    }
    // Arcs (endpoint order matches curve direction for a→b when a<b may reverse store).
    add(4, 5, bottom_z.clone());
    add(6, 7, seam_z.clone());
    add(6, 8, seam_y.clone());
    add(7, 8, seam_x.clone());
    add(9, 10, remote_y.clone());
    add(11, 12, remote_x.clone());

    let eid = |a: usize, b: usize| -> usize { *edge_of.get(&(a.min(b), a.max(b))).expect("edge") };
    let oriented_curve = |a: usize, b: usize| -> Curve {
        let edge = &edges[eid(a, b)];
        if a <= b {
            edge.curve.clone()
        } else {
            let mut c = edge.curve.clone();
            c.control_points.reverse();
            c.weights.reverse();
            c
        }
    };
    let project_uv = |curve: &Curve, map: &dyn Fn(&[f64]) -> Vec<f64>| -> Curve {
        Curve {
            control_points: curve.control_points.iter().map(|p| map(p)).collect(),
            ..curve.clone()
        }
    };
    let co = |a: usize, b: usize, pcurve: Curve| -> Coedge {
        Coedge {
            edge: eid(a, b),
            reversed: a > b,
            pcurve,
        }
    };
    let xy = |p: &[f64]| vec![(p[0] - lx) / (hx - lx), (p[1] - ly) / (hy - ly)];
    let xz = |p: &[f64]| vec![(p[0] - lx) / (hx - lx), (p[2] - lz) / (hz - lz)];
    let yz = |p: &[f64]| vec![(p[1] - ly) / (hy - ly), (p[2] - lz) / (hz - lz)];
    // For +X face: u along +Y, v along +Z.
    let x_face = |p: &[f64]| vec![(p[1] - ly) / (hy - ly), (p[2] - lz) / (hz - lz)];
    // For +Y face: u along +X, v along +Z.
    let y_face = |p: &[f64]| vec![(p[0] - lx) / (hx - lx), (p[2] - lz) / (hz - lz)];

    let mut loops = Vec::new();
    let mut faces = Vec::new();
    let mut shell = Vec::new();
    let push_face = |loops: &mut Vec<Loop>,
                     faces: &mut Vec<Face>,
                     shell: &mut Vec<FaceUse>,
                     surface: Surface,
                     cos: Vec<Coedge>,
                     reversed: bool| {
        let outer = loops.len();
        loops.push(Loop { coedges: cos });
        let face = faces.len();
        faces.push(Face {
            surface,
            outer,
            holes: vec![],
        });
        shell.push(FaceUse { face, reversed });
    };

    let planar = |origin: [f64; 3], du: [f64; 3], dv: [f64; 3]| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![
                origin.to_vec(),
                [origin[0] + dv[0], origin[1] + dv[1], origin[2] + dv[2]].to_vec(),
            ],
            vec![
                [origin[0] + du[0], origin[1] + du[1], origin[2] + du[2]].to_vec(),
                [
                    origin[0] + du[0] + dv[0],
                    origin[1] + du[1] + dv[1],
                    origin[2] + du[2] + dv[2],
                ]
                .to_vec(),
            ],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = |a: usize, b: usize, map: &dyn Fn(&[f64]) -> Vec<f64>| {
        co(a, b, project_uv(&oriented_curve(a, b), map))
    };

    // Bottom z=lz (outward -Z ⇒ reversed).
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([lx, ly, lz], [hx - lx, 0., 0.], [0., hy - ly, 0.]),
        vec![
            pc(0, 1, &xy),
            pc(1, 4, &xy),
            pc(4, 5, &xy),
            pc(5, 2, &xy),
            pc(2, 0, &xy),
        ],
        true,
    );
    // Top z=hz.
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([lx, ly, hz], [hx - lx, 0., 0.], [0., hy - ly, 0.]),
        vec![
            pc(3, 10, &xy),
            pc(10, 8, &xy),
            pc(8, 12, &xy),
            pc(12, 3, &xy),
        ],
        false,
    );
    // Face x=lx (outward -X ⇒ reversed).
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([lx, ly, lz], [0., hy - ly, 0.], [0., 0., hz - lz]),
        vec![
            pc(0, 2, &yz),
            pc(2, 11, &yz),
            pc(11, 12, &yz),
            pc(12, 3, &yz),
            pc(3, 0, &yz),
        ],
        true,
    );
    // Face y=ly. CCW UV; FaceUse false so edge 0-1 opposes bottom (XOR).
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([lx, ly, lz], [hx - lx, 0., 0.], [0., 0., hz - lz]),
        vec![
            pc(0, 1, &xz),
            pc(1, 9, &xz),
            pc(9, 10, &xz),
            pc(10, 3, &xz),
            pc(3, 0, &xz),
        ],
        false,
    );
    // Face x=hx.
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([hx, ly, lz], [0., hy - ly, 0.], [0., 0., hz - lz]),
        vec![
            pc(1, 4, &x_face),
            pc(4, 6, &x_face),
            pc(6, 9, &x_face),
            pc(9, 1, &x_face),
        ],
        false,
    );
    // Face y=hy — reverse prior winding to make UV CCW.
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        planar([lx, hy, lz], [hx - lx, 0., 0.], [0., 0., hz - lz]),
        vec![
            pc(2, 5, &y_face),
            pc(5, 7, &y_face),
            pc(7, 11, &y_face),
            pc(11, 2, &y_face),
        ],
        true,
    );

    let uv_line = |u0: f64, v0: f64, u1: f64, v1: f64| crate::line(vec![u0, v0], vec![u1, v1]);

    // Vertical cylinder Z: ruled between bottom_z and seam_z.
    let cyl_z = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|k| {
                vec![
                    bottom_z.control_points[k].clone(),
                    seam_z.control_points[k].clone(),
                ]
            })
            .collect(),
        weights: vec![vec![1., 1.], vec![ARC_WEIGHT, ARC_WEIGHT], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        cyl_z,
        vec![
            co(4, 5, uv_line(0., 0., 1., 0.)),
            co(5, 7, uv_line(1., 0., 1., 1.)),
            co(7, 6, uv_line(1., 1., 0., 1.)),
            co(6, 4, uv_line(0., 1., 0., 0.)),
        ],
        false,
    );

    // Cylinder along Y (edge +X/+Z): ruled between remote_y (y=ly) and seam_y (y=cy).
    let cyl_y = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|k| {
                vec![
                    remote_y.control_points[k].clone(),
                    seam_y.control_points[k].clone(),
                ]
            })
            .collect(),
        weights: vec![vec![2., 2.], vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        cyl_y,
        vec![
            co(9, 10, uv_line(0., 0., 1., 0.)),
            co(10, 8, uv_line(1., 0., 1., 1.)),
            co(8, 6, uv_line(1., 1., 0., 1.)),
            co(6, 9, uv_line(0., 1., 0., 0.)),
        ],
        true,
    );

    // Cylinder along X (edge +Y/+Z).
    let cyl_x = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|k| {
                vec![
                    remote_x.control_points[k].clone(),
                    seam_x.control_points[k].clone(),
                ]
            })
            .collect(),
        weights: vec![vec![2., 2.], vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        cyl_x,
        vec![
            co(11, 12, uv_line(0., 0., 1., 0.)),
            co(12, 8, uv_line(1., 0., 1., 1.)),
            co(8, 7, uv_line(1., 1., 0., 1.)),
            co(7, 11, uv_line(0., 1., 0., 0.)),
        ],
        false,
    );

    // Stereographic spherical octant at C (local +X/+Y/+Z), translated.
    let linear = [0., 0.5, 1.];
    let square = [0., 0., 1.];
    let a_dir = [1., 0.];
    let b_dir = [0., 1.];
    let weights: Vec<Vec<f64>> = (0..3)
        .map(|i| (0..3).map(|j| 1. + square[i] + square[j]).collect())
        .collect();
    let control_points: Vec<Vec<Vec<f64>>> = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let w = weights[i][j];
                    vec![
                        cx + 2. * r * (linear[i] * a_dir[0] + linear[j] * b_dir[0]) / w,
                        cy + 2. * r * (linear[i] * a_dir[1] + linear[j] * b_dir[1]) / w,
                        cz + r * (1. - square[i] - square[j]) / w,
                    ]
                })
                .collect()
        })
        .collect();
    let sphere = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    // Sphere loop matches analytic::sphere quadrant: pole(8) → +X(6) → +Y(7) → pole.
    let circular_trim = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., ARC_WEIGHT, 1.],
        periodic: false,
    };
    push_face(
        &mut loops,
        &mut faces,
        &mut shell,
        sphere,
        vec![
            co(8, 6, uv_line(0., 0., 1., 0.)),
            co(6, 7, circular_trim),
            co(7, 8, uv_line(0., 1., 0., 0.)),
        ],
        false,
    );

    assemble_imprint_solid(
        vertices,
        edges,
        loops,
        faces,
        shell,
        source.tolerance_mm,
        &[source],
    )
}

/// Exact constant-radius rounding of selected convex vertices of a simple
/// CCW profile, extruded along local +Z. Every round is one positive-weight
/// rational circular arc and therefore authors a cylindrical, not faceted,
/// side face.
pub(crate) fn rounded_convex_prism_edges(
    source: &Model,
    profile: &[[f64; 2]],
    rounded: &[bool],
    radius: f64,
    z0: f64,
    z1: f64,
) -> Result<Model> {
    if !z1.is_finite() || z1 <= z0 {
        return Err(refuse("Rounded convex prism height is invalid"));
    }
    let curves = rounded_profile_curves(source, profile, rounded, radius, z0)?;
    extrude_curve_loop(&curves, z0, z1, source.tolerance_mm, &[source])
}
