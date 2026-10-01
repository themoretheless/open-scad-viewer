//! Native workbench section selection, placement, matching and mesh construction.
use super::{Result, Value, cad_path, cad_sketch, encode, field, input};
fn placed(sketch: &Value, close: bool) -> Result<Vec<[f64; 3]>> {
    let mut points: Vec<[f64; 2]> = field(sketch, "points")?;
    if !(2..=512).contains(&points.len()) {
        return Err(input("Section requires 2–512 points."));
    }
    if close {
        points.push(points[0]);
    }
    let plane = sketch
        .get("plane")
        .cloned()
        .unwrap_or_else(|| value_codec::json!({"origin":[0.,0.,0.],"u":[1.,0.,0.],"v":[0.,1.,0.]}));
    let value = cad_sketch::world_points(value_codec::json!({"points":points,"plane":plane}))?;
    value_codec::from_value(value).map_err(|e| input(e.to_string()))
}
pub fn build(v: Value) -> Result<Value> {
    let sketches: Vec<Value> = field(&v, "sketches")?;
    let ids: Vec<String> = field(&v, "profileIds")?;
    let action: String = field(&v, "action")?;
    let mut profiles = Vec::new();
    for id in ids {
        if let Some(s) = sketches
            .iter()
            .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            profiles.push(s);
        }
    }
    let built = match action.as_str() {
        "loft" => {
            if !(2..=64).contains(&profiles.len()) {
                return Err(input("Choose 2–64 closed sections in loft order."));
            }
            let mut count = 0;
            for s in &profiles {
                if !field::<bool>(s, "closed")? {
                    return Err(input("Choose closed sections in loft order."));
                }
                let points: Vec<[f64; 2]> = field(s, "points")?;
                count = count.max(points.len());
            }
            if !(3..=512).contains(&count) {
                return Err(input("Loft requires 3–512 points per section."));
            }
            let fractions: Vec<f64> = (0..count).map(|i| i as f64 / count as f64).collect();
            let mut sections = Vec::with_capacity(profiles.len());
            for s in profiles {
                let path = placed(s, true)?;
                let sampled =
                    cad_path::sample(value_codec::json!({"path":path,"fractions":fractions}))?;
                let section: Vec<[f64; 3]> =
                    value_codec::from_value(sampled).map_err(|e| input(e.to_string()))?;
                sections.push(section);
            }
            polygon_core::solid::modeling::loft(&sections, true)?
        }
        "sweep" => {
            let id: String = field(&v, "pathId")?;
            let profile = profiles
                .first()
                .ok_or_else(|| input("Choose a closed profile and an open path."))?;
            let path = sketches
                .iter()
                .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
                .ok_or_else(|| input("Choose a closed profile and an open path."))?;
            if !field::<bool>(profile, "closed")? || field::<bool>(path, "closed")? {
                return Err(input("Choose a closed profile and an open path."));
            }
            let points: Vec<[f64; 2]> = field(profile, "points")?;
            let up: [f64; 3] = match profile.get("plane") {
                Some(p) => field(p, "u")?,
                None => [1., 0., 0.],
            };
            polygon_core::solid::modeling::sweep(&points, &placed(path, false)?, up, true)?
        }
        _ => return Err(input("Invalid section operation.")),
    };
    let report = built.mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. {
        return Err(input("Operation did not produce a closed solid."));
    }
    encode(built.mesh)
}

