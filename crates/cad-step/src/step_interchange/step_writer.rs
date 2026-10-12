use super::*;

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

pub(super) fn rebuild(kind: &AnalyticKind) -> Result<Model> {
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

pub(super) fn export_kind(kind: &AnalyticKind) -> Result<(String, FeatureCertificate)> {
    let mut w = StepWriter::new();
    let preamble = vec![
        "ISO-10303-21;".into(),
        "HEADER;".into(),
        "FILE_DESCRIPTION(('OpenSCAD Viewer analytic B-rep'),'2;1');".into(),
        "FILE_NAME('analytic-brep.step','2026-09-16',('open-scad-viewer'),(''),'analytic-features','','');"
            .into(),
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));".into(),
        "ENDSEC;".into(),
        "DATA;".into(),
    ];

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

pub(super) fn parse_vec3(s: &str) -> Option<[f64; 3]> {
    let parts: Vec<f64> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
    if parts.len() == 3 && parts.iter().all(|x| x.is_finite()) {
        Some([parts[0], parts[1], parts[2]])
    } else {
        None
    }
}

/// Part21 entity index: id → (type, raw args interior).
pub(super) fn parse_entities(text: &str) -> BTreeMap<usize, (String, String)> {
    let mut map = BTreeMap::new();
    let flat = text.replace(['\n', '\r'], " ");
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

pub(super) fn resolve_axis2(
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

pub(super) fn plane_axis_origins(entities: &BTreeMap<usize, (String, String)>) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for (ty, args) in entities.values() {
        if ty != "PLANE" {
            continue;
        }
        if let Some(axis_id) = args
            .split(',')
            .find_map(|t| t.trim().trim_start_matches('#').parse().ok())
            && let Some((po, _, _)) = resolve_axis2(entities, axis_id)
        {
            out.push(po);
        }
    }
    out
}
