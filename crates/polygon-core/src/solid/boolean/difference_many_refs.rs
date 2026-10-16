use super::*;

/// Borrowing variant of [`difference_many`]: avoids deep-cloning the cutters
/// at call sites that only hold references.
pub fn difference_many_refs(
    base: &Mesh,
    cutters: &[&Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
    batch: usize,
) -> Result<Mesh> {
    let batch = batch.max(1);
    let base_box = bounds(base);
    if base_box.is_none() {
        return Ok(crate::solid::primitives::empty());
    }
    let boxes: Vec<_> = cutters.iter().map(|m| bounds(m)).collect();
    let pad = bounds_pad(&[&[base_box][..], &boxes[..]].concat());
    let mut batches: Vec<Vec<usize>> = Vec::new();
    for (i, cutter_box) in boxes.iter().enumerate() {
        if !bounds_touch(base_box, *cutter_box, pad) {
            continue;
        }
        let open = batches.iter_mut().find(|members| {
            members.len() < batch
                && members
                    .iter()
                    .all(|&j| !bounds_touch(boxes[j], *cutter_box, pad))
        });
        match open {
            Some(members) => members.push(i),
            None => batches.push(vec![i]),
        }
    }
    let mut result = base.clone();
    for members in batches {
        let cutter = if members.len() == 1 {
            (*cutters[members[0]]).clone()
        } else {
            let parts: Vec<Mesh> = members.iter().map(|&j| (*cutters[j]).clone()).collect();
            crate::solid::primitives::join(&parts)?
        };
        result = pairwise(&result, &cutter)?;
        if result.indices.is_empty() {
            break;
        }
    }
    Ok(result)
}

/// Reconnect triangulated planar face boundaries after exact planar remeshing.
pub(crate) fn stitch_mesh(mesh: &Mesh, eps: f64) -> Result<Mesh> {
    let mut budget = Budget {
        stage: "stitch",
        work: 0,
        fragments: 0,
        options: Options::default(),
    };
    let p = polygons(mesh, eps, &mut budget)?;
    stitch(p, eps, &mut budget)
}


/// First disallowed contact at a relative tolerance, not an exact B-rep certificate.
/// Triangle indices refer to the original input. Degenerate inputs are refused.
#[derive(Clone, Debug)]
pub struct MeshIntersectionLocation {
    pub triangles: [usize; 2],
    pub point: Point,
    pub shared_vertices: usize,
    pub tolerance_mm: f64,
}
pub fn first_mesh_intersection(mesh: &Mesh, relative_tolerance: f64, max_work: usize) -> Result<Option<MeshIntersectionLocation>> {
    let (normalized,origin,extent)=intersection_mesh(mesh,relative_tolerance,max_work)?;
    let mut budget=Budget{stage:"mesh intersection inspection",work:0,fragments:0,
        options:Options{max_work,..Options::default()}};
    Ok(validation::first_geometry_failure(&normalized,relative_tolerance,&mut budget)?.map(|(a,b,p,shared)|MeshIntersectionLocation{
        triangles:[a,b],point:std::array::from_fn(|i|p[i]*extent+origin[i]),shared_vertices:shared,tolerance_mm:relative_tolerance*extent,
    }))
}

pub(super) fn intersection_mesh(mesh:&Mesh,relative_tolerance:f64,max_work:usize)->Result<(Mesh,Point,f64)> {
    mesh.validate()?;
    if !(1e-12..=1e-5).contains(&relative_tolerance) || max_work==0 || max_work>8_000_000 {
        return Err(invalid("Invalid intersection tolerance or work budget"));
    }
    if mesh.indices.is_empty(){return Ok((mesh.clone(),[0.;3],1.));}
    let (lo,hi)=bounds(mesh).ok_or_else(||invalid("Mesh bounds unavailable"))?;
    let extent=(0..3).map(|i|hi[i]-lo[i]).fold(0.,f64::max);
    if !extent.is_finite() || extent<=0. {return Err(invalid("Degenerate mesh bounds"));}
    let origin:Point=std::array::from_fn(|i|lo[i]/2.+hi[i]/2.);
    let mut normalized=mesh.clone();
    for p in normalized.positions.as_chunks_mut::<3>().0 {
        for i in 0..3 {p[i]=(p[i]-origin[i])/extent;}
    }
    for t in normalized.indices.as_chunks::<3>().0 {
        let a=normalized.point(t[0])?;let b=normalized.point(t[1])?;let c=normalized.point(t[2])?;
        if norm(cross(sub(b,a),sub(c,a)))<=relative_tolerance*relative_tolerance {
            return Err(invalid("Degenerate triangle: repair it before intersection inspection"));
        }
    }
    Ok((normalized,origin,extent))
}

/// Every disallowed triangle pair, with explicit incomplete results at resource limits.
/// This checks the displayed triangle mesh at a relative tolerance, not the source B-rep.
#[derive(Clone, Debug)]
pub struct MeshIntersectionReport {
    pub contacts: Vec<MeshIntersectionLocation>,
    pub complete: bool,
    pub stop_reason: Option<&'static str>,
    pub work: usize,
}
pub fn mesh_intersections(mesh:&Mesh,relative_tolerance:f64,max_work:usize,max_contacts:usize)->Result<MeshIntersectionReport> {
    if max_contacts==0 || max_contacts>100_000 {return Err(invalid("Invalid intersection contact limit"));}
    let (normalized,origin,extent)=intersection_mesh(mesh,relative_tolerance,max_work)?;
    let mut budget=Budget {stage:"mesh intersection inspection",work:0,fragments:0,options:Options{max_work,..Options::default()}};
    let mut report=validation::geometry_failures(&normalized,relative_tolerance,&mut budget,Some(max_contacts))?;
    report.contacts.sort_by_key(|&(a,b,_,_)|(a,b));
    Ok(MeshIntersectionReport {
        contacts:report.contacts.into_iter().map(|(a,b,p,shared)|MeshIntersectionLocation {
            triangles:[a,b],point:std::array::from_fn(|i|p[i]*extent+origin[i]),shared_vertices:shared,tolerance_mm:relative_tolerance*extent,
        }).collect(),
        complete:report.complete,stop_reason:report.stop_reason,work:budget.work.min(max_work),
    })
}
