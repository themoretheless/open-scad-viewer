//! Exploratory closed UV-product normal exclusion on immutable original inputs.
//! This emits diagnostics, never a wall or material-volume certificate.
use nurbs_core::{normal_alignment::OriginalNormalCache, surface::Surface};
fn transpose_parameters(s: &Surface) -> Surface {
    let rows=s.control_points.len();let columns=s.control_points[0].len();
    Surface {degree_u:s.degree_v,degree_v:s.degree_u,knots_u:s.knots_v.clone(),knots_v:s.knots_u.clone(),
        control_points:(0..columns).map(|v|(0..rows).map(|u|s.control_points[u][v].clone()).collect()).collect(),
        weights:(0..columns).map(|v|(0..rows).map(|u|s.weights[u][v]).collect()).collect(),
        periodic_u:s.periodic_v,periodic_v:s.periodic_u}
}
fn split_product(pair: [[[f64;2];2];2], side: usize, axis: usize, mid: f64)
    -> [[[[f64;2];2];2];2] {
    assert!(side<2 && axis<2 && pair[side][axis][0]<mid && mid<pair[side][axis][1]);
    let mut left=pair;let mut right=pair;
    left[side][axis][1]=mid;right[side][axis][0]=mid;[left,right]
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let cell_limit: usize = args.get(4).map(|v|v.parse().unwrap()).unwrap_or(128);
    assert!((1..=100000).contains(&cell_limit));
    let span_limit=(cell_limit*8).min(100000);
    let request: value_codec::Value = value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let classification: value_codec::Value = value_codec::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let source = &request["definition"]["shell"]["regions"];
    let surfaces: Vec<Surface> = source.as_array().unwrap().iter().map(|r| {
        let s = &r["surface"];
        Surface {
            degree_u: value_codec::from_value(s["degreeU"].clone()).unwrap(),
            degree_v: value_codec::from_value(s["degreeV"].clone()).unwrap(),
            knots_u: value_codec::from_value(s["knotsU"].clone()).unwrap(),
            knots_v: value_codec::from_value(s["knotsV"].clone()).unwrap(),
            control_points: value_codec::from_value(s["controlPoints"].clone()).unwrap(),
            weights: value_codec::from_value(s["weights"].clone()).unwrap(),
            periodic_u: value_codec::from_value(s["periodicU"].clone()).unwrap(),
            periodic_v: value_codec::from_value(s["periodicV"].clone()).unwrap(),
        }
    }).collect();
    let before = value_codec::to_string(&surfaces).unwrap();
    let domains: Vec<_> = surfaces.iter().map(|s| [[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]]).collect();
    let mut limiting_normals=Vec::new();
    if args.get(5).is_some_and(|v|v=="limits") {
        for (face,original) in surfaces.iter().enumerate() {for axis in 0..2 {
            let exchanged=transpose_parameters(original);
            assert_eq!(transpose_parameters(&exchanged),*original);
            let surface=if axis==0 {original} else {&exchanged};
            let domain=if axis==0 {domains[face]} else {[domains[face][1],domains[face][0]]};
            for end in [false,true] {
                let row=match nurbs_core::factored_surface_normals::prepare(surface,end,100_000_000) {
                    Ok(report)=>{
                        let bounds=report.certificate.as_ref().map(|c|{
                            assert!(std::ptr::eq(c.surface(),surface));c.bounds(domain).unwrap()
                        });
                        value_codec::json!({"face":face,"axis":axis,"end":end,"qualified":report.certificate.is_some(),"work":report.work,"reason":report.reason,"boundsOnPreparedChart":bounds,"parameterExchanged":axis==1})
                    }
                    Err(error)=>value_codec::json!({"face":face,"axis":axis,"end":end,"qualified":false,"error":error.to_string()}),
                };
                limiting_normals.push(row);
            }
        }}
    }
    let mut caches: Vec<_> = surfaces.iter().map(OriginalNormalCache::new).collect();
    let sine = 1e-6_f64;
    let threshold = ((4.*sine).next_up()*(1.-sine).next_up()).next_up();
    let mut rows = Vec::new();
    let pairs = classification["groups"]["distinct-no-shared-edge"].as_array().unwrap();
    for record in pairs {
        let [a,b]: [usize;2] = value_codec::from_value(record["faces"].clone()).unwrap();
        assert!(a<b && b<surfaces.len());
        let original = [domains[a],domains[b]];
        let mut pending = vec![original]; let mut cells=0; let mut spans=0; let mut reason="work-limit";
        let (left,right) = caches.split_at_mut(b);
        while cells<cell_limit && !pending.is_empty() {
            let pair=pending.pop().unwrap(); cells+=1;
            let r=left[a].inspect_other(&mut right[0],pair,threshold,span_limit-spans).unwrap(); spans+=r.spans;
            if r.aligned==Some(false) {continue;}
            if r.aligned==Some(true) {pending.push(pair);reason="aligned-region";break;}
            if spans==span_limit {pending.push(pair);reason="span-limit";break;}
            let mut choice=(0,0);let mut largest=0.;
            for side in 0..2 {for axis in 0..2 {
                let width=(pair[side][axis][1]-pair[side][axis][0])/(original[side][axis][1]-original[side][axis][0]);
                if width>largest {largest=width;choice=(side,axis);}
            }}
            let (side,axis)=choice;let d=pair[side][axis];let mid=d[0]*0.5+d[1]*0.5;
            if mid<=d[0] || mid>=d[1] {pending.push(pair);reason="resolution-limit";break;}
            let [l,r]=split_product(pair,side,axis,mid);
            pending.push(r);pending.push(l);
        }
        if pending.is_empty() {reason="normal-product-excluded";}
        rows.push(value_codec::json!({"faces":[a,b],"cells":cells,"spans":spans,"normalExcluded":pending.is_empty(),"reason":reason,"unresolvedPairs":pending}));
        let report=value_codec::json!({"scope":"Exploratory original closed UV-product normal exclusion only; no wall certificate", "completeAudit":rows.len()==pairs.len(),"expectedPairs":pairs.len(),"limitingNormals":limiting_normals,"maxSineSquared":sine,"necessaryPairSineSquared":threshold,"cellsPerPair":cell_limit,"spansPerPair":span_limit,"rows":rows});
        let temporary=format!("{}.tmp",args[3]);std::fs::write(&temporary,value_codec::to_string_pretty(&report).unwrap()).unwrap();std::fs::rename(temporary,&args[3]).unwrap();
    }
    assert_eq!(before,value_codec::to_string(&surfaces).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parameter_exchange_preserves_every_authored_coordinate_and_weight_bit() {
        let s=Surface{degree_u:1,degree_v:2,knots_u:vec![2.,2.,5.,5.],knots_v:vec![-1.,-1.,-1.,3.,3.,3.],
            control_points:vec![vec![vec![-0.,1.,2.],vec![3.,4.,5.],vec![6.,7.,8.]],vec![vec![9.,10.,11.],vec![12.,13.,14.],vec![15.,16.,17.]]],
            weights:vec![vec![1.,2.,3.],vec![4.,5.,6.]],periodic_u:false,periodic_v:false};
        let t=transpose_parameters(&s);assert_eq!(t.degree_u,s.degree_v);assert_eq!(t.knots_u,s.knots_v);
        let back=transpose_parameters(&t);assert_eq!(back,s);
        for u in 0..2 {for v in 0..3 {
            assert_eq!(t.weights[v][u].to_bits(),s.weights[u][v].to_bits());
            for k in 0..3 {assert_eq!(t.control_points[v][u][k].to_bits(),s.control_points[u][v][k].to_bits());}
        }}
    }
    #[test]
    fn closed_products_cover_every_split_boundary_and_keep_other_coordinates() {
        let pair=[[[2.,6.],[-3.,1.]],[[10.,14.],[20.,24.]]];
        for side in 0..2 {for axis in 0..2 {
            let d=pair[side][axis];let mid=(d[0]+d[1])*0.5;
            let children=split_product(pair,side,axis,mid);
            for child in children {for s in 0..2 {for a in 0..2 {
                if (s,a)!=(side,axis) {assert_eq!(child[s][a],pair[s][a]);}
            }}}
            for i in 0..625 {
                let mut code=i;let mut point=[[0.;2];2];
                for s in 0..2 {for a in 0..2 {
                    let fraction=(code%5) as f64/4.;code/=5;
                    point[s][a]=pair[s][a][0]+fraction*(pair[s][a][1]-pair[s][a][0]);
                }}
                assert!(children.iter().any(|child|(0..2).all(|s|(0..2).all(|a|
                    child[s][a][0]<=point[s][a] && point[s][a]<=child[s][a][1]))));
            }
        }}
    }
}
