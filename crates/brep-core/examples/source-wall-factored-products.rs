//! Exploratory composition of source-owned factored normals. No Body certificate.
use nurbs_core::{surface::Surface,interval_eval::Interval as I,Result};
use value_codec::{json,Value};
fn square(x:I)->Result<I>{
    let lo=if x.lo>0. {x.lo}else if x.hi<0. {-x.hi}else{0.};
    let hi=x.lo.abs().max(x.hi.abs());
    I::new((lo*lo).next_down().max(0.),(hi*hi).next_up())
}
fn norm(x:[I;3])->Result<I>{
    let mut sum=I::point(0.);for v in x {sum=sum.add(square(v)?)?;}
    I::new(sum.lo.max(0.),sum.hi)
}
fn angle(bounds:[[[f64;2];3];2],threshold:f64)->Result<Option<bool>>{
    let mut n=[[I::point(0.);3];2];
    for side in 0..2 {
        let hull=bounds[side];
        if hull.iter().all(|b|b[0]<=0.&&b[1]>=0.) {return Ok(None);}
        let scale=hull.iter().flatten().map(|v|v.abs()).fold(0_f64,f64::max);
        if scale<f64::MIN_POSITIVE {return Ok(None);}
        for k in 0..3 {n[side][k]=I::new(hull[k][0],hull[k][1])?.div(I::point(scale))?;}
    }
    let denominator=norm(n[0])?.mul(norm(n[1])?)?;
    if denominator.lo<=0. {return Ok(None);}
    let mut cross=[I::point(0.);3];
    for k in 0..3 {let a=(k+1)%3;let b=(k+2)%3;
        cross[k]=n[0][a].mul(n[1][b])?.sub(n[0][b].mul(n[1][a])?)?;}
    let sine=norm(cross)?.div(denominator)?;
    let interval=[sine.lo.max(0.),sine.hi.min(1.)];I::new(interval[0],interval[1])?;
    Ok(if interval[1]<=threshold {Some(true)}else if interval[0]>threshold {Some(false)}else{None})
}
fn main(){
    let args:Vec<_>=std::env::args().collect();
    let request:Value=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let classification:Value=value_codec::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let surfaces:Vec<Surface>=request["definition"]["shell"]["regions"].as_array().unwrap().iter().map(|r|{
        let s=&r["surface"];Surface{
            degree_u:value_codec::from_value(s["degreeU"].clone()).unwrap(),degree_v:value_codec::from_value(s["degreeV"].clone()).unwrap(),
            knots_u:value_codec::from_value(s["knotsU"].clone()).unwrap(),knots_v:value_codec::from_value(s["knotsV"].clone()).unwrap(),
            control_points:value_codec::from_value(s["controlPoints"].clone()).unwrap(),weights:value_codec::from_value(s["weights"].clone()).unwrap(),
            periodic_u:value_codec::from_value(s["periodicU"].clone()).unwrap(),periodic_v:value_codec::from_value(s["periodicV"].clone()).unwrap()}
    }).collect();
    let before=value_codec::to_string(&surfaces).unwrap();
    let domains:Vec<_>=surfaces.iter().map(|s|[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]]).collect();
    let mut exact_work=0_u64;let mut prepared=Vec::new();let mut preparations=Vec::new();
    for (face,surface) in surfaces.iter().enumerate(){
        let mut certificate=None;
        for end in [false,true] {
            if exact_work==100_000_000 {break;}
            match nurbs_core::factored_surface_normals::prepare(surface,end,100_000_000-exact_work) {
                Ok(r)=>{exact_work+=r.work;preparations.push(json!({"face":face,"end":end,"work":r.work,"reason":r.reason,"prepared":r.certificate.is_some()}));
                    if let Some(c)=r.certificate {assert!(std::ptr::eq(c.surface(),surface));certificate=Some(c);break;}}
                Err(e)=>preparations.push(json!({"face":face,"end":end,"error":e.to_string()})),
            }
        }
        prepared.push(certificate);
    }
    let mut caches:Vec<std::collections::HashMap<[[u64;2];2],[[f64;2];3]>>=(0..surfaces.len()).map(|_|Default::default()).collect();
    let sine=1e-6_f64;let threshold=((4.*sine).next_up()*(1.-sine).next_up()).next_up();
    let mut rows=Vec::new();
    let records=classification["groups"]["shared-edge"].as_array().unwrap().iter()
        .chain(classification["groups"]["distinct-no-shared-edge"].as_array().unwrap());
    for record in records {
        let faces:[usize;2]=value_codec::from_value(record["faces"].clone()).unwrap();
        if faces.iter().all(|f|prepared[*f].is_none()) {continue;}
        let original=faces.map(|f|domains[f]);let mut pending=vec![original];
        let mut cells=0;let mut spans=0;let mut factored_queries=0;let mut reason="cell-limit";
        while cells<128&&!pending.is_empty(){
            let pair=pending.pop().unwrap();cells+=1;let mut components=[None;2];
            for side in 0..2 {
                let face=faces[side];let rect=pair[side];let key=rect.map(|d|d.map(f64::to_bits));
                if let Some(bounds)=caches[face].get(&key){components[side]=Some(*bounds);continue;}
                let bounds=if let Some(c)=&prepared[face]{
                    if factored_queries==1024 {None}else{factored_queries+=1;Some(c.bounds(rect).unwrap())}
                }else if spans<1024 {
                    let (bounds,used)=nurbs_core::normal_alignment::normal_bounds(&surfaces[face],rect,1024-spans).unwrap();spans+=used;bounds
                }else{None};
                components[side]=bounds;if let Some(bounds)=bounds{caches[face].insert(key,bounds);}
            }
            let aligned=match components{[Some(a),Some(b)]=>angle([a,b],threshold).unwrap(),_=>None};
            if aligned==Some(false){continue;}
            if aligned==Some(true){pending.push(pair);reason="aligned-region";break;}
            if spans==1024||factored_queries==1024 {pending.push(pair);reason="query-limit";break;}
            let mut choice=None;let mut largest=0.;
            for side in 0..2{for axis in 0..2{
                let width=(pair[side][axis][1]-pair[side][axis][0])/(original[side][axis][1]-original[side][axis][0]);
                if width>largest{largest=width;choice=Some((side,axis));}
            }}
            let Some((side,axis))=choice else{pending.push(pair);reason="resolution-limit";break;};
            let d=pair[side][axis];let mid=d[0]*0.5+d[1]*0.5;
            if !(d[0]<mid&&mid<d[1]) {pending.push(pair);reason="resolution-limit";break;}
            let mut left=pair;let mut right=pair;left[side][axis][1]=mid;right[side][axis][0]=mid;
            pending.push(right);pending.push(left);
        }
        if pending.is_empty(){reason="factored-product-excluded";}
        rows.push(json!({"faces":faces,"cells":cells,"spans":spans,"factoredQueries":factored_queries,
            "excluded":pending.is_empty(),"reason":reason,"unresolvedPairs":pending}));
    }
    assert_eq!(before,value_codec::to_string(&surfaces).unwrap());
    std::fs::write(&args[3],value_codec::to_string_pretty(&json!({
        "scope":"Exploratory original factored normal products only; no Body certificate",
        "completeTargetedAudit":true,"exactWork":exact_work,"preparations":preparations,
        "originalSurfacesUnchanged":true,"rows":rows})).unwrap()).unwrap();
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn angular_interval_composition_matches_native_original_bounds(){
        let a=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        for slope in [0.,0.1,1.,100.] {let mut b=a.clone();for row in &mut b.control_points{for p in row{p[2]=slope*p[0];}}
            for threshold in [0.,1e-6,0.1,0.9]{let domains=[[[0.,1.];2];2];
                let native=nurbs_core::normal_alignment::inspect_pair([&a,&b],domains,threshold,10).unwrap();
                let x=nurbs_core::normal_alignment::normal_bounds(&a,domains[0],10).unwrap().0.unwrap();
                let y=nurbs_core::normal_alignment::normal_bounds(&b,domains[1],10).unwrap().0.unwrap();
                assert_eq!(angle([x,y],threshold).unwrap(),native.aligned);
            }
        }
        assert_eq!(angle([[[0.,0.];3];2],1e-6).unwrap(),None);
    }
}
