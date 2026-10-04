//! Separation and point-parity evidence for shell nesting. The interpretation
//! as containment requires independently certified connected embedded shells;
//! this module does not certify volume validity or orientation.
use crate::{Error, Model, Result, ray_parity, shell_distance};

pub struct Report {
    pub boundary_separation_certified: bool,
    pub boundary: shell_distance::ShellDistance,
    /// A point of A classified against B, then a point of B against A.
    pub witness_parity: [Option<ray_parity::PointReport>; 2],
    pub reason: &'static str,
}

/// Inspect two individual closed shells. Strict boundary separation is required
/// before classifying witnesses. A touching or unresolved pair never acquires a
/// containment answer merely from a convenient sample point.
pub fn inspect(
    a: &Model, b: &Model, tolerance_mm: f64, tolerance_uv: f64,
    max_cells: usize, max_domain_cells: usize,
) -> Result<Report> {
    for m in [a, b] {
        m.validate()?;
        if m.shells.len() != 1 || !m.shells[0].closed
            || m.shells[0].faces.len() != m.faces.len() || m.faces.is_empty()
        {
            return Err(Error::new("BREP_INVALID_INPUT", "Shell relation requires one nonempty closed shell per model"));
        }
    }
    // Half of each global budget is reserved for distance; unused work is
    // available to the two point classifications.
    if max_cells < 2 || max_cells > 1000000 || max_domain_cells < 2 || max_domain_cells > 8000000 {
        return Err(Error::new("BREP_INVALID_INPUT", "Shell relation requires 2..1000000 geometry cells and 2..8000000 domain cells"));
    }
    let boundary = shell_distance::distance(a, b, tolerance_mm, tolerance_uv, max_cells / 2, max_domain_cells / 2)?;
    let mut cells = max_cells - boundary.cells;
    let mut domains = max_domain_cells - boundary.domain_cells;
    let mut report = Report { boundary_separation_certified:boundary.lower_bound_mm>0.,boundary, witness_parity: [None, None], reason: "boundary-separation-unproven" };
    if report.boundary.lower_bound_mm <= 0. { return Ok(report); }
    report.reason = "boundary-witness-unavailable";
    let Some(witness) = report.boundary.witness.as_ref() else { return Ok(report); };
    let (Some(points), Some(boxes)) = (witness.points, witness.point_enclosures.as_ref()) else { return Ok(report); };
    // A rounded displayed point must be in the same complement component as
    // the true surface witness. Its entire error ball is smaller than the
    // proven distance to the opposite shell. L1 bounds the Euclidean error.
    for side in 0..2 {
        let mut radius = 0_f64;
        for axis in 0..3 {
            let delta = (points[side][axis] - boxes[side][axis][0]).abs()
                .max((points[side][axis] - boxes[side][axis][1]).abs()).next_up();
            radius = (radius + delta).next_up();
        }
        if !radius.is_finite() || radius >= report.boundary.lower_bound_mm {
            report.reason = "witness-rounding-unresolved";
            return Ok(report);
        }
    }
    let directions = [[1., 0.317, 0.173], [-0.219, 1., 0.413], [0.271, -0.193, 1.]];
    for side in 0..2 {
        if cells == 0 || domains == 0 { break; }
        let result = ray_parity::classify_point(if side == 0 {b} else {a}, points[side], &directions,
            tolerance_uv, (cells / (2-side)).max(1), (domains / (2-side)).max(1))?;
        cells -= result.cells;
        domains -= result.domain_cells;
        report.witness_parity[side] = Some(result);
    }
    report.reason = if report.witness_parity.iter().all(|r| r.as_ref().is_some_and(|r| r.parity.is_some())) {
        "separated-witness-parities"
    } else { "point-parity-unresolved" };
    Ok(report)
}

