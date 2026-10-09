//! Sampled analytic sketch shapes with bounded output.
use math_core::{Error, Result};
fn fail(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveKind {
    Circle,
    Arc,
}
fn finite(v: impl IntoIterator<Item = f64>) -> Result<()> {
    if v.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(fail("Sketch geometry exceeds finite numeric range."))
    }
}
pub fn arc_direction(degrees: f64) -> [f64; 2] {
    let a = degrees.rem_euclid(360.);
    match a {
        0. => [1., 0.],
        90. => [0., 1.],
        180. => [-1., 0.],
        270. => [0., -1.],
        _ => {
            let (s, c) = a.to_radians().sin_cos();
            [c, s]
        }
    }
}
pub fn sample(
    kind: CurveKind,
    center: [f64; 2],
    radius: f64,
    start: f64,
    sweep: f64,
) -> Result<Vec<[f64; 2]>> {
    if !center
        .into_iter()
        .chain([radius, start, sweep])
        .all(f64::is_finite)
        || !(0.01..=1e6).contains(&radius)
        || !(0.1..=360.).contains(&sweep.abs())
    {
        return Err(fail("Invalid circle or arc parameters."));
    }
    let sweep = if kind == CurveKind::Circle {
        360.
    } else {
        sweep
    };
    let n = (sweep.abs() / 3.).ceil().max(8.) as usize;
    let count = if kind == CurveKind::Circle { n } else { n + 1 };
    let mut points = Vec::with_capacity(count);
    for i in 0..count {
        let [c, s] = arc_direction(if i == n {
            start + sweep
        } else {
            start + sweep * i as f64 / n as f64
        });
        let p = [center[0] + radius * c, center[1] + radius * s];
        finite(p)?;
        points.push(p);
    }
    Ok(points)
}
pub fn slot(a: [f64; 2], b: [f64; 2], width: f64) -> Result<Vec<[f64; 2]>> {
    if !a
        .into_iter()
        .chain(b)
        .chain([width])
        .all(|x| x.is_finite() && x.abs() <= 1e6)
        || width < 0.02
        || (b[0] - a[0]).hypot(b[1] - a[1]) < 1e-6
    {
        return Err(fail(
            "Slot requires distinct centers and width between 0.02 and 1000000 mm.",
        ));
    }
    let angle = (b[1] - a[1]).atan2(b[0] - a[0]);
    let mut points = Vec::with_capacity(66);
    for (center, start) in [
        (b, angle - std::f64::consts::FRAC_PI_2),
        (a, angle + std::f64::consts::FRAC_PI_2),
    ] {
        for i in 0..=32 {
            let t = start + std::f64::consts::PI * i as f64 / 32.;
            let p = [
                center[0] + width * 0.5 * t.cos(),
                center[1] + width * 0.5 * t.sin(),
            ];
            if p.iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
                return Err(fail("Slot exceeds coordinate range."));
            }
            points.push(p);
        }
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarter_endpoints_and_full_circle_counts() {
        assert_eq!(
            sample(CurveKind::Arc, [0., 0.], 2., 0., 90.)
                .unwrap()
                .last(),
            Some(&[0., 2.])
        );
        assert_eq!(
            sample(CurveKind::Circle, [0., 0.], 2., 0., 30.)
                .unwrap()
                .len(),
            120
        );
        assert!(sample(CurveKind::Arc, [0., 0.], 0., 0., 90.).is_err());
    }
}

/// Authored capsule arc definitions. Sampling uses the same native endpoint
/// arithmetic as retained sketch arcs; no near-duplicate connectors are added.
pub fn slot_caps(a:[f64;2],b:[f64;2],width:f64)->Option<([f64;2],[f64;2],f64,f64)> {
    let r=width/2.;
    if !a.into_iter().chain(b).chain([width]).all(|x|x.is_finite() && x.abs()<=1e6) || !(0.02..=1e6).contains(&width) || (b[0]-a[0]).hypot(b[1]-a[1])<1e-6 || a.into_iter().chain(b).any(|x|x.abs()+r>1e6){return None;}
    Some((a,b,r,(b[1]-a[1]).atan2(b[0]-a[0])*180./std::f64::consts::PI))
}
pub fn numeric_rectangle(origin:[f64;2],size:[f64;2])->Option<[[f64;2];4]> {
    if !origin.into_iter().all(|x|x.is_finite() && x.abs()<=1e6) || !size.into_iter().all(|x|x.is_finite() && (0.01..=1e6).contains(&x)){return None;}
    let end=[origin[0]+size[0],origin[1]+size[1]];
    if end.into_iter().any(|x|!x.is_finite() || x.abs()>1e6){return None;}
    Some([origin,[end[0],origin[1]],end,[origin[0],end[1]]])
}
