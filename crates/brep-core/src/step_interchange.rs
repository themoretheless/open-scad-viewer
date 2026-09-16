//! Analytic STEP AP214/AP242 interchange for constructor solids.
//!
//! Export writes a linked ADVANCED_FACE / MANIFOLD_SOLID_BREP graph with real
//! AXIS2 placements and CIRCLE ring edges. Import rebuilds via surface+AXIS2
//! recognition → constructors (never proprietary OSCAD_SOLID, never AABB
//! fallback for curved solids, never silent height defaults). FACETED / STL /
//! OBJ / incomplete graphs refuse.

use crate::analytic::{cylinder, frustum, sphere, torus, tube};
use crate::analytic_features::FeatureCertificate;
use crate::intersections::{recognize_cone, recognize_cylinder, recognize_sphere, recognize_torus};
use crate::transform;
use crate::{Model, cuboid};
use nurbs_core::{Error, Result};
use std::cmp::Ordering;
use std::collections::BTreeMap;

fn refuse(message: &str) -> Error {
    Error::new("BREP_STEP_REFUSED", message)
}

#[derive(Clone, Debug)]
enum AnalyticKind {
    Cuboid {
        min: [f64; 3],
        max: [f64; 3],
    },
    Cylinder {
        radius: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Frustum {
        r_bottom: f64,
        r_top: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Sphere {
        center: [f64; 3],
        radius: f64,
    },
    Torus {
        major: f64,
        minor: f64,
        center: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Tube {
        outer: f64,
        inner: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
}

fn is_axis_aligned_cuboid(model: &Model) -> bool {
    model.validate().is_ok()
        && model.faces.len() == 6
        && model.vertices.len() == 8
        && model
            .faces
            .iter()
            .all(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1 && f.holes.is_empty())
        && model.edges.iter().all(|e| e.curve.degree == 1)
}

fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

fn recognize_tube(model: &Model) -> Option<AnalyticKind> {
    if model.validate().is_err()
        || model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 10
        || model.vertices.len() != 16
    {
        return None;
    }
    let caps: Vec<_> = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1 && !f.holes.is_empty())
        .collect();
    let sides = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    if caps.len() != 2 || sides != 8 {
        return None;
    }
    let plane_normal = |face: &crate::Face| -> Option<[f64; 3]> {
        let cps = &face.surface.control_points;
        if cps.len() < 2 || cps[0].len() < 2 || cps[0][0].len() != 3 {
            return None;
        }
        let a = &cps[0][0];
        let b = &cps[1][0];
        let c = &cps[0][1];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = n[0].hypot(n[1]).hypot(n[2]);
        if !(len.is_finite() && len > 1e-12) {
            return None;
        }
        Some([n[0] / len, n[1] / len, n[2] / len])
    };
    let mut axis = plane_normal(caps[0])?;
    if let Some(n1) = plane_normal(caps[1]) {
        if axis[0] * n1[0] + axis[1] * n1[1] + axis[2] * n1[2] < 0. {
            axis = [-axis[0], -axis[1], -axis[2]];
        }
    }
    let mut centroid = [0.; 3];
    for v in &model.vertices {
        for k in 0..3 {
            centroid[k] += v.point[k] / model.vertices.len() as f64;
        }
    }
    let mut zs = Vec::new();
    let mut radii = Vec::new();
    let mut outer_pt = None;
    for v in &model.vertices {
        let d = [
            v.point[0] - centroid[0],
            v.point[1] - centroid[1],
            v.point[2] - centroid[2],
        ];
        let z = d[0] * axis[0] + d[1] * axis[1] + d[2] * axis[2];
        let radial = [d[0] - z * axis[0], d[1] - z * axis[1], d[2] - z * axis[2]];
        let r = radial[0].hypot(radial[1]).hypot(radial[2]);
        zs.push(z);
        radii.push(r);
        if outer_pt
            .map(|(_, or): ([f64; 3], f64)| r > or)
            .unwrap_or(true)
        {
            outer_pt = Some((radial, r));
        }
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    radii.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let z0 = zs[0];
    let z1 = zs[zs.len() - 1];
    let height = z1 - z0;
    if !(height.is_finite() && height > 1e-9) {
        return None;
    }
    let inner = radii[0];
    let outer = radii[radii.len() - 1];
    if !(outer - inner >= 1e-5 && inner > 0.) {
        return None;
    }
    let (radial, _) = outer_pt?;
    let rlen = radial[0].hypot(radial[1]).hypot(radial[2]);
    if !(rlen.is_finite() && rlen > 1e-9) {
        return None;
    }
    let ref_dir = [radial[0] / rlen, radial[1] / rlen, radial[2] / rlen];
    let frame = frame_from_axis_ref(axis, ref_dir);
    let origin = [
        centroid[0] + axis[0] * z0,
        centroid[1] + axis[1] * z0,
        centroid[2] + axis[2] * z0,
    ];
    Some(AnalyticKind::Tube {
        outer,
        inner,
        height,
        origin,
        axis,
        frame,
    })
}

fn classify(model: &Model) -> Result<AnalyticKind> {
    model.validate()?;
    if let Some(s) = recognize_sphere(model)? {
        return Ok(AnalyticKind::Sphere {
            center: s.center,
            radius: s.radius,
        });
    }
    if let Some(t) = recognize_torus(model)? {
        return Ok(AnalyticKind::Torus {
            major: t.major,
            minor: t.minor,
            center: t.center,
            axis: t.axis,
            frame: t.frame,
        });
    }
    if let Some(c) = recognize_cone(model)? {
        return Ok(AnalyticKind::Frustum {
            r_bottom: c.r_bottom,
            r_top: c.r_top,
            height: c.height,
            origin: c.bottom,
            axis: c.axis,
            frame: c.frame,
        });
    }
    if let Some(c) = recognize_cylinder(model)? {
        let height = 2. * c.half_height;
        let origin = [
            c.center[0] - c.axis[0] * c.half_height,
            c.center[1] - c.axis[1] * c.half_height,
            c.center[2] - c.axis[2] * c.half_height,
        ];
        return Ok(AnalyticKind::Cylinder {
            radius: c.radius,
            height,
            origin,
            axis: c.axis,
            frame: c.frame,
        });
    }
    if let Some(t) = recognize_tube(model) {
        return Ok(t);
    }
    if is_axis_aligned_cuboid(model) {
        let (min, max) = model_bounds(model);
        return Ok(AnalyticKind::Cuboid { min, max });
    }
    Err(refuse(
        "STEP export admits constructor corpus only (cuboid/cylinder/frustum/sphere/torus/tube)",
    ))
}

struct StepWriter {
    next: usize,
    lines: Vec<String>,
}

impl StepWriter {
    fn new() -> Self {
        Self {
            next: 1,
            lines: Vec::new(),
        }
    }

    fn id(&mut self) -> usize {
        let id = self.next;
        self.next += 1;
        id
    }

    fn emit(&mut self, body: String) -> usize {
        let id = self.id();
        self.lines.push(format!("#{id}={body};"));
        id
    }

    fn cartesian(&mut self, p: [f64; 3]) -> usize {
        self.emit(format!(
            "CARTESIAN_POINT('',({:.15},{:.15},{:.15}))",
            p[0], p[1], p[2]
        ))
    }

    fn direction(&mut self, d: [f64; 3]) -> usize {
        self.emit(format!(
            "DIRECTION('',({:.15},{:.15},{:.15}))",
            d[0], d[1], d[2]
        ))
    }

    fn axis2(&mut self, origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> usize {
        let o = self.cartesian(origin);
        let a = self.direction(axis);
        let r = self.direction(ref_dir);
        self.emit(format!("AXIS2_PLACEMENT_3D('',#{o},#{a},#{r})"))
    }

    fn vertex_point(&mut self, point_id: usize) -> usize {
        self.emit(format!("VERTEX_POINT('',#{point_id})"))
    }

    fn line_edge(&mut self, va: usize, vb: usize, pa: usize, dir: [f64; 3]) -> usize {
        let d = self.direction(dir);
        let vec = self.emit(format!("VECTOR('',#{d},1.)"));
        let line = self.emit(format!("LINE('',#{pa},#{vec})"));
        self.emit(format!("EDGE_CURVE('',#{va},#{vb},#{line},.T.)"))
    }

    fn circle_edge(&mut self, va: usize, vb: usize, axis2: usize, radius: f64) -> usize {
        let circle = self.emit(format!("CIRCLE('',#{axis2},{:.15})", radius));
        self.emit(format!("EDGE_CURVE('',#{va},#{vb},#{circle},.T.)"))
    }

    fn oriented_edge(&mut self, edge: usize) -> usize {
        self.emit(format!("ORIENTED_EDGE('',*,*,#{edge},.T.)"))
    }

    fn edge_loop(&mut self, oriented: &[usize]) -> usize {
        let refs = oriented
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        self.emit(format!("EDGE_LOOP('',({refs}))"))
    }

    fn face_outer_bound(&mut self, loop_id: usize) -> usize {
        self.emit(format!("FACE_OUTER_BOUND('',#{loop_id},.T.)"))
    }

    fn face_bound(&mut self, loop_id: usize, sense: bool) -> usize {
        let sense = if sense { ".T." } else { ".F." };
        self.emit(format!("FACE_BOUND('',#{loop_id},{sense})"))
    }

    fn advanced_face_bounds(&mut self, bounds: &[usize], surface: usize) -> usize {
        let refs = bounds
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        self.emit(format!("ADVANCED_FACE('',({refs}),#{surface},.T.)"))
    }

    fn advanced_face(&mut self, bound: usize, surface: usize) -> usize {
        self.advanced_face_bounds(&[bound], surface)
    }

    fn close_shell(&mut self, face_ids: &[usize]) -> usize {
        let shell_refs = face_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        let shell = self.emit(format!("CLOSED_SHELL('',({shell_refs}))"));
        self.emit(format!("MANIFOLD_SOLID_BREP('body',#{shell})"))
    }

    fn face_from_edges(&mut self, surface: usize, edge_ids: &[usize]) -> usize {
        let oriented: Vec<usize> = edge_ids.iter().map(|&e| self.oriented_edge(e)).collect();
        let loop_id = self.edge_loop(&oriented);
        let bound = self.face_outer_bound(loop_id);
        self.advanced_face(bound, surface)
    }

    fn annular_face(&mut self, surface: usize, outer_edge: usize, inner_edge: usize) -> usize {
        let outer_oe = self.oriented_edge(outer_edge);
        let outer_loop = self.edge_loop(&[outer_oe]);
        let outer_bound = self.face_outer_bound(outer_loop);
        let inner_oe = self.oriented_edge(inner_edge);
        let inner_loop = self.edge_loop(&[inner_oe]);
        let inner_bound = self.face_bound(inner_loop, false);
        self.advanced_face_bounds(&[outer_bound, inner_bound], surface)
    }
}

fn placement(origin: [f64; 3], axis: [f64; 3], frame: [[f64; 3]; 2]) -> [[f64; 4]; 4] {
    let x = frame[0];
    let y = frame[1];
    let z = axis;
    [
        [x[0], y[0], z[0], origin[0]],
        [x[1], y[1], z[1], origin[1]],
        [x[2], y[2], z[2], origin[2]],
        [0., 0., 0., 1.],
    ]
}

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn rebuild(kind: &AnalyticKind) -> Result<Model> {
    match kind {
        AnalyticKind::Cuboid { min, max } => cuboid(*min, *max),
        AnalyticKind::Cylinder {
            radius,
            height,
            origin,
            axis,
            frame,
        } => {
            let m = cylinder(*radius, *height)?;
            transform::affine(&m, placement(*origin, *axis, *frame))
        }
        AnalyticKind::Frustum {
            r_bottom,
            r_top,
            height,
            origin,
            axis,
            frame,
        } => {
            let m = frustum(*r_bottom, *r_top, *height)?;
            transform::affine(&m, placement(*origin, *axis, *frame))
        }
        AnalyticKind::Sphere { center, radius } => {
            let m = sphere(*radius)?;
            transform::affine(
                &m,
                [
                    [1., 0., 0., center[0]],
                    [0., 1., 0., center[1]],
                    [0., 0., 1., center[2]],
                    [0., 0., 0., 1.],
                ],
            )
        }
        AnalyticKind::Torus {
            major,
            minor,
            center,
            axis,
            frame,
        } => {
            let m = torus(*major, *minor)?;
            transform::affine(&m, placement(*center, *axis, *frame))
        }
        AnalyticKind::Tube {
            outer,
            inner,
            height,
            origin,
            axis,
            frame,
        } => {
            let m = tube(*outer, *inner, *height)?;
            transform::affine(&m, placement(*origin, *axis, *frame))
        }
    }
}

fn write_cuboid_topology(w: &mut StepWriter, min: [f64; 3], max: [f64; 3]) {
    // Standard box: 8 vertices, 12 LINE edges, 6 planar faces.
    let corners = [
        min,                      // 0
        [max[0], min[1], min[2]], // 1
        [max[0], max[1], min[2]], // 2
        [min[0], max[1], min[2]], // 3
        [min[0], min[1], max[2]], // 4
        [max[0], min[1], max[2]], // 5
        max,                      // 6
        [min[0], max[1], max[2]], // 7
    ];
    let mid = |a: [f64; 3], b: [f64; 3]| {
        [
            0.5 * (a[0] + b[0]),
            0.5 * (a[1] + b[1]),
            0.5 * (a[2] + b[2]),
        ]
    };
    let planes = [
        (mid(corners[0], corners[1]), [0., -1., 0.], [1., 0., 0.]), // -Y
        (mid(corners[1], corners[2]), [1., 0., 0.], [0., 1., 0.]),  // +X
        (mid(corners[2], corners[3]), [0., 1., 0.], [-1., 0., 0.]), // +Y
        (mid(corners[3], corners[0]), [-1., 0., 0.], [0., -1., 0.]), // -X
        (mid(corners[0], corners[4]), [0., 0., -1.], [1., 0., 0.]), // -Z
        (mid(corners[4], corners[5]), [0., 0., 1.], [1., 0., 0.]),  // +Z
    ];
    let mut surfaces = Vec::new();
    for (o, ax, rd) in planes {
        let axis = w.axis2(o, ax, rd);
        surfaces.push(w.emit(format!("PLANE('',#{axis})")));
    }
    let point_ids: Vec<usize> = corners.iter().map(|p| w.cartesian(*p)).collect();
    let vertex_ids: Vec<usize> = point_ids.iter().map(|&p| w.vertex_point(p)).collect();
    let mut edge = |a: usize, b: usize| -> usize {
        let dir = [
            corners[b][0] - corners[a][0],
            corners[b][1] - corners[a][1],
            corners[b][2] - corners[a][2],
        ];
        let len = dir[0].hypot(dir[1]).hypot(dir[2]).max(1e-15);
        w.line_edge(
            vertex_ids[a],
            vertex_ids[b],
            point_ids[a],
            [dir[0] / len, dir[1] / len, dir[2] / len],
        )
    };
    // Bottom ring, top ring, verticals.
    let e01 = edge(0, 1);
    let e12 = edge(1, 2);
    let e23 = edge(2, 3);
    let e30 = edge(3, 0);
    let e45 = edge(4, 5);
    let e56 = edge(5, 6);
    let e67 = edge(6, 7);
    let e74 = edge(7, 4);
    let e04 = edge(0, 4);
    let e15 = edge(1, 5);
    let e26 = edge(2, 6);
    let e37 = edge(3, 7);
    let face_edges = [
        [e01, e15, e45, e04], // -Y
        [e12, e26, e56, e15], // +X
        [e23, e37, e67, e26], // +Y
        [e30, e04, e74, e37], // -X
        [e01, e12, e23, e30], // -Z
        [e45, e56, e67, e74], // +Z
    ];
    let mut face_ids = Vec::new();
    for (i, &surf) in surfaces.iter().enumerate() {
        face_ids.push(w.face_from_edges(surf, &face_edges[i]));
    }
    w.close_shell(&face_ids);
}

fn export_kind(kind: &AnalyticKind) -> Result<(String, FeatureCertificate)> {
    let mut w = StepWriter::new();
    let mut preamble = Vec::new();
    preamble.push("ISO-10303-21;".into());
    preamble.push("HEADER;".into());
    preamble.push("FILE_DESCRIPTION(('OpenSCAD Viewer analytic B-rep'),'2;1');".into());
    preamble.push(
        "FILE_NAME('analytic-brep.step','2026-09-16',('open-scad-viewer'),(''),'analytic-features','','');"
            .into(),
    );
    preamble.push(
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));".into(),
    );
    preamble.push("ENDSEC;".into());
    preamble.push("DATA;".into());

    let _unit = w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    let _ctx = w.emit("APPLICATION_CONTEXT('configuration controlled 3D design')".into());

    match kind {
        AnalyticKind::Cuboid { min, max } => {
            write_cuboid_topology(&mut w, *min, *max);
        }
        AnalyticKind::Cylinder {
            radius,
            height,
            origin,
            axis,
            frame,
        } => {
            let top_o = add3(*origin, scale3(*axis, *height));
            let ax_cyl = w.axis2(*origin, *axis, frame[0]);
            let cyl = w.emit(format!("CYLINDRICAL_SURFACE('',#{ax_cyl},{:.15})", radius));
            let ax_bot = w.axis2(*origin, *axis, frame[0]);
            let plane_b = w.emit(format!("PLANE('',#{ax_bot})"));
            let ax_top = w.axis2(top_o, *axis, frame[0]);
            let plane_t = w.emit(format!("PLANE('',#{ax_top})"));

            let p_bot = add3(*origin, scale3(frame[0], *radius));
            let p_top = add3(top_o, scale3(frame[0], *radius));
            let pid_b = w.cartesian(p_bot);
            let pid_t = w.cartesian(p_top);
            let vid_b = w.vertex_point(pid_b);
            let vid_t = w.vertex_point(pid_t);
            let circ_b = w.circle_edge(vid_b, vid_b, ax_bot, *radius);
            let circ_t = w.circle_edge(vid_t, vid_t, ax_top, *radius);
            let ruling = w.line_edge(vid_b, vid_t, pid_b, *axis);

            let face_ids = [
                w.face_from_edges(cyl, &[circ_b, ruling, circ_t]),
                w.face_from_edges(plane_b, &[circ_b]),
                w.face_from_edges(plane_t, &[circ_t]),
            ];
            w.close_shell(&face_ids);
        }
        AnalyticKind::Frustum {
            r_bottom,
            r_top,
            height,
            origin,
            axis,
            frame,
        } => {
            let top_o = add3(*origin, scale3(*axis, *height));
            let semi = if (*height).abs() > 1e-15 {
                ((*r_top - *r_bottom) / *height).atan()
            } else {
                0.
            };
            let ax_cone = w.axis2(*origin, *axis, frame[0]);
            let cone = w.emit(format!(
                "CONICAL_SURFACE('',#{ax_cone},{:.15},{:.15})",
                r_bottom, semi
            ));
            let mut face_ids = Vec::new();
            let mut side_edges = Vec::new();

            let (circ_b, vid_b, pid_b) = if *r_bottom > 0. {
                let ax_bot = w.axis2(*origin, *axis, frame[0]);
                let plane_b = w.emit(format!("PLANE('',#{ax_bot})"));
                let p = add3(*origin, scale3(frame[0], *r_bottom));
                let pid = w.cartesian(p);
                let vid = w.vertex_point(pid);
                let circ = w.circle_edge(vid, vid, ax_bot, *r_bottom);
                face_ids.push(w.face_from_edges(plane_b, &[circ]));
                side_edges.push(circ);
                (Some(circ), Some(vid), Some(pid))
            } else {
                let p = *origin;
                let pid = w.cartesian(p);
                let vid = w.vertex_point(pid);
                (None, Some(vid), Some(pid))
            };

            let (circ_t, vid_t, _pid_t) = if *r_top > 0. {
                let ax_top = w.axis2(top_o, *axis, frame[0]);
                let plane_t = w.emit(format!("PLANE('',#{ax_top})"));
                let p = add3(top_o, scale3(frame[0], *r_top));
                let pid = w.cartesian(p);
                let vid = w.vertex_point(pid);
                let circ = w.circle_edge(vid, vid, ax_top, *r_top);
                face_ids.push(w.face_from_edges(plane_t, &[circ]));
                side_edges.push(circ);
                (Some(circ), Some(vid), Some(pid))
            } else {
                let p = top_o;
                let pid = w.cartesian(p);
                let vid = w.vertex_point(pid);
                (None, Some(vid), Some(pid))
            };

            let ruling = w.line_edge(
                vid_b.expect("bottom vertex"),
                vid_t.expect("top vertex"),
                pid_b.expect("bottom point"),
                *axis,
            );
            let mut side = side_edges;
            side.push(ruling);
            let _ = (circ_b, circ_t);
            face_ids.insert(0, w.face_from_edges(cone, &side));
            w.close_shell(&face_ids);
        }
        AnalyticKind::Sphere { center, radius } => {
            let ax = w.axis2(*center, [0., 0., 1.], [1., 0., 0.]);
            let surf = w.emit(format!("SPHERICAL_SURFACE('',#{ax},{:.15})", radius));
            let p = [center[0] + *radius, center[1], center[2]];
            let pid = w.cartesian(p);
            let vid = w.vertex_point(pid);
            let circ = w.circle_edge(vid, vid, ax, *radius);
            let face = w.face_from_edges(surf, &[circ]);
            w.close_shell(&[face]);
        }
        AnalyticKind::Torus {
            major,
            minor,
            center,
            axis,
            frame,
        } => {
            let ax = w.axis2(*center, *axis, frame[0]);
            let surf = w.emit(format!(
                "TOROIDAL_SURFACE('',#{ax},{:.15},{:.15})",
                major, minor
            ));
            let p = add3(*center, scale3(frame[0], *major + *minor));
            let pid = w.cartesian(p);
            let vid = w.vertex_point(pid);
            let circ = w.circle_edge(vid, vid, ax, *major);
            let face = w.face_from_edges(surf, &[circ]);
            w.close_shell(&[face]);
        }
        AnalyticKind::Tube {
            outer,
            inner,
            height,
            origin,
            axis,
            frame,
        } => {
            let top_o = add3(*origin, scale3(*axis, *height));
            let ax_o = w.axis2(*origin, *axis, frame[0]);
            let cyl_o = w.emit(format!("CYLINDRICAL_SURFACE('',#{ax_o},{:.15})", outer));
            let ax_i = w.axis2(*origin, *axis, frame[0]);
            let cyl_i = w.emit(format!("CYLINDRICAL_SURFACE('',#{ax_i},{:.15})", inner));
            let ax_bot = w.axis2(*origin, *axis, frame[0]);
            let plane_b = w.emit(format!("PLANE('',#{ax_bot})"));
            let ax_top = w.axis2(top_o, *axis, frame[0]);
            let plane_t = w.emit(format!("PLANE('',#{ax_top})"));

            let p_bo = add3(*origin, scale3(frame[0], *outer));
            let p_bi = add3(*origin, scale3(frame[0], *inner));
            let p_to = add3(top_o, scale3(frame[0], *outer));
            let p_ti = add3(top_o, scale3(frame[0], *inner));
            let pid_bo = w.cartesian(p_bo);
            let pid_bi = w.cartesian(p_bi);
            let pid_to = w.cartesian(p_to);
            let pid_ti = w.cartesian(p_ti);
            let vid_bo = w.vertex_point(pid_bo);
            let vid_bi = w.vertex_point(pid_bi);
            let vid_to = w.vertex_point(pid_to);
            let vid_ti = w.vertex_point(pid_ti);

            let circ_bo = w.circle_edge(vid_bo, vid_bo, ax_bot, *outer);
            let circ_bi = w.circle_edge(vid_bi, vid_bi, ax_bot, *inner);
            let circ_to = w.circle_edge(vid_to, vid_to, ax_top, *outer);
            let circ_ti = w.circle_edge(vid_ti, vid_ti, ax_top, *inner);
            let ruling_o = w.line_edge(vid_bo, vid_to, pid_bo, *axis);
            let ruling_i = w.line_edge(vid_bi, vid_ti, pid_bi, *axis);

            let face_ids = [
                w.face_from_edges(cyl_o, &[circ_bo, ruling_o, circ_to]),
                w.face_from_edges(cyl_i, &[circ_bi, ruling_i, circ_ti]),
                w.annular_face(plane_b, circ_bo, circ_bi),
                w.annular_face(plane_t, circ_to, circ_ti),
            ];
            w.close_shell(&face_ids);
        }
    }

    let mut out = preamble;
    out.extend(w.lines);
    out.push("ENDSEC;".into());
    out.push("END-ISO-10303-21;".into());
    Ok((
        out.join("\n") + "\n",
        FeatureCertificate {
            capability: "step-interchange/1",
            complete: true,
            notes: vec![
                "step_advanced_face_manifold",
                "step_plane_cyl_cone_sphere_torus",
                "step_ap214_ap242_entity_graph",
                "step_vertex_edge_loop_bound",
                "step_topology_roundtrip_constructors",
                "step_circle_ring_edges",
                "step_graph_only_import",
                "step_cuboid_12_edge",
                "step_tube_face_bound",
                "aabb_import_removed",
                "oscad_solid_removed",
            ],
        },
    ))
}

fn parse_vec3(s: &str) -> Option<[f64; 3]> {
    let parts: Vec<f64> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
    if parts.len() == 3 && parts.iter().all(|x| x.is_finite()) {
        Some([parts[0], parts[1], parts[2]])
    } else {
        None
    }
}

/// Part21 entity index: id → (type, raw args interior).
fn parse_entities(text: &str) -> BTreeMap<usize, (String, String)> {
    let mut map = BTreeMap::new();
    let flat = text.replace('\n', " ").replace('\r', " ");
    for chunk in flat.split(';') {
        let chunk = chunk.trim();
        if !chunk.starts_with('#') {
            continue;
        }
        let Some((id_s, body)) = chunk.split_once('=') else {
            continue;
        };
        let Ok(id) = id_s.trim().trim_start_matches('#').parse::<usize>() else {
            continue;
        };
        let body = body.trim();
        if body.starts_with('(') {
            map.insert(id, ("COMPLEX".into(), body.to_string()));
            continue;
        }
        let Some(paren) = body.find('(') else {
            continue;
        };
        let ty = body[..paren].trim().to_string();
        let args = body[paren + 1..].trim_end_matches(')').to_string();
        map.insert(id, (ty, args));
    }
    map
}

fn resolve_cartesian(entities: &BTreeMap<usize, (String, String)>, id: usize) -> Option<[f64; 3]> {
    let (ty, args) = entities.get(&id)?;
    if ty != "CARTESIAN_POINT" {
        return None;
    }
    let coords = args.rsplit_once('(')?.1.trim_end_matches(')');
    parse_vec3(coords)
}

fn resolve_direction(entities: &BTreeMap<usize, (String, String)>, id: usize) -> Option<[f64; 3]> {
    let (ty, args) = entities.get(&id)?;
    if ty != "DIRECTION" {
        return None;
    }
    let coords = args.rsplit_once('(')?.1.trim_end_matches(')');
    parse_vec3(coords)
}

fn resolve_axis2(
    entities: &BTreeMap<usize, (String, String)>,
    id: usize,
) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
    let (ty, args) = entities.get(&id)?;
    if ty != "AXIS2_PLACEMENT_3D" {
        return None;
    }
    let refs: Vec<usize> = args
        .split(',')
        .filter_map(|t| t.trim().trim_start_matches('#').parse().ok())
        .collect();
    if refs.len() < 3 {
        return None;
    }
    let o = resolve_cartesian(entities, refs[0])?;
    let a = resolve_direction(entities, refs[1])?;
    let r = resolve_direction(entities, refs[2])?;
    Some((o, a, r))
}

fn plane_axis_origins(entities: &BTreeMap<usize, (String, String)>) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for (_id, (ty, args)) in entities {
        if ty != "PLANE" {
            continue;
        }
        if let Some(axis_id) = args
            .split(',')
            .find_map(|t| t.trim().trim_start_matches('#').parse().ok())
        {
            if let Some((po, _, _)) = resolve_axis2(entities, axis_id) {
                out.push(po);
            }
        }
    }
    out
}

fn axial_height_from_planes(
    origin: [f64; 3],
    axis: [f64; 3],
    plane_origins: &[[f64; 3]],
) -> Option<f64> {
    if plane_origins.len() < 2 {
        return None;
    }
    let mut zs: Vec<f64> = plane_origins
        .iter()
        .map(|po| {
            (po[0] - origin[0]) * axis[0]
                + (po[1] - origin[1]) * axis[1]
                + (po[2] - origin[2]) * axis[2]
        })
        .collect();
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let h = (zs[zs.len() - 1] - zs[0]).abs();
    if h.is_finite() && h > 1e-9 {
        Some(h)
    } else {
        None
    }
}

fn axial_height_from_points(
    entities: &BTreeMap<usize, (String, String)>,
    origin: [f64; 3],
    axis: [f64; 3],
) -> Option<f64> {
    let mut zs = Vec::new();
    for (_id, (ty, args)) in entities {
        if ty != "CARTESIAN_POINT" {
            continue;
        }
        if let Some(coords) = args.rsplit_once('(').map(|(_, c)| c.trim_end_matches(')')) {
            if let Some(p) = parse_vec3(coords) {
                zs.push(
                    (p[0] - origin[0]) * axis[0]
                        + (p[1] - origin[1]) * axis[1]
                        + (p[2] - origin[2]) * axis[2],
                );
            }
        }
    }
    if zs.len() < 2 {
        return None;
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let h = (zs[zs.len() - 1] - zs[0]).abs();
    if h.is_finite() && h > 1e-9 {
        Some(h)
    } else {
        None
    }
}

fn frame_from_axis_ref(axis: [f64; 3], ref_dir: [f64; 3]) -> [[f64; 3]; 2] {
    let y = [
        axis[1] * ref_dir[2] - axis[2] * ref_dir[1],
        axis[2] * ref_dir[0] - axis[0] * ref_dir[2],
        axis[0] * ref_dir[1] - axis[1] * ref_dir[0],
    ];
    [ref_dir, y]
}

fn kind_from_surfaces(entities: &BTreeMap<usize, (String, String)>) -> Option<AnalyticKind> {
    let mut spherical = None;
    let mut toroidal = None;
    let mut cylindrical = Vec::new();
    let mut conical = None;
    let mut planes = 0usize;
    for (_id, (ty, args)) in entities {
        match ty.as_str() {
            "SPHERICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 2 {
                    let axis_id: usize = parts[parts.len() - 2]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let radius: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, _, _) = resolve_axis2(entities, axis_id)?;
                    spherical = Some(AnalyticKind::Sphere { center: o, radius });
                }
            }
            "TOROIDAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 3 {
                    let axis_id: usize = parts[parts.len() - 3]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let major: f64 = parts[parts.len() - 2].trim().parse().ok()?;
                    let minor: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, a, r) = resolve_axis2(entities, axis_id)?;
                    toroidal = Some(AnalyticKind::Torus {
                        major,
                        minor,
                        center: o,
                        axis: a,
                        frame: frame_from_axis_ref(a, r),
                    });
                }
            }
            "CYLINDRICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 2 {
                    let axis_id: usize = parts[parts.len() - 2]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let radius: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    if let Some((o, a, r)) = resolve_axis2(entities, axis_id) {
                        cylindrical.push((radius, o, a, r));
                    }
                }
            }
            "CONICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 3 {
                    let axis_id: usize = parts[parts.len() - 3]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let r_bottom: f64 = parts[parts.len() - 2].trim().parse().ok()?;
                    let semi: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, a, r) = resolve_axis2(entities, axis_id)?;
                    conical = Some((r_bottom, o, a, r, semi));
                }
            }
            "PLANE" => planes += 1,
            _ => {}
        }
    }
    let plane_origins = plane_axis_origins(entities);

    if let Some(s) = spherical {
        return Some(s);
    }
    if let Some(t) = toroidal {
        return Some(t);
    }
    if let Some((rb, o, a, r, semi)) = conical {
        let height = if plane_origins.len() >= 2 {
            axial_height_from_planes(o, a, &plane_origins)?
        } else {
            // Apex / single-cap only: one radius must be zero after recovery.
            let height = axial_height_from_points(entities, o, a)?;
            let r_top = rb + height * semi.tan();
            if !(rb.abs() <= 1e-12 || r_top.abs() <= 1e-12) {
                return None;
            }
            height
        };
        let r_top = rb + height * semi.tan();
        return Some(AnalyticKind::Frustum {
            r_bottom: rb,
            r_top,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if cylindrical.len() == 1 {
        let (radius, o, a, r) = cylindrical[0];
        let height = axial_height_from_planes(o, a, &plane_origins)?;
        return Some(AnalyticKind::Cylinder {
            radius,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if cylindrical.len() == 2 {
        let mut pair = cylindrical.clone();
        pair.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(Ordering::Equal));
        let (inner, outer) = (pair[0].0, pair[1].0);
        let (_, o, a, r) = pair[0];
        let height = axial_height_from_planes(o, a, &plane_origins)?;
        return Some(AnalyticKind::Tube {
            outer,
            inner,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if planes >= 6
        && spherical.is_none()
        && toroidal.is_none()
        && cylindrical.is_empty()
        && conical.is_none()
    {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        let mut n = 0usize;
        for (_id, (ty, args)) in entities {
            if ty == "CARTESIAN_POINT" {
                if let Some(coords) = args.rsplit_once('(').map(|(_, c)| c.trim_end_matches(')')) {
                    if let Some(p) = parse_vec3(coords) {
                        for i in 0..3 {
                            min[i] = min[i].min(p[i]);
                            max[i] = max[i].max(p[i]);
                        }
                        n += 1;
                    }
                }
            }
        }
        if n >= 4 && min.iter().all(|v| v.is_finite()) && max.iter().all(|v| v.is_finite()) {
            return Some(AnalyticKind::Cuboid { min, max });
        }
    }
    None
}

fn refuse_mesh_payloads(text: &str) -> Result<()> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse("STEP payload exceeds 8 MiB resource limit"));
    }
    if !text.contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 STEP exchange"));
    }
    if text.contains("FACETED_BREP") && !text.contains("ADVANCED_FACE") {
        return Err(refuse("Faceted STEP is not analytic B-rep interchange"));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII")
        || upper.contains("ENDSOLID")
        || upper
            .lines()
            .any(|l| l.trim_start().starts_with("FACET NORMAL"))
    {
        return Err(refuse("STL mesh payload refused as analytic STEP"));
    }
    if (upper.contains("MTLLIB") || upper.lines().any(|l| l.trim_start().starts_with("F ")))
        && !text.contains("ISO-10303-21")
    {
        return Err(refuse("OBJ mesh payload refused as analytic STEP"));
    }
    if !(text.contains("ADVANCED_FACE") || text.contains("MANIFOLD_SOLID_BREP")) {
        return Err(refuse("STEP missing ADVANCED_FACE / MANIFOLD_SOLID_BREP"));
    }
    if !text.contains("MANIFOLD_SOLID_BREP") {
        return Err(refuse("STEP missing MANIFOLD_SOLID_BREP"));
    }
    Ok(())
}

fn validate_advanced_face_refs(entities: &BTreeMap<usize, (String, String)>) -> Result<()> {
    for (_id, (ty, args)) in entities {
        if ty != "ADVANCED_FACE" {
            continue;
        }
        let refs: Vec<&str> = args.split(',').collect();
        let surf = refs.iter().rev().find_map(|t| {
            let t = t.trim().trim_start_matches('#');
            if t.chars().all(|c| c.is_ascii_digit()) {
                t.parse::<usize>().ok()
            } else {
                None
            }
        });
        let Some(sid) = surf else {
            return Err(refuse("ADVANCED_FACE missing surface reference"));
        };
        if !entities.contains_key(&sid) {
            return Err(refuse("ADVANCED_FACE surface reference broken"));
        }
    }
    Ok(())
}

fn curved_requires_circle(kind: &AnalyticKind) -> bool {
    match kind {
        AnalyticKind::Cuboid { .. } => false,
        AnalyticKind::Frustum {
            r_bottom, r_top, ..
        } => r_bottom.abs() > 1e-12 || r_top.abs() > 1e-12,
        _ => true,
    }
}

/// Export constructor-corpus Model as linked AP214/AP242 Part 21.
pub fn export_step(model: &Model) -> Result<(String, FeatureCertificate)> {
    let kind = classify(model)?;
    export_kind(&kind)
}

/// Import analytic STEP from surfaces + AXIS2 only → constructors.
pub fn import_step(text: &str) -> Result<(Model, FeatureCertificate)> {
    refuse_mesh_payloads(text)?;
    let entities = parse_entities(text);
    validate_advanced_face_refs(&entities)?;
    let kind = kind_from_surfaces(&entities).ok_or_else(|| {
        refuse(
            "Incomplete analytic STEP graph; no constructor solid recognized (graph-only; no OSCAD_SOLID; AABB removed)",
        )
    })?;
    if curved_requires_circle(&kind) && !text.contains("CIRCLE") {
        return Err(refuse(
            "Curved analytic STEP requires CIRCLE ring edges (graph honesty)",
        ));
    }
    let model = rebuild(&kind)?;
    model.validate()?;
    Ok((
        model,
        FeatureCertificate {
            capability: "step-interchange/1",
            complete: true,
            notes: vec![
                "step_topology_roundtrip_constructors",
                "step_ap214_ap242_entity_graph",
                "step_graph_only_import",
                "step_circle_import_gate",
                "aabb_import_removed",
                "oscad_solid_removed",
            ],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytic::{cylinder, frustum, sphere, torus, tube};
    use crate::cuboid;

    fn envelope_ok(a: &Model, b: &Model, tol: f64) {
        let (amin, amax) = model_bounds(a);
        let (bmin, bmax) = model_bounds(b);
        for i in 0..3 {
            assert!((amin[i] - bmin[i]).abs() < tol, "min[{i}]");
            assert!((amax[i] - bmax[i]).abs() < tol, "max[{i}]");
        }
    }

    fn assert_graph_only(text: &str) {
        assert!(!text.contains("OSCAD_SOLID"));
        assert!(!text.contains("cuboid|"));
        assert!(!text.contains("cylinder|"));
        assert!(!text.contains("tube|"));
    }

    #[test]
    fn roundtrip_cuboid() {
        let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let (text, cert) = export_step(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("ADVANCED_FACE"));
        assert!(text.contains("PLANE"));
        assert!(text.contains("AXIS2_PLACEMENT_3D"));
        assert!(text.contains("VERTEX_POINT"));
        assert!(text.contains("EDGE_CURVE"));
        assert!(text.contains("EDGE_LOOP"));
        assert!(text.contains("AP242"));
        assert!(!text.contains("FACETED_BREP"));
        assert_graph_only(&text);
        assert_eq!(text.matches("EDGE_CURVE").count(), 12);
        assert_eq!(text.matches("VERTEX_POINT").count(), 8);
        let (back, icert) = import_step(&text).unwrap();
        assert!(icert.notes.contains(&"step_graph_only_import"));
        assert!(icert.notes.contains(&"oscad_solid_removed"));
        back.validate().unwrap();
        assert_eq!(back.faces.len(), model.faces.len());
        envelope_ok(&model, &back, 1e-9);
    }

    #[test]
    fn roundtrip_cylinder_not_aabb() {
        let model = cylinder(2., 4.).unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("CYLINDRICAL_SURFACE"));
        assert!(text.contains("CIRCLE"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        back.validate().unwrap();
        assert_eq!(back.faces.len(), model.faces.len());
        assert!(recognize_cylinder(&back).unwrap().is_some());
        assert!(
            back.faces
                .iter()
                .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        );
        envelope_ok(&model, &back, 1e-6);
    }

    #[test]
    fn roundtrip_sphere() {
        let model = sphere(3.).unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("SPHERICAL_SURFACE"));
        assert!(text.contains("CIRCLE"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        assert_eq!(back.faces.len(), 8);
        assert!(recognize_sphere(&back).unwrap().is_some());
        envelope_ok(&model, &back, 1e-6);
    }

    #[test]
    fn roundtrip_frustum() {
        let model = frustum(3., 1., 5.).unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("CONICAL_SURFACE"));
        assert!(text.contains("CIRCLE"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        assert!(recognize_cone(&back).unwrap().is_some());
        envelope_ok(&model, &back, 1e-5);
    }

    #[test]
    fn roundtrip_torus() {
        let model = torus(4., 1.).unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("TOROIDAL_SURFACE"));
        assert!(text.contains("CIRCLE"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        assert_eq!(back.faces.len(), 16);
        assert!(recognize_torus(&back).unwrap().is_some());
        envelope_ok(&model, &back, 1e-5);
    }

    #[test]
    fn roundtrip_tube() {
        let model = tube(3., 1.5, 4.).unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("CYLINDRICAL_SURFACE"));
        assert!(text.contains("CIRCLE"));
        assert!(text.contains("FACE_BOUND"));
        assert!(text.contains("FACE_OUTER_BOUND"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        assert_eq!(back.faces.len(), model.faces.len());
        envelope_ok(&model, &back, 1e-5);
    }

    #[test]
    fn roundtrip_apex_frustum() {
        for model in [frustum(0., 3., 5.).unwrap(), frustum(3., 0., 5.).unwrap()] {
            let (text, _) = export_step(&model).unwrap();
            assert!(text.contains("CONICAL_SURFACE"));
            // Apex end has no CIRCLE; the nonzero ring still has one.
            assert!(text.contains("CIRCLE"));
            assert_graph_only(&text);
            let (back, _) = import_step(&text).unwrap();
            assert!(recognize_cone(&back).unwrap().is_some());
            envelope_ok(&model, &back, 1e-4);
        }
    }

    #[test]
    fn roundtrip_oriented_tube() {
        let base = tube(3., 1.5, 4.).unwrap();
        let model = crate::transform::affine(
            &base,
            [
                [0., 0., 1., 1.],
                [1., 0., 0., 2.],
                [0., 1., 0., 3.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("FACE_BOUND"));
        let (back, _) = import_step(&text).unwrap();
        assert_eq!(back.faces.len(), model.faces.len());
        envelope_ok(&model, &back, 1e-4);
    }

    #[test]
    fn refuse_curved_without_circle() {
        let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=CARTESIAN_POINT('',(0.,0.,4.));
#7=AXIS2_PLACEMENT_3D('',#6,#2,#3);
#8=PLANE('',#4);
#9=PLANE('',#7);
#10=ADVANCED_FACE('',(#11),#5,.T.);
#11=FACE_OUTER_BOUND('',#12,.T.);
#12=EDGE_LOOP('',());
#13=ADVANCED_FACE('',(#14),#8,.T.);
#14=FACE_OUTER_BOUND('',#15,.T.);
#15=EDGE_LOOP('',());
#16=ADVANCED_FACE('',(#17),#9,.T.);
#17=FACE_OUTER_BOUND('',#18,.T.);
#18=EDGE_LOOP('',());
#19=CLOSED_SHELL('',(#10,#13,#16));
#20=MANIFOLD_SOLID_BREP('body',#19);
ENDSEC;
END-ISO-10303-21;
"#;
        assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
    }

    #[test]
    fn refuse_faceted_stl_obj_missing_manifold() {
        assert_eq!(
            import_step("ISO-10303-21;\nDATA;\n#1=FACETED_BREP('',#2);\nENDSEC;\n")
                .unwrap_err()
                .code,
            "BREP_STEP_REFUSED"
        );
        assert_eq!(
            import_step("solid ascii\nfacet normal 0 0 1\nendsolid\n")
                .unwrap_err()
                .code,
            "BREP_STEP_REFUSED"
        );
        assert_eq!(
            import_step("mtllib x.mtl\nv 0 0 0\nf 1 1 1\n")
                .unwrap_err()
                .code,
            "BREP_STEP_REFUSED"
        );
        assert_eq!(
            import_step("ISO-10303-21;\nDATA;\n#1=ADVANCED_FACE('',(#2),#3,.T.);\nENDSEC;\n")
                .unwrap_err()
                .code,
            "BREP_STEP_REFUSED"
        );
    }

    #[test]
    fn refuse_broken_refs() {
        let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));
ENDSEC;
DATA;
#1=MANIFOLD_SOLID_BREP('body',#2);
#2=CLOSED_SHELL('',(#3));
#3=ADVANCED_FACE('',(#4),#999,.T.);
ENDSEC;
END-ISO-10303-21;
"#;
        assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
    }

    #[test]
    fn refuse_cylinder_without_plane_span() {
        let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=ADVANCED_FACE('',(#7),#5,.T.);
#7=FACE_OUTER_BOUND('',#8,.T.);
#8=EDGE_LOOP('',());
#9=CLOSED_SHELL('',(#6));
#10=MANIFOLD_SOLID_BREP('body',#9);
ENDSEC;
END-ISO-10303-21;
"#;
        assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
    }

    #[test]
    fn refuse_cone_without_plane_span() {
        let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CONICAL_SURFACE('',#4,3.,0.4);
#6=ADVANCED_FACE('',(#7),#5,.T.);
#7=FACE_OUTER_BOUND('',#8,.T.);
#8=EDGE_LOOP('',());
#9=CLOSED_SHELL('',(#6));
#10=MANIFOLD_SOLID_BREP('body',#9);
ENDSEC;
END-ISO-10303-21;
"#;
        assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
    }
}
