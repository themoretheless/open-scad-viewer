//! Bounded proof of a simple outer UV loop with disjoint, oppositely oriented
//! holes. All conclusions require exact joins and proven loop simplicity.
use crate::{Result,check,curve::Curve,curve_distance,trim_domain::{TrimDomain,Location},trim_simplicity};
pub struct Report {
    /// None is unproven; false is a demonstrated role/orientation violation.
    pub valid:Option<bool>,
    pub reason:&'static str,
    pub loops:Vec<trim_simplicity::Report>,
    pub winding:Vec<Option<i32>>,
    pub pairs:usize,
    pub cells:usize,
    pub domain_cells:usize,
    pub problem_loops:Option<[usize;2]>,
}
fn classify(domain:&TrimDomain,p:[f64;2],out:&mut Report,max:usize)->Result<Option<(Location,Option<i32>)>>{
    if out.domain_cells==max {return Ok(None);}
    let r=domain.classify(p.map(|x|[x,x]),(max-out.domain_cells).min(4096))?;
    out.domain_cells+=r.cells;
    Ok(Some((r.location,r.winding)))
}
pub fn inspect(loops:&[Vec<Curve>],tolerance_uv:f64,max_pairs:usize,max_cells:usize,max_domain_cells:usize)->Result<Report>{
    let count=loops.iter().map(Vec::len).sum::<usize>();
    check((1..=16).contains(&loops.len()) && count<=256 && loops.iter().all(|l|l.len()>=2)
        && (1..=100000).contains(&max_pairs) && (1..=100000).contains(&max_cells)
        && (1..=1000000).contains(&max_domain_cells) && tolerance_uv.is_finite() && tolerance_uv>0.,
        "Trim region audit requires 1..16 loops, at most 256 curves, positive tolerance and bounded work")?;
    let mut out=Report{valid:None,reason:"loop-simplicity-unproven",loops:Vec::new(),winding:vec![None;loops.len()],pairs:0,cells:0,domain_cells:0,problem_loops:None};
    for (i,l) in loops.iter().enumerate(){
        if out.pairs==max_pairs || out.cells==max_cells {return Ok(out);}
        let r=trim_simplicity::inspect(l,tolerance_uv,max_pairs-out.pairs,((max_cells-out.cells)/(loops.len()-i+1)).max(1))?;
        out.pairs+=r.pairs.len();out.cells+=r.cells;
        let proven=r.proven_simple;out.loops.push(r);
        if !proven {out.problem_loops=Some([i,i]);return Ok(out);}
    }
    out.reason="loop-separation-unproven";
    for a in 0..loops.len(){for b in a+1..loops.len(){
        for ca in &loops[a]{for cb in &loops[b]{
            if out.pairs==max_pairs || out.cells==max_cells {out.problem_loops=Some([a,b]);return Ok(out);}
            let r=match curve_distance::prove_separation(ca,cb,tolerance_uv,((max_cells-out.cells)/(count*count).max(1)).max(1)) {
                Ok(report)=>report,
                Err(error) if error.code=="NURBS_RESOURCE_LIMIT"=>{
                    out.pairs+=1;out.problem_loops=Some([a,b]);return Ok(out);
                }
                Err(error)=>return Err(error),
            };
            out.pairs+=1;out.cells+=r.cells;
            if r.distance_interval_mm[0]<=0. {out.problem_loops=Some([a,b]);return Ok(out);}
        }}
    }}
    let domains=loops.iter().map(|l|TrimDomain::new(std::slice::from_ref(l),tolerance_uv)).collect::<Result<Vec<_>>>()?;
    out.reason="loop-interior-unproven";
    // A successful interval winding query proves an interior witness; failure
    // of this finite search never means that the loop has no interior.
    for (i,l) in loops.iter().enumerate(){
        let mut bounds=[[f64::INFINITY,f64::NEG_INFINITY];2];
        for p in l.iter().flat_map(|c|&c.control_points){for k in 0..2 {bounds[k][0]=bounds[k][0].min(p[k]);bounds[k][1]=bounds[k][1].max(p[k]);}}
        'search:for n in [2,4,8,16] {for x in (1..n).step_by(2){for y in (1..n).step_by(2){
            let fractions=[x as f64/n as f64,y as f64/n as f64];
            let p=std::array::from_fn(|k|bounds[k][0]*(1.-fractions[k])+bounds[k][1]*fractions[k]);
            let Some((location,winding))=classify(&domains[i],p,&mut out,max_domain_cells)? else {return Ok(out)};
            if location==Location::Inside && winding.is_some_and(|w|w==1||w==-1){out.winding[i]=winding;break 'search;}
        }}}
        if out.winding[i].is_none(){out.problem_loops=Some([i,i]);return Ok(out);}
    }
    out.reason="hole-containment-unproven";
    for i in 1..loops.len(){
        let p=&loops[i][0].control_points[0];let point=[p[0],p[1]];
        let Some((location,_))=classify(&domains[0],point,&mut out,max_domain_cells)? else{return Ok(out)};
        if location==Location::Unresolved {out.problem_loops=Some([0,i]);return Ok(out);}
        if location==Location::Outside {out.valid=Some(false);out.reason="hole-outside-outer";out.problem_loops=Some([0,i]);return Ok(out);}
        if out.winding[i]==out.winding[0] {out.valid=Some(false);out.reason="hole-orientation";out.problem_loops=Some([0,i]);return Ok(out);}
        for j in 1..loops.len(){if i!=j {
            let Some((location,_))=classify(&domains[j],point,&mut out,max_domain_cells)? else{return Ok(out)};
            if location==Location::Unresolved {out.problem_loops=Some([i,j]);return Ok(out);}
            if location==Location::Inside {out.valid=Some(false);out.reason="nested-holes";out.problem_loops=Some([i,j]);return Ok(out);}
        }}
    }
    out.valid=Some(true);out.reason="simple-outer-with-disjoint-holes";Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn square(lo:f64,hi:f64,reverse:bool)->Vec<Curve>{
        let mut p=vec![[lo,lo],[hi,lo],[hi,hi],[lo,hi]];if reverse{p.reverse();}
        (0..4).map(|i|Curve::from_polyline(vec![p[i].to_vec(),p[(i+1)%4].to_vec()]).unwrap()).collect()
    }
    fn audit(l:Vec<Vec<Curve>>)->Report{inspect(&l,1e-8,1000,10000,100000).unwrap()}
    #[test]
    fn multispan_rational_outer_with_hole_is_audited_without_chords(){
        let mut outer=square(0.,10.,false);
        outer[0]=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],control_points:vec![vec![0.,0.],vec![2.5,-1.],vec![7.5,-1.],vec![10.,0.]],weights:vec![1.,0.8,1.2,1.],periodic:false};
        let loops=vec![outer,square(2.,4.,true)];let before=format!("{loops:?}");
        let r=audit(loops.clone());assert_eq!(r.valid,Some(true));assert!(r.loops[0].injective[0]);assert_eq!(r.winding,vec![Some(1),Some(-1)]);
        assert_eq!(format!("{loops:?}"),before);
        let limited=inspect(&loops,1e-8,1000,1,100000).unwrap();assert_eq!(limited.valid,None);assert!(limited.cells<=1);
    }
    #[test]
    fn correct_hole_and_reversed_whole_region_are_admitted(){
        for reverse in [false,true]{let r=audit(vec![square(0.,10.,reverse),square(2.,4.,!reverse)]);assert_eq!(r.valid,Some(true));assert!(r.cells<=10000&&r.domain_cells<=100000);}
    }
    #[test]
    fn wrong_orientation_outside_nested_and_touching_holes_are_not_admitted(){
        for (loops,reason) in [
            (vec![square(0.,10.,false),square(2.,4.,false)],"hole-orientation"),
            (vec![square(0.,10.,false),square(12.,14.,true)],"hole-outside-outer"),
            (vec![square(0.,10.,false),square(2.,8.,true),square(3.,4.,true)],"nested-holes"),
        ]{let r=audit(loops);assert_eq!(r.valid,Some(false));assert_eq!(r.reason,reason);assert!(r.problem_loops.is_some());}
        assert_eq!(audit(vec![square(0.,10.,false),square(0.,4.,true)]).valid,None);
    }
    #[test]
    fn resource_limits_preserve_unknown_and_do_not_modify_inputs(){
        let loops=vec![square(0.,10.,false),square(2.,4.,true)];let before=format!("{loops:?}");
        for (pairs,cells,domains) in [(1,10000,100000),(1000,1,100000),(1000,10000,1)]{
            let r=inspect(&loops,1e-8,pairs,cells,domains).unwrap();assert_eq!(r.valid,None);assert!(r.pairs<=pairs&&r.cells<=cells&&r.domain_cells<=domains);
        }
        assert_eq!(format!("{loops:?}"),before);
    }
}
