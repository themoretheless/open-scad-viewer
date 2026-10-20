//! Bounded replay of an owned proposed Hermite expression, never solid admission.
use super::{field,check,Value,Result,json};
use crate::{surface::Surface,normal_alignment::qualify_weight_profile,profile_section_bridge::ProfileSectionBridge};
pub(super) fn inspect(v:&Value)->Result<Value> {
    let sources:[Surface;2]=field(v,"surfaces")?;
    let pivots:[[usize;2];2]=field(v,"pivots")?;
    let axes:[usize;2]=field(v,"axes")?;
    let parameters:[f64;2]=field(v,"parameters")?;
    let scales:[f64;2]=field(v,"scales")?;
    let sine:f64=field(v,"maxSineSquared")?;
    let max_spans:usize=field(v,"maxSpans")?;
    let max_controls:usize=field(v,"maxControls")?;
    let pair:[[[f64;2];2];2]=field(v,"pair")?;
    let coverage=if let Some(c)=v.get("coverage") {
        let domain:[[f64;2];2]=field(c,"domain")?;
        let cells:usize=field(c,"cells")?;let spans:usize=field(c,"spans")?;
        check(cells<=100000 && spans<=100000 && domain.iter().all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<d[1]),"Bound whole bridge coverage work and domain")?;
        Some((domain,cells,spans))
    }else{None};
    for s in &sources {s.validate()?;}
    check(axes.iter().all(|&a|a<2) && parameters.iter().all(|x|x.is_finite())
        && scales.iter().all(|x|x.is_finite() && *x!=0.) && sine.is_finite() && (0. ..1.).contains(&sine)
        && max_spans<=100000 && max_controls<=1000000
        && pair.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<d[1]),
        "Bound profile bridge parameters, angular policy and work")?;
    let natural=|s:&Surface|[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]];
    let ranges=[natural(&sources[0]),natural(&sources[1])];
    for i in 0..2 {
        check(parameters[i]>=ranges[i][axes[i]][0] && parameters[i]<=ranges[i][axes[i]][1]
            && pivots[i][0]<sources[i].control_points.len() && pivots[i][1]<sources[i].control_points[0].len(),
            "Keep section parameters and profile pivots in the source domain")?;
    }
    let free=ranges[0][1-axes[0]];
    check(free==ranges[1][1-axes[1]] && pair.iter().all(|p|p[0][0]>=free[0] && p[0][1]<=free[1]
        && p[1][0]>=0. && p[1][1]<=1.),"Keep chord rectangles in a common bridge chart")?;
    if let Some((domain,_,_))=coverage {
        check(domain[0][0]>=free[0] && domain[0][1]<=free[1] && domain[1][0]>=0. && domain[1][1]<=1.,
            "Keep whole bridge coverage within the owned chart")?;
    }
    let (a,used_a)=qualify_weight_profile(&sources[0],pivots[0],max_controls)?;
    let (b,used_b)=qualify_weight_profile(&sources[1],pivots[1],max_controls-used_a)?;
    let mut base=json!({"version":1,"method":"owned-profile-bridge-endpoint-chords",
        "proposedExpression":true,"wholeThicknessQualified":false,"solidQualified":false,
        "request":v,"profileControls":used_a+used_b,"spans":0,"excluded":Value::Null,
        "reason":"profile-bridge-profile-unproven"});
    if let Some((domain,_,_))=coverage {
        base["coverage"]=json!({"cells":0,"spans":0,"exclusions":0,
            "unresolvedPairs":[[domain,domain]],"completeExclusion":false});
    }
    let (Some(a),Some(b))=(a,b) else {return Ok(base);};
    let bridge=ProfileSectionBridge::new(a.section(axes[0],parameters[0])?,b.section(axes[1],parameters[1])?,scales)?;
    let (excluded,spent)=bridge.inspect_self_chord_pair(pair,sine,max_spans)?;
    let mut out=base;
    out["excluded"]=json!(excluded);out["spans"]=json!(spent);
    out["profilePositionDeviationUpperMm"]=json!([a.axis_deviation_upper(),b.axis_deviation_upper()]);
    out["profileWeightDeviationUpper"]=json!([a.weight_deviation_upper(),b.weight_deviation_upper()]);
    out["reason"]=json!(match excluded {Some(true)=>"profile-bridge-chord-excluded",Some(false)=>"profile-bridge-chord-unexcluded",None=>"profile-bridge-chord-unproven"});
    if let Some((domain,cells,spans))=coverage {
        let report=bridge.inspect_self_chord_coverage(domain,sine,cells,spans)?;
        out["coverage"]=json!({"cells":report.cells,"spans":report.spans,"exclusions":report.exclusions,
            "unresolvedPairs":report.unresolved_pairs,"completeExclusion":report.complete_exclusion});
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane()->Surface {Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}}
    fn request()->Value {json!({"op":"profile_section_bridge_chords","surfaces":[plane(),plane()],"pivots":[[0,0],[0,0]],
        "axes":[0,0],"parameters":[0.,1.],"scales":[1.,1.],"maxSineSquared":1e-4,"maxSpans":1000,"maxControls":100,
        "pair":[[[0.1,0.11],[0.1,0.11]],[[0.2,0.21],[0.2,0.21]]]})}
    #[test] fn replays_independent_owners_and_preserves_refusal(){
        let v=request();let direct=inspect(&v).unwrap();
        assert_eq!(super::super::dispatch(v.clone()).unwrap(),direct);
        let mut rational=v.clone();
        rational["surfaces"][1]["weights"]=json!([[1.,1.],[1.,2.]]);
        rational["surfaces"][1]["controlPoints"][1][1][2]=json!(0.25);
        let proposed=inspect(&rational).unwrap();
        assert!(proposed["profileWeightDeviationUpper"][1].as_f64().unwrap()>=1.);
        assert!(proposed["profilePositionDeviationUpperMm"][1][2].as_f64().unwrap()>0.);
        assert_eq!(proposed["proposedExpression"],json!(true));
        assert_eq!(proposed["solidQualified"],json!(false));
        assert_eq!(proposed["wholeThicknessQualified"],json!(false));
        let arch=Surface {degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|
                [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
            weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false};
        let mut diameter=v.clone();diameter["surfaces"]=json!([arch.clone(),arch]);
        diameter["pair"]=json!([[[0.,0.001],[0.4,0.41]],[[0.999,1.],[0.4,0.41]]]);
        let preserved=inspect(&diameter).unwrap();
        assert_ne!(preserved["excluded"],json!(true),"Preserve a genuine normal diameter");
        let mut whole=v.clone();whole["coverage"]=json!({"domain":[[0.,1.],[0.,1.]],"cells":10000,"spans":100000});
        let covered=inspect(&whole).unwrap();assert_eq!(covered["coverage"]["completeExclusion"],json!(true));
        assert_eq!(covered["solidQualified"],json!(false));
        let mut stopped=whole.clone();stopped["coverage"]["cells"]=json!(0);
        let stopped_report=inspect(&stopped).unwrap();assert_eq!(stopped_report["coverage"]["completeExclusion"],json!(false));
        assert_eq!(stopped_report["coverage"]["unresolvedPairs"],json!([[[[0.,1.],[0.,1.]],[[0.,1.],[0.,1.]]]]));
        let mut curved_whole=diameter.clone();
        curved_whole["coverage"]=json!({"domain":[[0.,1.],[0.,1.]],"cells":31,"spans":10000});
        let curved_report=inspect(&curved_whole).unwrap();
        assert_eq!(curved_report["coverage"]["completeExclusion"],json!(false));
        let pending:Vec<[[[f64;2];2];2]>=field(&curved_report["coverage"],"unresolvedPairs").unwrap();
        let contains=|r:[[f64;2];2],p:[f64;2]|(0..2).all(|i|r[i][0]<=p[i] && p[i]<=r[i][1]);
        assert!(pending.iter().any(|p|
            (contains(p[0],[0.,0.4]) && contains(p[1],[1.,0.4])) ||
            (contains(p[1],[0.,0.4]) && contains(p[0],[1.,0.4]))),
            "Whole coverage must retain the genuine normal diameter, including chart endpoints");
        let mut bad=stopped.clone();bad["maxControls"]=json!(0);bad["coverage"]["domain"]=json!([[0.,2.],[0.,1.]]);
        assert!(inspect(&bad).is_err());
        if let Ok(path)=std::env::var("NURBS_PROFILE_BRIDGE_REPORT_PATH") {
            std::fs::write(path,json!({"request":v,"report":direct,
                "rationalProposed":{"request":rational,"report":proposed},
                "normalDiameter":{"request":diameter,"report":preserved},
                "curvedWholeCoverage":{"request":curved_whole,"report":curved_report},
                "wholeCoverage":{"request":whole,"report":covered},"stoppedCoverage":{"request":stopped,"report":stopped_report}}).to_string()).unwrap();
        }

        assert_eq!(direct["excluded"],json!(true));assert_eq!(direct["solidQualified"],json!(false));
        assert_eq!(direct["wholeThicknessQualified"],json!(false));assert_eq!(direct["proposedExpression"],json!(true));
        assert!(direct["profilePositionDeviationUpperMm"].is_array());
        let mut zero=v.clone();zero["maxSpans"]=json!(0);
        let refused=inspect(&zero).unwrap();assert!(refused["excluded"].is_null());assert_eq!(refused["spans"],json!(0));
        zero["maxControls"]=json!(0);assert!(inspect(&zero).unwrap()["excluded"].is_null());
        for (field,value) in [("maxSineSquared",json!(1.)),("maxSpans",json!(100001)),("maxControls",json!(1000001)),
            ("axes",json!([2,0])),("scales",json!([0.,1.]))] {
            let mut bad=v.clone();bad[field]=value;assert!(inspect(&bad).is_err(),"{field}");
        }
        // A zero work budget must never bypass source/chart validation.
        for controls in [0,100] {
            for (field,value) in [("pair",json!([[[0.1,1.1],[0.1,0.2]],[[0.2,0.3],[0.2,0.3]]])),
                ("parameters",json!([-0.1,1.])),("pivots",json!([[2,0],[0,0]]))] {
                let mut bad=v.clone();bad["maxControls"]=json!(controls);bad["maxSpans"]=json!(0);
                bad[field]=value;assert!(inspect(&bad).is_err(),"{field}, controls={controls}");
            }
        }
        let mut bad=v;bad["surfaces"][0]["weights"]=json!([]);assert!(inspect(&bad).is_err());
    }
}
