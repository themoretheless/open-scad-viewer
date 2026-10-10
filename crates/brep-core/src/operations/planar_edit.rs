use super::*;
/// Translate one supporting plane of a convex planar body. All adjacent faces
/// are rebuilt from their exact half-spaces; no display mesh participates.
/// Draft a convex planar prism about a neutral plane perpendicular to `axis`.
/// Positive angles expand sections in the positive axis direction. Caps remain
/// fixed; each lateral support tilts by the requested geometric angle.
/// This numerical half-space construction does not accept curved or oblique faces.
pub fn draft_planar_prism(
    model: &Model,
    axis: [f64; 3],
    origin: [f64; 3],
    angle: f64,
) -> Result<Model> {
    if !axis
        .iter()
        .chain(&origin)
        .chain([&angle])
        .all(|x| x.is_finite())
        || angle.abs() > 60.
    {
        return Err(failed(
            "Draft requires finite parameters and an angle within ±60 degrees",
        ));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(failed("Draft requires a nonzero axis"));
    }
    let axis = unit(axis.map(|x| x / magnitude))?;
    let mut planes = convex_planes(model)?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Draft supports at most 64 support planes",
        ));
    }
    let tangent = angle.to_radians().tan();
    let neutral = dot(axis, origin);
    let mut sides = 0;
    let mut caps = [false; 2];
    for plane in &mut planes {
        let alignment = dot(plane.normal, axis);
        if alignment.abs() <= 1e-10 {
            sides += 1;
            let normal = sub(plane.normal, mul(axis, tangent));
            let length = norm(normal);
            plane.normal = mul(normal, 1. / length);
            plane.offset = (plane.offset - tangent * neutral) / length;
            if !plane.offset.is_finite() {
                return Err(failed("Draft exceeds finite numeric range"));
            }
        } else if alignment.abs() >= 1. - 1e-10 {
            caps[usize::from(alignment > 0.)] = true;
        } else {
            return Err(unsupported(
                "Draft requires planar prism sides parallel to the axis and perpendicular caps",
            ));
        }
    }
    if sides < 3 || !caps.iter().all(|x| *x) {
        return Err(unsupported("Draft requires a closed planar prism"));
    }
    if angle == 0. {
        return Ok(model.clone());
    }
    let mut out = model_from_planes(&planes, model.tolerance_mm)?;
    // Reject disappearance of a cap or lateral support, including taper collapse.
    for plane in &planes {
        let count = out
            .vertices
            .iter()
            .filter(|v| {
                (dot(plane.normal, v.point) - plane.offset).abs() <= model.tolerance_mm * 8.
            })
            .count();
        if count < 3 {
            return Err(failed("Draft consumes a support face"));
        }
    }
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}

