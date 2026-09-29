//! Native planar sketch placement and analytic circle/arc display sampling.
use super::{Result, Value, encode, field, input};
fn finite(v: impl IntoIterator<Item = f64>) -> Result<()> {
    if v.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(input("Sketch geometry exceeds finite numeric range."))
    }
}
/// One angular convention for display, snapping and retained arc endpoints.
pub(crate) fn arc_direction(degrees:f64)->[f64;2]{
 let a=degrees.rem_euclid(360.);
 match a{0.=>[1.,0.],90.=>[0.,1.],180.=>[-1.,0.],270.=>[0.,-1.],_=>{let(s,c)=a.to_radians().sin_cos();[c,s]}}
}
pub fn sample(curve: Value) -> Result<Value> {
    let kind: String = field(&curve, "kind")?;
    let center: [f64; 2] = field(&curve, "center")?;
    let radius: f64 = field(&curve, "radius")?;
    let start: f64 = field(&curve, "start")?;
    let sweep: f64 = field(&curve, "sweep")?;
    if !["circle", "arc"].contains(&kind.as_str())
        || !center
            .into_iter()
            .chain([radius, start, sweep])
            .all(f64::is_finite)
        || !(0.01..=1e6).contains(&radius)
        || !(0.1..=360.).contains(&sweep.abs())
    {
        return Err(input("Invalid circle or arc parameters."));
    }
    let sweep = if kind == "circle" { 360. } else { sweep };
    let n = (sweep.abs() / 3.).ceil().max(8.) as usize;
    let count = if kind == "circle" { n } else { n + 1 };
    let mut points = Vec::with_capacity(count);
    for i in 0..count {
        let [c,s] = arc_direction(if i==n {start+sweep} else {start + sweep * i as f64 / n as f64});
        let p = [center[0] + radius * c, center[1] + radius * s];
        finite(p)?;
        points.push(p);
    }
    encode(points)
}
pub fn transform(v: Value) -> Result<Value> {
    let mut sketch: Value = field(&v, "sketch")?;
    let delta: [f64; 2] = field(&v, "delta")?;
    let angle: f64 = field(&v, "angle")?;
    let scale: f64 = field(&v, "scale")?;
    if !delta.into_iter().chain([angle, scale]).all(f64::is_finite) || scale <= 0. {
        return Err(input("Invalid sketch transform."));
    }
    let points: Vec<[f64; 2]> = field(&sketch, "points")?;
    if points.is_empty() {
        return Err(input("Sketch requires points."));
    }
    for &p in &points {
        finite(p)?;
    }
    let center: [f64; 2] = if v.get("pivot").is_some_and(|p| !p.is_null()) {
        field(&v, "pivot")?
    } else if let Some(analytic) = sketch.get("analytic") {
        field(analytic, "center")?
    } else {
        std::array::from_fn(|k| points.iter().map(|p| p[k] / points.len() as f64).sum())
    };
    finite(center)?;
    let (s, c) = angle.to_radians().sin_cos();
    let apply = |p: [f64; 2]| -> [f64; 2] {
        let x = p[0] - center[0];
        let y = p[1] - center[1];
        [
            center[0] + delta[0] + scale * (x * c - y * s),
            center[1] + delta[1] + scale * (x * s + y * c),
        ]
    };
    let moved = points.into_iter().map(apply).collect::<Vec<_>>();
    finite(moved.iter().flatten().copied())?;
    sketch["points"] = encode(moved)?;
    if let Some(analytic) = sketch.get_mut("analytic") {
        let center = apply(field(analytic, "center")?);
        let radius = field::<f64>(analytic, "radius")? * scale;
        let start = field::<f64>(analytic, "start")? + angle;
        finite(center.into_iter().chain([radius, start]))?;
        analytic["center"] = encode(center)?;
        analytic["radius"] = encode(radius)?;
        analytic["start"] = encode(start)?;
        let sampled = sample(analytic.clone())?;
        sketch["points"] = sampled;
    }
    Ok(sketch)
}

