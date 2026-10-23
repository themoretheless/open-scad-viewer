//! Exact separating planes of positive rational control hulls.
use cad_predicates::{AuthoredScalar,SourceArena,ToleranceContext,PredicateContext,Limits,Outcome,Sign};
use nurbs_core::surface::Surface;
/// Each attempted plane consumes one contact cell. Predicate work is bounded
/// independently; exhausted work or unsupported nets retain unresolved status.
pub(crate) fn inspect(a:&Surface,b:&Surface,max_cells:usize)->(bool,usize) {
    let points=[a,b].map(|s|s.control_points.iter().flatten().collect::<Vec<_>>());
    if max_cells==0 || points.iter().any(|ps|ps.len()>16) {return (false,0);}
    let values=points.iter().flatten().flat_map(|p|p.iter())
        .map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect();
    let Ok(source)=SourceArena::authored("contact-control-hulls",1,values) else {return (false,0);};
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:100000,..Limits::default()},None);
    let refs=|i:usize|std::array::from_fn(|k|source.leaf(3*i+k).unwrap());
    let mut cells=0;
    let mut offset=0;
    for face in 0..2 {
        let n=points[face].len();
        for i in 0..n {for j in i+1..n {for k in j+1..n {
            if cells==max_cells {return (false,cells);}
            cells+=1;
            let mut sides=[None,None]; let mut valid=true;
            for other in 0..2 {
                let start=if other==0 {0}else{points[0].len()};
                for p in start..start+points[other].len() {
                    let Ok(proof)=cad_predicates::orient3d(&mut ctx,refs(offset+i),refs(offset+j),refs(offset+k),refs(p))
                        else {return (false,cells);};
                    let Outcome::Sign(sign)=proof.outcome else {return (false,cells);};
                    if sign==Sign::Zero {
                        if other!=face {valid=false;break;}
                    } else {
                        if sides[other].is_some_and(|s|s!=sign) {valid=false;break;}
                        sides[other]=Some(sign);
                    }
                }
                if !valid {break;}
            }
            if valid && sides[1-face].is_some()
                && (sides[face].is_none() || sides[face]!=sides[1-face]) {return (true,cells);}
        }}}
        offset+=n;
    }
    (false,cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patch(offset:f64)->Surface {
        Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,offset],vec![0.,1.,offset]],
                vec![vec![1.,0.,0.5+offset],vec![1.,1.,0.5+offset]]],
            weights:vec![vec![1.,2.],vec![3.,4.]],periodic_u:false,periodic_v:false}
    }
    #[test]
    fn exact_oblique_gap_is_proved_but_touch_overlap_and_zero_budget_are_not() {
        let a=patch(0.);let b=patch(0.25);
        assert!(inspect(&a,&b,64).0);
        assert!(inspect(&b,&a,64).0);
        assert_eq!(inspect(&a,&b,0),(false,0));
        assert!(!inspect(&a,&a,64).0);
        let mut crossed=b.clone();crossed.control_points[0][0][2]=-0.25;
        assert!(!inspect(&a,&crossed,64).0);
        let original=a.clone();inspect(&a,&b,64);assert_eq!(a,original);
    }
}
