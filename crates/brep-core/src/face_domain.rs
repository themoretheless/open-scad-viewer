//! Prepare the authored UV loops of a B-rep face for conservative classification.
//! This does not certify surface/edge agreement or validity of the whole solid.
use crate::{Error, Model, Result};
use nurbs_core::trim_domain::{Classification, TrimDomain};

pub struct FaceDomain {
    pub(crate) region: TrimDomain,
    natural: [[f64; 2]; 2],
}
impl FaceDomain {
    pub fn new(model: &Model, face_index: usize, tolerance_uv: f64) -> Result<Self> {
        let face = model.faces.get(face_index).ok_or_else(|| {
            Error::new(
                "BREP_INVALID_INPUT",
                "Choose an existing face for UV classification",
            )
        })?;
        face.surface.validate()?;
        let loops = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .map(|index| {
                let loop_ = model.loops.get(index).ok_or_else(|| {
                    Error::new("BREP_INVALID_TOPOLOGY", "Face references a missing UV loop")
                })?;
                // Pcurves already run in the loop traversal direction. `reversed`
                // applies to the 3D edge; applying it again here breaks the UV loop.
                Ok(loop_.coedges.iter().map(|c| c.pcurve.clone()).collect())
            })
            .collect::<Result<Vec<_>>>()?;
        let s = &face.surface;
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        Ok(Self {
            region: TrimDomain::new(&loops, tolerance_uv)?,
            natural,
        })
    }
    pub fn classify(&self, rectangle: [[f64; 2]; 2], max_cells: usize) -> Result<Classification> {
        if !(0..2)
            .all(|k| rectangle[k][0] >= self.natural[k][0] && rectangle[k][1] <= self.natural[k][1])
        {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "UV rectangle must lie in the face surface domain",
            ));
        }
        self.region.classify(rectangle, max_cells)
    }
}

/// Distance between two selected trimmed faces, not between filled body volumes.
pub fn distance_between_faces(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<nurbs_core::trimmed_surface_distance::TrimmedDistance> {
    a.validate()?;
    b.validate()?;
    let da = FaceDomain::new(a, face_a, tolerance_uv)?;
    let db = FaceDomain::new(b, face_b, tolerance_uv)?;
    nurbs_core::trimmed_surface_distance::distance(
        &a.faces[face_a].surface,
        &da.region,
        &b.faces[face_b].surface,
        &db.region,
        tolerance_mm,
        max_cells,
        max_domain_cells,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::{curve::Curve, trim_domain::Location};
    #[test]
    fn concentric_retained_walls_have_certified_clearance_with_one_geometry_cell() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let before=format!("{model:?}");
        let r=distance_between_faces(&model,16,&model,17,1e-6,1e-8,1,10000).unwrap();
        assert!(r.converged,"{r:?}");
        assert!(r.lower_bound_mm<=15.&&r.upper_bound_mm.unwrap()>=15.);
        assert!(r.upper_bound_mm.unwrap()-r.lower_bound_mm<1e-6);
        assert!(r.parameters.is_some()&&r.point_enclosures.is_some());
        assert_eq!(r.cells,1);
        assert_eq!(format!("{model:?}"),before);
    }
    #[test]
    fn box_faces_use_pcurves_in_loop_order() {
        let model = crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap();
        assert!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .any(|c| c.reversed)
        );
        for i in 0..model.faces.len() {
            let d = FaceDomain::new(&model, i, 1e-7).unwrap();
            assert_eq!(
                d.classify([[0.5; 2]; 2], 1000).unwrap().location,
                Location::Inside
            );
        }
        assert!(FaceDomain::new(&model, 100, 1e-7).is_err());
    }
    #[test]
    fn cap_hole_is_excluded_and_boundary_cells_stay_unresolved() {
        let outer = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![10., 0.],
            vec![10., 10.],
            vec![0., 10.],
            vec![0., 0.],
        ])
        .unwrap();
        let hole = Curve::from_polyline(vec![
            vec![3., 3.],
            vec![3., 7.],
            vec![7., 7.],
            vec![7., 3.],
            vec![3., 3.],
        ])
        .unwrap();
        let model = crate::prism::extrude(&[vec![outer], vec![hole]], 0., 2.).unwrap();
        model.validate().unwrap();
        let d = FaceDomain::new(&model, model.faces.len() - 1, 1e-7).unwrap();
        assert_eq!(
            d.classify([[0.5; 2]; 2], 1000).unwrap().location,
            Location::Outside
        );
        assert_eq!(
            d.classify([[0.1, 0.2], [0.1, 0.2]], 1000).unwrap().location,
            Location::Inside
        );
        assert_eq!(
            d.classify([[0.2, 0.4], [0.4, 0.6]], 1000).unwrap().location,
            Location::Unresolved
        );
        assert!(d.classify([[-0.1, 0.2], [0.1, 0.2]], 1000).is_err());
    }
}