/// Place local 2D/3D points in an authored basis, retaining basis scale.
pub fn world_points(v: Value) -> Result<Value> {
    let points: Vec<Vec<f64>> = field(&v, "points")?;
    if points.len() > 300000 {
        return Err(input("Sketch placement point budget exceeded."));
    }
    let plane: Value = field(&v, "plane")?;
    let origin: [f64; 3] = field(&plane, "origin")?;
    let u: [f64; 3] = field(&plane, "u")?;
    let w: [f64; 3] = field(&plane, "v")?;
    finite(origin.into_iter().chain(u).chain(w))?;
    let normal = [
        u[1] * w[2] - u[2] * w[1],
        u[2] * w[0] - u[0] * w[2],
        u[0] * w[1] - u[1] * w[0],
    ];
    finite(normal)?;
    let mut result = Vec::with_capacity(points.len());
    for p in points {
        if !(2..=3).contains(&p.len()) {
            return Err(input("Sketch placement requires 2D or 3D points."));
        }
        finite(p.iter().copied())?;
        let z = p.get(2).copied().unwrap_or(0.);
        let point: [f64; 3] =
            std::array::from_fn(|k| origin[k] + p[0] * u[k] + p[1] * w[k] + z * normal[k]);
        finite(point)?;
        result.push(point);
    }
    encode(result)
}

/// Legacy direct point transform around the arithmetic centroid, in native code.
pub fn transform_points(v: Value) -> Result<Value> {
    let points: Vec<Vec<f64>> = field(&v, "points")?;
    let delta: Vec<f64> = field(&v, "delta")?;
    let angle: f64 = field(&v, "angle")?;
    let scale: f64 = field(&v, "scale")?;
    if points.is_empty()
        || points.len() > 300000
        || !(2..=3).contains(&delta.len())
        || !angle.is_finite()
        || !scale.is_finite()
        || scale <= 0.
    {
        return Err(input("Invalid transform."));
    }
    finite(delta.iter().copied())?;
    for p in &points {
        if !(2..=3).contains(&p.len()) {
            return Err(input("Transform requires 2D or 3D points."));
        }
        finite(p.iter().copied())?;
    }
    // Divide before summation so a representable centroid does not overflow merely
    // because the unscaled coordinate sum exceeds binary64 range.
    let center: [f64; 3] = std::array::from_fn(|k| {
        points
            .iter()
            .map(|p| p.get(k).copied().unwrap_or(0.) / points.len() as f64)
            .sum()
    });
    finite(center)?;
    let (s, c) = angle.to_radians().sin_cos();
    let mut result = Vec::with_capacity(points.len());
    for p in points {
        let x = (p[0] - center[0]) * scale;
        let y = (p[1] - center[1]) * scale;
        let mut q = vec![
            center[0] + c * x - s * y + delta[0],
            center[1] + s * x + c * y + delta[1],
        ];
        if p.len() == 3 {
            q.push(center[2] + (p[2] - center[2]) * scale + delta.get(2).copied().unwrap_or(0.));
        }
        finite(q.iter().copied())?;
        result.push(q);
    }
    encode(result)
}

/// A closed tessellated capsule, specified by two end-cap centers and full width.
pub fn slot(v: Value) -> Result<Value> {
    let a: [f64; 2] = field(&v, "a")?;
    let b: [f64; 2] = field(&v, "b")?;
    let width: f64 = field(&v, "width")?;
    if !a.into_iter().chain(b).chain([width]).all(|x| x.is_finite() && x.abs() <= 1e6)
        || width < 0.02 || (b[0]-a[0]).hypot(b[1]-a[1]) < 1e-6 {
        return Err(input("Slot requires distinct centers and width between 0.02 and 1000000 mm."));
    }
    let angle = (b[1]-a[1]).atan2(b[0]-a[0]);
    let mut points = Vec::with_capacity(66);
    for (center, start) in [(b, angle-std::f64::consts::FRAC_PI_2), (a, angle+std::f64::consts::FRAC_PI_2)] {
        for i in 0..=32 {
            let t = start + std::f64::consts::PI * i as f64 / 32.;
            let p = [center[0]+width*0.5*t.cos(),center[1]+width*0.5*t.sin()];
            if p.iter().any(|x| !x.is_finite() || x.abs()>1e6) { return Err(input("Slot exceeds coordinate range.")); }
            points.push(p);
        }
    }
    encode(points)
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    #[test]
    fn capsule_bounds_and_invalid_centers() {
        let result=slot(super::super::json!({"a":[0.,0.],"b":[10.,0.],"width":4.})).unwrap();
        let points: Vec<[f64;2]>=result.as_array().unwrap().iter().map(|p|[p[0].as_f64().unwrap(),p[1].as_f64().unwrap()]).collect();
        assert_eq!(points.len(),66);
        assert!((points.iter().map(|p|p[0]).fold(f64::INFINITY,f64::min)+2.).abs()<1e-9);
        assert!((points.iter().map(|p|p[0]).fold(f64::NEG_INFINITY,f64::max)-12.).abs()<1e-9);
        assert!(slot(super::super::json!({"a":[0.,0.],"b":[0.,0.],"width":4.})).is_err());
        assert!(slot(super::super::json!({"a":[0.,0.],"b":[10.,0.],"width":0.})).is_err());
    }
}