fn retained_loft(profiles: &[&Value]) -> Result<Value> {
    use nurbs_core::curve::Curve;
    if profiles.len()!=2 { return Err(input("Retained loft currently requires two corresponding sections; use two sections or split the loft.")); }
    let loops=|sketch: &Value| -> Result<Vec<Vec<Curve>>> {
        if !field::<bool>(sketch,"closed")? { return Err(input("Loft sections must be closed.")); }
        if let Some(profile)=sketch.get("retainedProfile") { field(profile,"loops") }
        else if sketch.get("analytic").is_none() { Ok(vec![brep_core::sketch::polygon_wire(field(sketch,"points")?)?]) }
        else { Err(input("Prepare analytic curves as retained profiles before loft.")) }
    };
    let a=loops(profiles[0])?;let b=loops(profiles[1])?;
    for profile in [&a,&b] {
        if profile.len()>brep_core::prism::MAX_PROFILE_LOOPS
            || profile.iter().map(Vec::len).sum::<usize>()>brep_core::prism::MAX_PROFILE_CURVES
            || profile.iter().flatten().map(|c|c.control_points.len()).sum::<usize>()>brep_core::prism::MAX_PROFILE_CONTROLS {
            return Err(input("Retained loft exceeds profile loop, span or control budget."));
        }
    }
    if a.is_empty() || a.len()!=b.len() || a.iter().zip(&b).any(|(a,b)|a.len()!=b.len()) {
        return Err(input("Loft sections require matching contour and segment counts."));
    }
    for (a,b) in a.iter().flatten().zip(b.iter().flatten()) {
        a.validate()?;b.validate()?;
        if a.degree!=b.degree || a.knots!=b.knots || a.weights!=b.weights || a.periodic!=b.periodic || a.control_points.len()!=b.control_points.len()
            || a.control_points.iter().chain(&b.control_points).any(|p|p.len()!=2) {
            return Err(input("Retained loft requires matching curve degrees, knots, weights and control counts; align the source curves."));
        }
    }
    let points: Vec<_>=a.iter().flatten().flat_map(|c|&c.control_points).zip(b.iter().flatten().flat_map(|c|&c.control_points)).collect();
    let (first,top)=points[0];
    let scale=points.iter().find_map(|(p,q)|(0..2).find(|&i|(p[i]-first[i]).abs()>1e-10).map(|i|(q[i]-top[i])/(p[i]-first[i])))
        .ok_or_else(||input("Loft profile has degenerate control bounds."))?;
    let offset=[top[0]-scale*first[0],top[1]-scale*first[1]];
    if !scale.is_finite() || scale<=0. || points.iter().any(|(p,q)|(0..2).any(|i|scale*p[i]+offset[i]!=q[i])) {
        return Err(input("Retained loft requires an exactly corresponding positive uniform scale and translation; edit the section copies."));
    }
    let plane=|s: &Value|s.get("plane").cloned().unwrap_or_else(||value_codec::json!({"origin":[0.,0.,0.],"u":[1.,0.,0.],"v":[0.,1.,0.]}));
    let pa=plane(profiles[0]);let pb=plane(profiles[1]);
    let origin:[f64;3]=field(&pa,"origin")?;let end:[f64;3]=field(&pb,"origin")?;
    let u:[f64;3]=field(&pa,"u")?;let v:[f64;3]=field(&pa,"v")?;
    let dot=|a:[f64;3],b:[f64;3]|(0..3).map(|i|a[i]*b[i]).sum::<f64>();
    if u!=field::<[f64;3]>(&pb,"u")? || v!=field::<[f64;3]>(&pb,"v")? || (dot(u,u)-1.).abs()>1e-12 || (dot(v,v)-1.).abs()>1e-12 || dot(u,v).abs()>1e-12 {
        return Err(input("Retained loft requires parallel sections with the same orthonormal sketch basis."));
    }
    let n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
    let delta=std::array::from_fn(|i|end[i]-origin[i]);
    let height=dot(delta,n);
    if !height.is_finite() || height<=0. { return Err(input("Order loft sections along the positive sketch normal with distinct planes.")); }
    let local=brep_core::prism::loft_scaled(&a,0.,height,scale,[offset[0]+dot(delta,u),offset[1]+dot(delta,v)])?;
    let matrix=std::array::from_fn(|i|if i==3 {[0.,0.,0.,1.]}else{[u[i],v[i],n[i],origin[i]]});
    let model=brep_core::transform::affine(&local,matrix)?;
    let mesh=super::brep::nurbs(&model,4)?.built.mesh;
    Ok(value_codec::json!({"brep":model,"mesh":mesh}))
}

/// Preserve authored polygon vertices for ruled B-rep correspondence.
pub fn ruled(v: Value) -> Result<Value> {
    let sketches: Vec<Value> = field(&v, "sketches")?;
    let ids: Vec<String> = field(&v, "ids")?;
    if !(2..=64).contains(&ids.len()) {
        return Err(input("Select 2–64 closed polygon sketches in loft order."));
    }
    let profiles: Vec<_>=ids.iter().map(|id|sketches.iter().find(|s|s.get("id").and_then(Value::as_str)==Some(id.as_str())).ok_or_else(||input("Loft section is missing."))).collect::<Result<_>>()?;
    if profiles.iter().any(|s|s.get("retainedProfile").is_some()) { return retained_loft(&profiles); }
    let mut sections = Vec::with_capacity(ids.len());
    for id in ids {
        let sketch = sketches
            .iter()
            .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| input("Select only closed polygon sketches for ruled loft."))?;
        if !field::<bool>(sketch, "closed")? || sketch.get("analytic").is_some() {
            return Err(input(
                "Ruled loft requires closed polygon sketches; analytic curves are not supported.",
            ));
        }
        sections.push(placed(sketch, false)?);
    }
    let model = brep_core::ruled_loft(&sections)?;
    let mesh = super::brep::nurbs(&model, 4)?.built.mesh;
    Ok(value_codec::json!({"brep":model,"mesh":mesh}))
}

#[cfg(test)]
mod retained_tests {
    use super::*;
    #[test]
    fn retained_scaled_sections_preserve_holes_and_refuse_noncorresponding_controls() {
        use nurbs_core::curve::Curve;
        let mut hole=brep_core::sketch::circle_wire(1.).unwrap();
        hole=hole.iter().rev().map(Curve::reverse).collect::<nurbs_core::Result<Vec<_>>>().unwrap();
        let loops=vec![brep_core::sketch::circle_wire(3.).unwrap(),hole];
        let mut top=loops.clone();
        for p in top.iter_mut().flatten().flat_map(|c|&mut c.control_points) { p[0]=2.*p[0]+5.;p[1]=2.*p[1]+7.; }
        let request=value_codec::json!({"ids":["base","top"],"sketches":[
            {"id":"base","closed":true,"points":[],"retainedProfile":{"loops":loops}},
            {"id":"top","closed":true,"points":[],"retainedProfile":{"loops":top},"plane":{"origin":[0.,0.,10.],"u":[1.,0.,0.],"v":[0.,1.,0.]}}
        ]});
        let before=request.clone();
        let result=ruled(request.clone()).unwrap();
        let model:brep_core::Model=field(&result,"brep").unwrap();
        model.validate().unwrap();
        assert_eq!(model.bodies.len(),1);
        assert_eq!(model.faces.iter().filter(|f|f.holes.len()==1).count(),2);
        assert_eq!(request,before);
        let mut invalid=request.clone();
        invalid["sketches"][1]["retainedProfile"]["loops"][0][0]["controlPoints"][1][0]=Value::from(123.);
        assert!(ruled(invalid).is_err());
    }
}
