//! Direct sketch dimensions. Values are measured from geometry; editing moves b
//! around a (length) or c around b (signed angle a-b-c).
use super::{Result, Value, field, input, json};
pub fn dimensions(v: Value) -> Result<Value> {
    let mut points: Vec<[f64; 2]> = field(&v, "points")?;
    if points.len() < 2 || points.len() > 512 || points.iter().flatten().any(|x| !x.is_finite() || x.abs() > 1e6) {
        return Err(input("Invalid dimension points."));
    }
    let specs: Vec<Value> = field(&v, "dimensions")?;
    if specs.len() > 96 { return Err(input("Too many dimensions.")); }
    let mut refs = Vec::new();
    for spec in &specs {
        let kind: String = field(spec, "kind")?;
        let a: usize = field(spec, "a")?;
        let b: usize = field(spec, "b")?;
        let c: usize = if kind == "angle" { field(spec, "c")? } else { a };
        if !["length", "angle", "horizontal", "vertical"].contains(&kind.as_str()) || [a,b,c].iter().any(|&i| i >= points.len()) || a == b || (kind == "angle" && (c == a || c == b)) {
            return Err(input("Invalid dimension references."));
        }
        refs.push((kind,a,b,c));
    }
    if let Some(edit) = v.get("edit") {
        let index: usize = field(edit, "index")?;
        let value: f64 = field(edit, "value")?;
        let Some((kind,a,b,c)) = refs.get(index) else { return Err(input("Unknown dimension.")); };
        if !value.is_finite() { return Err(input("Invalid dimension value.")); }
        if kind == "length" {
            if !(1e-6..=1e6).contains(&value) { return Err(input("Length must be positive (mm).")); }
            let u = [points[*b][0]-points[*a][0], points[*b][1]-points[*a][1]];
            let len = u[0].hypot(u[1]);
            if len < 1e-9 { return Err(input("Cannot resize a zero-length segment.")); }
            points[*b] = std::array::from_fn(|k| points[*a][k]+u[k]*value/len);
        } else if kind == "angle" {
            if !(-180.0..=180.0).contains(&value) { return Err(input("Signed angle must be between -180 and 180 degrees.")); }
            let u = [points[*a][0]-points[*b][0],points[*a][1]-points[*b][1]];
            let w = [points[*c][0]-points[*b][0],points[*c][1]-points[*b][1]];
            let len = w[0].hypot(w[1]);
            if len < 1e-9 || u[0].hypot(u[1]) < 1e-9 { return Err(input("Angle requires two nonzero segments.")); }
            let angle = u[1].atan2(u[0]) + value.to_radians();
            points[*c] = [points[*b][0]+len*angle.cos(),points[*b][1]+len*angle.sin()];
        } else {
            if value.abs() > 1e6 { return Err(input("Dimension value exceeds coordinate range.")); }
            if kind == "horizontal" { points[*b][0] = points[*a][0] + value; }
            else { points[*b][1] = points[*a][1] + value; }
        }
    }
    if points.iter().flatten().any(|x| !x.is_finite() || x.abs() > 1e6) { return Err(input("Dimension edit exceeds coordinate range.")); }
    let measurements: Vec<Value> = refs.iter().map(|(kind,a,b,c)| {
        let u = [points[*a][0]-points[*b][0],points[*a][1]-points[*b][1]];
        if kind == "length" {
            let length=u[0].hypot(u[1]);
            let offset=[-u[1]*0.15,u[0]*0.15];
            let start: [f64;2]=std::array::from_fn(|k|points[*a][k]+offset[k]);
            let end: [f64;2]=std::array::from_fn(|k|points[*b][k]+offset[k]);
            json!({"value":length,"label":[(start[0]+end[0])/2.,(start[1]+end[1])/2.],"lines":[vec![points[*a],start,end,points[*b]]]})
        } else if kind == "angle" {
            let w = [points[*c][0]-points[*b][0],points[*c][1]-points[*b][1]];
            let valid = u[0].hypot(u[1]) > 1e-9 && w[0].hypot(w[1]) > 1e-9;
            let angle = (u[0]*w[1]-u[1]*w[0]).atan2(u[0]*w[0]+u[1]*w[1]).to_degrees();
            let radius=u[0].hypot(u[1]).min(w[0].hypot(w[1]))*0.3;
            let start=u[1].atan2(u[0]);
            let arc: Vec<[f64;2]>=(0..=32).map(|i|{let t=start+angle.to_radians()*i as f64/32.;[points[*b][0]+radius*t.cos(),points[*b][1]+radius*t.sin()]}).collect();
            let middle=start+angle.to_radians()/2.;
            json!({"value":if valid {json!(angle)} else {Value::Null},"label":[points[*b][0]+radius*middle.cos(),points[*b][1]+radius*middle.sin()],"lines":[arc]})
        } else {
            let value = if kind == "horizontal" { u[0] } else { u[1] };
            let offset = if kind == "horizontal" { [0., 0.15 * u[0].signum()] } else { [0.15 * u[1].signum(), 0.] };
            json!({"value":value,"label":[(points[*a][0]+points[*b][0])/2.+offset[0],(points[*a][1]+points[*b][1])/2.+offset[1]],"lines":[vec![points[*a],points[*b]]]})
        }
    }).collect();
    Ok(json!({"points":points,"measurements":measurements}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn length_and_signed_angle() {
        let v=json!({"points":[[0,0],[10,0],[10,10]],"dimensions":[{"kind":"length","a":0,"b":1},{"kind":"angle","a":0,"b":1,"c":2}]});
        let measured=dimensions(v.clone()).unwrap();
        assert_eq!(measured["measurements"][1]["value"].as_f64(),Some(-90.));
        let mut edit=v.clone();edit["edit"]=json!({"index":1,"value":-60});
        let result=dimensions(edit).unwrap();
        assert!((result["measurements"][1]["value"].as_f64().unwrap()+60.).abs()<1e-9);
        let mut bad=v;bad["edit"]=json!({"index":0,"value":-1});assert!(dimensions(bad).is_err());
    }
    #[test]
    fn horizontal_and_vertical_dimensions_edit_one_axis() {
        let mut v=json!({"points":[[0,0],[3,4]],"dimensions":[{"kind":"horizontal","a":0,"b":1},{"kind":"vertical","a":0,"b":1}]});
        assert_eq!(dimensions(v.clone()).unwrap()["measurements"][0]["value"].as_f64(),Some(-3.));
        v["edit"]=json!({"index":0,"value":8});
        let result=dimensions(v).unwrap();
        assert_eq!(result["points"][1][0].as_f64(),Some(8.));
        assert_eq!(result["points"][1][1].as_f64(),Some(4.));
    }
}
