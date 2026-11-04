//! Both directed covers exhaust complete original parameter forests. Every
//! retained source point has a sealed whole-image material proof in the target.
use super::*;
use nurbs_core::trim_domain::Location;
use std::collections::VecDeque;
pub(super) struct Covers {
    pub work: RootCavityCoverageWork,
    // Keep the exact source geometry, UV restrictions and original target owners.
    _forward: Vec<closed::QualifiedSurfaceMaterialBox>,
    _reverse: Vec<QualifiedRootSurfaceMaterialBox>,
}
fn roots(
    surface: &crate::Surface,
    restriction: Option<[[f64; 2]; 2]>,
    max: usize,
) -> Result<VecDeque<([[f64; 2]; 2], usize)>> {
    let mut out = VecDeque::new();
    for u in surface.degree_u..surface.control_points.len() {
        for v in surface.degree_v..surface.control_points[0].len() {
            let mut d = [
                [surface.knots_u[u], surface.knots_u[u + 1]],
                [surface.knots_v[v], surface.knots_v[v + 1]],
            ];
            if let Some(r) = restriction {
                for k in 0..2 {
                    d[k][0] = d[k][0].max(r[k][0]);
                    d[k][1] = d[k][1].min(r[k][1]);
                }
            }
            if d.iter().any(|x| x[0] >= x[1]) {
                continue;
            }
            if out.len() == max {
                return Err(unproved(
                    "Cavity original span roots exceed coverage allowance",
                ));
            }
            out.push_back((d, 0));
        }
    }
    if out.is_empty() {
        return Err(unproved("Cavity coverage has no original span roots"));
    }
    Ok(out)
}
fn split(
    pending: &mut VecDeque<([[f64; 2]; 2], usize)>,
    d: [[f64; 2]; 2],
    depth: usize,
) -> Result<()> {
    let axis = usize::from(d[1][1] - d[1][0] > d[0][1] - d[0][0]);
    let m = d[axis][0] * 0.5 + d[axis][1] * 0.5;
    if depth == 64 || !(d[axis][0] < m && m < d[axis][1]) {
        return Err(unproved(
            "Cavity contact or source separation precision unresolved",
        ));
    }
    let mut a = d;
    let mut b = d;
    a[axis][1] = m;
    b[axis][0] = m;
    pending.push_back((b, depth + 1));
    pending.push_back((a, depth + 1));
    Ok(())
}
pub(super) fn qualify(
    inner: &Arc<QualifiedPairedRootSolid>,
    outer: &Arc<closed::QualifiedClosedSolid>,
    directions: &[[f64; 3]],
    limits: RootCavityLimits,
) -> Result<Covers> {
    let p = limits.root_material.material.point;
    let dl = RootBoundaryVolumeLimits {
        arc_subdivisions: p.arc_subdivisions,
        band_queries: p.band_queries,
        winding_queries: p.winding_queries,
        winding: p.winding,
        geometry: p.geometry,
        ..Default::default()
    };
    if !(1..=64).contains(&p.arc_subdivisions)
        || !p.arc_subdivisions.is_power_of_two()
        || p.band_queries > 1000000
        || p.winding_queries > 65536
    {
        return Err(unproved("Invalid cavity source-domain work"));
    }
    let mut dw = RootBoundaryVolumeReport {
        signed_bounds: None,
        accuracy_proven: false,
        cells: 0,
        spans: 0,
        band_queries: 0,
        winding_queries: 0,
        seeds: 0,
        propagations: 0,
        reason: "cavity-domain-work",
    };
    let mut region = domain::Domains::new(inner.boundary(), dl, &mut dw)?
        .ok_or_else(|| unproved("Cavity source root-band coverage incomplete"))?;
    let mut out = Covers {
        work: RootCavityCoverageWork {
            cells: [0; 2],
            accepted: [0; 2],
            domain_queries: [0; 2],
        },
        _forward: Vec::new(),
        _reverse: Vec::new(),
    };
    for (face, f) in inner.model().topology().faces.iter().enumerate() {
        let mut pending = roots(&f.surface, Some(region.roots[face]), limits.coverage_cells)?;
        while let Some((d, depth)) = pending.pop_front() {
            if out.work.cells[0] == limits.coverage_cells {
                return Err(unproved("Inner cavity boundary cover exhausted"));
            }
            out.work.cells[0] += 1;
            let mut wrong_material = false;
            if let Ok(proof) = closed::qualify_surface_material_box(
                outer,
                &f.surface,
                d,
                directions,
                limits.original_material,
            ) {
                if proof.inside() {
                    out.work.accepted[0] += 1;
                    out._forward.push(proof);
                    continue;
                }
                wrong_material = true;
            }
            if out.work.domain_queries[0] < limits.domain_queries {
                out.work.domain_queries[0] += 1;
                let mut q = region.classify(face, d, dl, &mut dw)?;
                if q.is_none() {
                    q = region.classify_box(face, d, dl, &mut dw)?;
                }
                if q == Some(false) {
                    continue;
                }
                if q == Some(true) && wrong_material {
                    return Err(unproved(
                        "Retained inner boundary is outside cavity host material",
                    ));
                }
            }
            split(&mut pending, d, depth)?;
        }
    }
    for (face, f) in outer.model().faces.iter().enumerate() {
        let source = crate::face_domain::FaceDomain::new(outer.model(), face, p.tolerance_uv)?;
        let mut pending = roots(&f.surface, None, limits.coverage_cells)?;
        while let Some((d, depth)) = pending.pop_front() {
            if out.work.cells[1] == limits.coverage_cells {
                return Err(unproved("Original host boundary cover exhausted"));
            }
            out.work.cells[1] += 1;
            let mut wrong_material = false;
            if let Ok(proof) =
                inner.qualify_surface_material_box(&f.surface, d, directions, limits.root_material)
            {
                if !proof.inside() {
                    out.work.accepted[1] += 1;
                    out._reverse.push(proof);
                    continue;
                }
                wrong_material = true;
            }
            if out.work.domain_queries[1] < limits.domain_queries {
                out.work.domain_queries[1] += 1;
                let location = source
                    .classify(
                        d,
                        limits.original_material.distance_domain_cells.min(100000),
                    )?
                    .location;
                if location == Location::Outside {
                    continue;
                }
                if location == Location::Inside && wrong_material {
                    return Err(unproved(
                        "Retained host boundary lies inside the proposed cavity",
                    ));
                }
            }
            split(&mut pending, d, depth)?;
        }
    }
    if out.work.accepted.contains(&0) {
        return Err(unproved("Cavity cover has no retained material witnesses"));
    }
    Ok(out)
}