// Move an exposed polygonal cap along straight parallel side edges. The cap
// may be nonconvex or contain holes; no other vertex level may be crossed.
fn push_prismatic_cap(model: &Model, face_id: usize, distance: f64) -> Result<Model> {
    model.validate()?;
    if model.bodies.len()!=1 || model.shells.len()!=1 || !model.bodies[0].inner_shells.is_empty() {
        return Err(unsupported("Cap editing requires one connected shell"));
    }
    let shell=&model.shells[model.bodies[0].outer_shell];
    let selected=shell.faces.iter().find(|f|f.face==face_id)
        .ok_or_else(||unsupported("Unknown cap face"))?;
    let mut plane=face_plane(model,face_id)?;
    if selected.reversed {plane.normal=mul(plane.normal,-1.);plane.offset=-plane.offset;}
    let tolerance=model.tolerance_mm*8.;
    let face=&model.faces[face_id];
    let mut moved=BTreeSet::new();
    let mut cap_edges=BTreeSet::new();
    for ring in std::iter::once(&face.outer).chain(&face.holes) {
        moved.extend(loop_vertices(model,*ring)?);
        cap_edges.extend(model.loops[*ring].coedges.iter().map(|c|c.edge));
    }
    // A displayed planar rim can consist of several adjacent B-rep faces.
    // Include its connected coplanar component, never a remote coplanar cap.
    loop {
        let count=moved.len();
        for usage in &shell.faces {
            let f=&model.faces[usage.face];
            let mut ids=Vec::new();
            for ring in std::iter::once(&f.outer).chain(&f.holes) {ids.extend(loop_vertices(model,*ring)?);}
            let edges=std::iter::once(&f.outer).chain(&f.holes).flat_map(|&id|model.loops[id].coedges.iter().map(|c|c.edge)).collect::<Vec<_>>();
            if edges.iter().any(|id|cap_edges.contains(id)) && ids.iter().all(|&i|(dot(plane.normal,model.vertices[i].point)-plane.offset).abs()<=tolerance) {
                moved.extend(ids);
                cap_edges.extend(edges);
            }
        }
        if moved.len()==count {break;}
    }
    for (i,v) in model.vertices.iter().enumerate() {
        let gap=plane.offset-dot(plane.normal,v.point);
        if moved.contains(&i) {if gap.abs()>tolerance{return Err(unsupported("Cap is not planar"));}}
        else if gap<=tolerance || gap+distance<=tolerance {
            return Err(unsupported("Cap displacement reaches another vertex level"));
        }
    }
    for edge in &model.edges {
        if moved.contains(&edge.vertices[0])!=moved.contains(&edge.vertices[1]) {
            let d=sub(model.vertices[edge.vertices[1]].point,model.vertices[edge.vertices[0]].point);
            if norm(cross(d,plane.normal))>tolerance {
                return Err(unsupported("Cap side edges must follow the displacement direction"));
            }
        }
    }
    let mut polygons=Vec::new();
    for usage in &shell.faces {
        face_plane(model,usage.face)?; // Refuse curved geometry; do not flatten it.
        let f=&model.faces[usage.face];
        let ring=|id| -> Result<Vec<[f64;3]>> {
            let mut points=loop_vertices(model,id)?.into_iter().map(|i| {
                let p=model.vertices[i].point;
                if moved.contains(&i){add(p,mul(plane.normal,distance))}else{p}
            }).collect::<Vec<_>>();
            if usage.reversed {points.reverse();}
            Ok(points)
        };
        polygons.push(PlanarBoundary{outer:ring(f.outer)?,holes:f.holes.iter().map(|&id|ring(id)).collect::<Result<Vec<_>>>()?});
    }
    let mut out=model_from_trimmed_polygons(polygons,model.tolerance_mm)?;
    // This edit changes geometry without splitting or merging topology. Match
    // the rebuilt incidence graph to the source, including displaced vertices.
    fn correspondence<T: PartialEq>(target: &[T], source: &[T]) -> Result<Vec<usize>> {
        if target.len()!=source.len() {return Err(unsupported("Cap reconstruction changed topology"));}
        let mut used=BTreeSet::new();
        let mut result=Vec::new();
        for key in target {
            let matches=source.iter().enumerate().filter(|(_,candidate)|*candidate==key).map(|(i,_)|i).collect::<Vec<_>>();
            if matches.len()!=1 || !used.insert(matches[0]) {return Err(unsupported("Cap topology correspondence is ambiguous"));}
            result.push(matches[0]);
        }
        Ok(result)
    }
    let expected=model.vertices.iter().enumerate().map(|(i,v)|if moved.contains(&i){add(v.point,mul(plane.normal,distance))}else{v.point}).collect::<Vec<_>>();
    let vertex_map=correspondence(&out.vertices.iter().map(|v|v.point).collect::<Vec<_>>(),&expected)?;
    let edge_key=|mut pair:[usize;2]|{pair.sort();pair};
    let edge_map=correspondence(&out.edges.iter().map(|e|edge_key(e.vertices.map(|i|vertex_map[i]))).collect::<Vec<_>>(),&model.edges.iter().map(|e|edge_key(e.vertices)).collect::<Vec<_>>())?;
    let ring_key=|mut edges:Vec<usize>|{edges.sort();edges};
    let loop_map=correspondence(&out.loops.iter().map(|l|ring_key(l.coedges.iter().map(|c|edge_map[c.edge]).collect())).collect::<Vec<_>>(),&model.loops.iter().map(|l|ring_key(l.coedges.iter().map(|c|c.edge).collect())).collect::<Vec<_>>())?;
    let face_map=correspondence(&out.faces.iter().map(|f|(loop_map[f.outer],ring_key(f.holes.iter().map(|&i|loop_map[i]).collect()))).collect::<Vec<_>>(),&model.faces.iter().map(|f|(f.outer,ring_key(f.holes.clone()))).collect::<Vec<_>>())?;
    out.1.vertices=vertex_map.iter().map(|&i|model.1.vertices[i]).collect();
    out.1.edges=edge_map.iter().map(|&i|model.1.edges[i]).collect();
    out.1.loops=loop_map.iter().map(|&i|model.1.loops[i]).collect();
    out.1.faces=face_map.iter().map(|&i|model.1.faces[i]).collect();
    out.1.shells=model.1.shells.clone();
    out.1.bodies=model.1.bodies.clone();
    out.1.lineage=model.1.lineage.clone();
    out.refresh_change_set(&[model]);
    out.validate()?;
    Ok(out)
}