/// Used only after the volume audit's fresh pair certificates prove every
/// cross-shell face pair disjoint. Exact clamped edge poles need no rounded
/// surface-evaluation witness or quantitative clearance estimate.
pub(crate) fn classify_exact_boundary_witnesses(a:&Model,b:&Model,report:&mut Report,
    tolerance_uv:f64,max_cells:usize,max_domains:usize)->Result<()> {
    let witness=|model:&Model|->Option<[f64;3]>{
        let face=model.faces.get(model.shells[0].faces[0].face)?;
        let edge=model.loops.get(face.outer)?.coedges.first()?.edge;
        let curve=&model.edges.get(edge)?.curve;
        let p=curve.degree;let n=curve.control_points.len();
        if curve.periodic || curve.knots[..p+1].iter().any(|k|*k!=curve.knots[p])
            || curve.knots[n..].iter().any(|k|*k!=curve.knots[n]) {return None;}
        let pole=curve.control_points.first()?;
        (pole.len()==3).then(||[pole[0],pole[1],pole[2]])
    };
    let (Some(pa),Some(pb))=(witness(a),witness(b)) else {return Ok(());};
    let mut cells=max_cells-report.boundary.cells;
    let mut domains=max_domains-report.boundary.domain_cells;
    report.boundary_separation_certified=true;
    let directions=[[1.,0.317,0.173],[-0.219,1.,0.413],[0.271,-0.193,1.]];
    for (side,point) in [pa,pb].into_iter().enumerate(){
        if cells==0 || domains==0 {break;}
        let r=ray_parity::classify_point(if side==0 {b}else{a},point,&directions,tolerance_uv,
            (cells/(2-side)).max(1),(domains/(2-side)).max(1))?;
        cells-=r.cells;domains-=r.domain_cells;
        report.witness_parity[side]=Some(r);
    }
    report.reason=if report.witness_parity.iter().all(|r|r.as_ref().is_some_and(|r|r.parity.is_some())){
        "certified-boundaries-exact-witness-parities"
    }else{"certified-boundaries-point-parity-unresolved"};
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_and_separate_shells_keep_both_parities_and_global_budget() {
        let a = crate::cuboid([0.;3], [10.;3]).unwrap();
        for (min, max, expected) in [([2.;3], [8.;3], [false,true]), ([12.;3], [14.;3], [false,false])] {
            let b = crate::cuboid(min,max).unwrap();
            let r = inspect(&a,&b,1e-5,1e-7,20000,200000).unwrap();
            assert_eq!(r.reason,"separated-witness-parities");
            for i in 0..2 { assert_eq!(r.witness_parity[i].as_ref().unwrap().parity,Some(expected[i])); }
            assert!(r.boundary.cells + r.witness_parity.iter().flatten().map(|r|r.cells).sum::<usize>() <= 20000);
            assert!(r.boundary.domain_cells + r.witness_parity.iter().flatten().map(|r|r.domain_cells).sum::<usize>() <= 200000);
        }
    }
    #[test]
    fn rotation_swapping_and_exhaustion_preserve_the_contract() {
        let a = crate::cuboid([0.;3], [10.;3]).unwrap();
        let b = crate::cuboid([2.;3], [8.;3]).unwrap();
        let (s,c) = 0.37_f64.sin_cos();
        let matrix = [[c,-s,0.,13.],[s,c,0.,-7.],[0.,0.,1.,3.],[0.,0.,0.,1.]];
        let a = crate::transform::affine(&a,matrix).unwrap();
        let b = crate::transform::affine(&b,matrix).unwrap();
        let before = format!("{a:?}{b:?}");
        let r = inspect(&b,&a,1e-4,1e-7,20000,200000).unwrap();
        assert_eq!(r.reason,"separated-witness-parities");
        assert_eq!(r.witness_parity[0].as_ref().unwrap().parity,Some(true));
        assert_eq!(r.witness_parity[1].as_ref().unwrap().parity,Some(false));
        let low = inspect(&a,&b,1e-4,1e-7,2,2).unwrap();
        assert_ne!(low.reason,"separated-witness-parities");
        assert_eq!(format!("{a:?}{b:?}"),before);
        assert!(inspect(&a,&b,1e-4,1e-7,0,100).is_err());
    }
    #[test]
    fn rational_spheres_and_authored_cavity_shells_keep_nesting_evidence() {
        let a = crate::sphere(5.).unwrap();
        let b = crate::sphere(2.).unwrap();
        let r = inspect(&a,&b,0.01,1e-7,100000,1000000).unwrap();
        assert_eq!(r.reason,"separated-witness-parities");
        assert_eq!(r.witness_parity[0].as_ref().unwrap().parity,Some(false));
        assert_eq!(r.witness_parity[1].as_ref().unwrap().parity,Some(true));
        let a = crate::cuboid([0.;3], [10.;3]).unwrap();
        let b = crate::cuboid([2.;3], [8.;3]).unwrap();
        let cavity = crate::operations::boolean(&a,&b,"difference").unwrap();
        let body = &cavity.bodies[0];
        let outer = crate::solid_audit::isolated_outward_shell(&cavity,body.outer_shell,false).unwrap();
        let inner = crate::solid_audit::isolated_outward_shell(&cavity,body.inner_shells[0],true).unwrap();
        let r = inspect(&outer,&inner,1e-5,1e-7,20000,200000).unwrap();
        assert_eq!(r.reason,"separated-witness-parities");
        assert_eq!(r.witness_parity[0].as_ref().unwrap().parity,Some(false));
        assert_eq!(r.witness_parity[1].as_ref().unwrap().parity,Some(true));
        // A multi-shell body cannot accidentally be treated as one boundary.
        assert!(inspect(&cavity,&b,1e-5,1e-7,20000,200000).is_err());
    }
    #[test]
    fn touching_shells_cannot_be_classified_by_samples() {
        let a = crate::cuboid([0.;3],[1.;3]).unwrap();
        let b = crate::cuboid([1.,0.,0.],[2.,1.,1.]).unwrap();
        let r = inspect(&a,&b,1e-5,1e-7,1000,10000).unwrap();
        assert_eq!(r.reason,"boundary-separation-unproven");
        assert!(r.witness_parity.iter().all(Option::is_none));
    }
}
