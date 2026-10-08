//! Exploratory closed UV-product normal exclusion on immutable original inputs.
//! This emits diagnostics, never a wall or material-volume certificate.
use nurbs_core::{normal_alignment::OriginalNormalCache, surface::Surface};
fn main() {
    let args: Vec<_> = std::env::args().collect();
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
        while cells<128 && !pending.is_empty() {
            let pair=pending.pop().unwrap(); cells+=1;
            let r=left[a].inspect_other(&mut right[0],pair,threshold,1024-spans).unwrap(); spans+=r.spans;
            if r.aligned==Some(false) {continue;}
            if r.aligned==Some(true) {pending.push(pair);reason="aligned-region";break;}
            if spans==1024 {pending.push(pair);reason="span-limit";break;}
            let mut choice=(0,0);let mut largest=0.;
            for side in 0..2 {for axis in 0..2 {
                let width=(pair[side][axis][1]-pair[side][axis][0])/(original[side][axis][1]-original[side][axis][0]);
                if width>largest {largest=width;choice=(side,axis);}
            }}
            let (side,axis)=choice;let d=pair[side][axis];let mid=d[0]*0.5+d[1]*0.5;
            if mid<=d[0] || mid>=d[1] {pending.push(pair);reason="resolution-limit";break;}
            let mut l=pair;let mut r=pair;l[side][axis][1]=mid;r[side][axis][0]=mid;
            pending.push(r);pending.push(l);
        }
        if pending.is_empty() {reason="normal-product-excluded";}
        rows.push(value_codec::json!({"faces":[a,b],"cells":cells,"spans":spans,"normalExcluded":pending.is_empty(),"reason":reason,"unresolvedPairs":pending}));
        let report=value_codec::json!({"scope":"Exploratory original closed UV-product normal exclusion only; no wall certificate", "completeAudit":rows.len()==pairs.len(),"expectedPairs":pairs.len(),"maxSineSquared":sine,"necessaryPairSineSquared":threshold,"cellsPerPair":128,"spansPerPair":1024,"rows":rows});
        let temporary=format!("{}.tmp",args[3]);std::fs::write(&temporary,value_codec::to_string_pretty(&report).unwrap()).unwrap();std::fs::rename(temporary,&args[3]).unwrap();
    }
    assert_eq!(before,value_codec::to_string(&surfaces).unwrap());
}