pub fn push_planar_face(model: &Model, face_id: usize, distance: f64) -> Result<Model> {
    if !distance.is_finite() || distance.abs() > 1e6 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Face displacement must be finite and within ±1000000 mm",
        ));
    }
    if model.bodies.len() > 1 {
        return crate::body_edit::edit_face(model, face_id, |part, face| {
            push_planar_face(part, face, distance)
        });
    }
    if let Some(cylinder) = crate::intersections::recognize_cylinder(model)? {
        let cap = cylinder
            .caps
            .iter()
            .position(|&id| id == face_id)
            .ok_or_else(|| unsupported("Cylinder push/pull requires a planar end cap"))?;
        let height = 2. * cylinder.half_height;
        let next_height = height + distance;
        if next_height <= model.tolerance_mm * 8. {
            return Err(failed("Displacement consumes the cylinder"));
        }
        // Stretch only along the axis, fixing the opposite cap. Affine mapping
        // preserves rational circles, trim curves, and retained topology IDs.
        let fixed = add(
            cylinder.center,
            mul(
                cylinder.axis,
                if cap == 0 {
                    cylinder.half_height
                } else {
                    -cylinder.half_height
                },
            ),
        );
        let delta = distance / height;
        let mut matrix = [[0.; 4]; 4];
        for i in 0..3 {
            for j in 0..3 {
                matrix[i][j] =
                    if i == j { 1. } else { 0. } + delta * cylinder.axis[i] * cylinder.axis[j];
            }
            matrix[i][3] = -delta * cylinder.axis[i] * dot(cylinder.axis, fixed);
        }
        matrix[3][3] = 1.;
        return crate::transform::affine(model, matrix);
    }
    // Preserve original rational side surfaces, weights and topology for a
    // recognized single XY profile prism. Fix the opposite cap exactly.
    if model.bodies.len()==1 {
        if let Some(profile)=crate::prism::recognize(model)? {
            if let Some(z)=model.faces.get(face_id).and_then(|f|crate::prism::planar_cap_z(&f.surface)) {
                if z==profile.z_min || z==profile.z_max {
                    let height=profile.z_max-profile.z_min;
                    if height+distance<=model.tolerance_mm*8. {return Err(failed("Displacement consumes the prism"));}
                    let scale=1.+distance/height;
                    let fixed=if z==profile.z_max {profile.z_min}else{profile.z_max};
                    return crate::transform::affine(model,[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,scale,(1.-scale)*fixed],[0.,0.,0.,1.]]);
                }
            }
        }
    }
    let mut planes = match convex_planes(model) {Ok(p)=>p,Err(_)=>return push_prismatic_cap(model,face_id,distance)};
    let faces = &model.shells[model.bodies[0].outer_shell].faces;
    let selected = faces
        .iter()
        .position(|u| u.face == face_id)
        .ok_or_else(|| Error::new("BREP_INVALID_SELECTION", "Unknown face"))?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar editing supports at most 64 support planes",
        ));
    }
    // Boolean output can partition one supporting plane into several faces.
    // Moving only one leaves the other coincident constraints clipping the cap.
    let support = planes[selected].clone();
    for plane in &mut planes {
        if norm(sub(plane.normal, support.normal)) <= 1e-10
            && (plane.offset - support.offset).abs() <= model.tolerance_mm
        {
            plane.offset += distance;
        }
    }
    let mut out = model_from_planes(&planes, model.tolerance_mm)?;
    if !out.vertices.iter().any(|v| {
        (dot(planes[selected].normal, v.point) - planes[selected].offset).abs()
            <= model.tolerance_mm * 8.
    }) {
        return Err(failed("Displacement consumes the selected face"));
    }
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}
/// Exact planar inward shell of a convex body. Selected opening planes extend
/// the inner cutter outside the original envelope before the native Boolean.
pub fn shell_planar(model: &Model, openings: &[usize], thickness: f64) -> Result<Model> {
    if !thickness.is_finite() || !(1e-5..=1e6).contains(&thickness) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Shell thickness must be 0.00001..1000000 mm",
        ));
    }
    let mut planes = convex_planes(model)?;
    let mut cavity_planes = planes.clone();
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar shell supports at most 64 support planes",
        ));
    }
    let faces = &model.shells[model.bodies[0].outer_shell].faces;
    if openings.iter().any(|f| !faces.iter().any(|u| u.face == *f)) {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Unknown shell opening face",
        ));
    }
    for (plane, face) in planes.iter_mut().zip(faces) {
        plane.offset += if openings.contains(&face.face) {
            thickness * 2.
        } else {
            -thickness
        };
    }
    // Opening supports extend the cutter, but an actual cavity must still lie
    // inside the original body. Otherwise an excessive thickness can move the
    // complete cutter outside stock and make difference return unchanged stock.
    cavity_planes.extend(planes.iter().copied());
    model_from_planes(&cavity_planes, model.tolerance_mm)?;
    let inner = model_from_planes(&planes, model.tolerance_mm)?;
    let mut out = boolean(model, &inner, "difference")?;
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}