pub struct TrimAudit {
    /// Missing entries remain explicit for unsupported or unvisited faces.
    pub faces: Vec<Option<nurbs_core::trim_region_audit::Report>>,
    pub all_valid: bool,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
/// UV-region evidence only; edge lifts, surface embedding and volume validity
/// have separate checks. Every budget is shared by all inspected faces.
pub fn audit_trim_regions(model: &Model,tolerance_uv:f64,max_pairs:usize,max_cells:usize,max_domain_cells:usize)->Result<TrimAudit>{
    model.validate_boundary_diagnostic_inputs()?;
    if !(1..=100000).contains(&max_pairs)||!(1..=100000).contains(&max_cells)||!(1..=1000000).contains(&max_domain_cells)
        || !tolerance_uv.is_finite()||tolerance_uv<=0. {
        return Err(Error::new("BREP_INVALID_INPUT","Trim audit requires positive tolerance and bounded work"));
    }
    let mut out=TrimAudit{faces:Vec::new(),all_valid:!model.faces.is_empty(),pairs:0,cells:0,domain_cells:0};
    for face in &model.faces {
        let loops=std::iter::once(face.outer).chain(face.holes.iter().copied())
            .map(|wire|model.loops[wire].coedges.iter().map(|c|c.pcurve.clone()).collect::<Vec<_>>()).collect::<Vec<_>>();
        let result=if out.pairs<max_pairs&&out.cells<max_cells&&out.domain_cells<max_domain_cells
            && loops.len()<=16&&loops.iter().all(|l|l.len()>=2)&&loops.iter().map(Vec::len).sum::<usize>()<=256 {
            Some(nurbs_core::trim_region_audit::inspect(&loops,tolerance_uv,max_pairs-out.pairs,max_cells-out.cells,max_domain_cells-out.domain_cells)?)
        }else{None};
        if let Some(r)=&result{out.pairs+=r.pairs;out.cells+=r.cells;out.domain_cells+=r.domain_cells;}
        out.all_valid &= result.as_ref().is_some_and(|r|r.valid==Some(true));out.faces.push(result);
    }
    Ok(out)
}
#[cfg(test)]
mod trim_audit_tests {
    use super::*;
    #[test]
    fn cube_regions_and_global_exhaustion_preserve_face_indices(){
        let m=crate::cuboid([0.;3],[1.;3]).unwrap();let before=format!("{m:?}");
        let r=audit_trim_regions(&m,1e-8,1000,10000,100000).unwrap();
        assert!(r.all_valid);assert_eq!(r.faces.len(),6);
        for (p,c,d) in [(1,10000,100000),(1000,1,100000),(1000,10000,1)]{
            let r=audit_trim_regions(&m,1e-8,p,c,d).unwrap();
            assert!(!r.all_valid);assert_eq!(r.faces.len(),6);assert!(r.pairs<=p&&r.cells<=c&&r.domain_cells<=d);
        }
        assert_eq!(format!("{m:?}"),before);
    }
}