/// Exact outward shell of a convex planar body. The expanded support-plane
/// envelope is cut by the original body; selected opening supports extend that
/// cutter through the expanded envelope. No sampled or mesh offset is used.
pub fn shell_planar_outward(model: &Model, openings: &[usize], thickness: f64) -> Result<Model> {
    if !thickness.is_finite() || !(1e-5..=1e6).contains(&thickness) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Shell thickness must be 0.00001..1000000 mm",
        ));
    }
    let source_planes = convex_planes(model)?;
    if source_planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar shell supports at most 64 support planes",
        ));
    }
    let faces = &model.shells[model.bodies[0].outer_shell].faces;
    if openings.iter().any(|f| !faces.iter().any(|u| u.face == *f)) {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Unknown shell opening face",
        ));
    }
    let mut envelope_planes = source_planes.clone();
    for plane in &mut envelope_planes {
        plane.offset += thickness;
    }
    let envelope = model_from_planes(&envelope_planes, model.tolerance_mm)?;
    let mut cutter_planes = source_planes;
    for (plane, face) in cutter_planes.iter_mut().zip(faces) {
        if openings.contains(&face.face) {
            plane.offset += thickness * 2.;
        }
    }
    // This also proves that all selected extensions retain a bounded cutter
    // and that no support-plane collision consumed the authored envelope.
    let cutter = model_from_planes(&cutter_planes, model.tolerance_mm)?;
    let mut out = boolean(&envelope, &cutter, "difference")?;
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}
/// Two closed pieces from an exact support-plane cut of a convex planar body.
pub fn split_planar(model: &Model, normal: [f64; 3], offset: f64) -> Result<[Model; 2]> {
    if normal.iter().any(|v| !v.is_finite()) || !offset.is_finite() {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Split plane must be finite",
        ));
    }
    // Scale first so equivalent finite plane equations cannot overflow the
    // norm (or collapse to zero) merely because of their coefficient units.
    let magnitude = normal.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Split plane normal must be nonzero",
        ));
    }
    let scaled = normal.map(|x| x / magnitude);
    let length = norm(scaled);
    let normal = mul(scaled, 1. / length);
    let offset = (offset / magnitude) / length;
    if !offset.is_finite() {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Normalized split offset must be finite",
        ));
    }
    let planes = convex_planes(model)?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar split supports at most 64 support planes",
        ));
    }
    let mut a = planes.clone();
    a.push(Plane { normal, offset });
    let mut b = planes;
    b.push(Plane {
        normal: mul(normal, -1.),
        offset: -offset,
    });
    let mut pair = [
        model_from_planes(&a, model.tolerance_mm)?,
        model_from_planes(&b, model.tolerance_mm)?,
    ];
    for part in &mut pair {
        part.inherit_topology_ids(&[model]);
        part.validate()?;
    }
    Ok(pair)
}
