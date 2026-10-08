//! Continuous authored-frame jets from original rational vector laws.
//! Derivatives use normalized traversal; this alone is not a sweep certificate.
use super::{scalar_certificate::Status, vector_certificate};
use crate::numerics::interval_vec3::{add, cross, div, dot_tight as dot, scale as mul, sub};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
type V = [I; 3];
mod fixed_path_values;
mod fixed_normal_path;
mod planar_principal_phase;
pub(crate) use planar_principal_phase::{certify_initial_planar_principal_phase,certify_planar_principal_values,certify_planar_principal_control_values,certify_planar_principal_control_trajectories,certify_planar_principal_relative_values};
mod frenet_path_values;
pub use frenet_path_values::{certify_frenet_cover,certify_frenet_path_values,certify_frenet_control_values,certify_frenet_planar_control_trajectories,certify_frenet_path,certify_frenet_control_trajectories};
pub use fixed_normal_path::{certify_fixed_normal_path,certify_fixed_normal_path_values,certify_fixed_normal_control_values,certify_fixed_normal_control_trajectories,certify_fixed_normal_cover};
pub use fixed_path_values::{certify_fixed_path_values,certify_fixed_path_control_values,certify_fixed_path,certify_fixed_path_control_trajectories};
pub(crate) use fixed_path_values::{certify_fixed_relative_value,certify_fixed_relative_trajectory};
pub(crate) use fixed_path_values::{certify_fixed_path_control_values_with_frame_path,certify_fixed_path_control_trajectories_with_frame_path};
pub(crate) use trajectory::{translated_constant_values,translated_constant_jets};
pub(crate) use fixed_normal_path::{certify_fixed_normal_relative_values,certify_proved_planar_relative_values,certify_proved_planar_control_values};
mod guided_axis;
mod guided_path_values;
mod paired_path_values;
mod guide_width;
pub use guide_width::{GuideWidthReport, certify_guide_width};
mod contact_fit;
pub use contact_fit::{certify_contact_fit, certify_contact_control_trajectories, certify_contact_control_trajectory};
mod contact_fit_value;
pub use contact_fit_value::{ContactFitValueReport, certify_contact_fit_value, certify_contact_control_values, certify_contact_control_value};
pub use guided_path_values::{certify_path_guide_values,certify_path_guide_control_values,certify_path_guide_control_value,certify_path_guide,certify_path_guide_control_trajectories,certify_path_guide_control_trajectory};
mod trajectory;
pub use trajectory::{TrajectoriesReport,ControlValuesReport,ControlValueReport, TrajectoryReport, certify_control_trajectory, certify_control_value, certify_control_trajectories, certify_control_values};
pub use guided_axis::{certify_authored_guide, certify_authored_guide_values};
#[derive(Clone, Debug)]
pub struct ValuesReport {
    pub longitudinal: Option<[[f64; 2]; 3]>,
    pub status: Status,
    pub cells: usize,
    pub transverse: Option<[[f64; 2]; 3]>,
    pub binormal: Option<[[f64; 2]; 3]>,
}
/// Value-only endpoint path avoids derivative division on subnormal-width
/// point restrictions. Work is conservatively charged for every active span
/// of each scalar component, including spans outside the requested interval.
pub fn certify_twisted_values(
    longitudinal: &Curve,
    transverse: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ValuesReport> {
    check(
        max_cells <= 100000,
        "Authored frame budget exceeds100000 cells",
    )?;
    super::validate_law(twist, false)?;
    let mut out = ValuesReport {
        longitudinal: None,
        status: Status::Unresolved,
        cells: 0,
        transverse: None,
        binormal: None,
    };
    let mut value = |curve: &Curve| -> Result<Option<I>> {
        let charge = (curve.degree..curve.control_points.len())
            .filter(|&i| curve.knots[i] < curve.knots[i + 1])
            .count();
        if charge > max_cells - out.cells {
            return Ok(None);
        }
        let result = super::scalar_certificate::value_traversal(curve, traversal, charge)?;
        out.cells += charge;
        result.map(|v| I::new(v[0], v[1])).transpose()
    };
    let mut vectors = [[I::point(0.); 3]; 2];
    for (index, curve) in [longitudinal, transverse].into_iter().enumerate() {
        curve.validate()?;
        check(
            curve.control_points.iter().all(|p| p.len() == 3),
            "Authored frame requires XYZ controls",
        )?;
        for k in 0..3 {
            let mut scalar = curve.clone();
            scalar.control_points = curve
                .control_points
                .iter()
                .map(|p| vec![p[k], 0., 0.])
                .collect();
            let Some(v) = value(&scalar)? else {
                return Ok(out);
            };
            vectors[index][k] = v;
        }
    }
    let Some(theta) = value(twist)? else {
        return Ok(out);
    };
    let constant = |v| Jet {
        v,
        d: [I::point(0.); 3],
        dd: [I::point(0.); 3],
    };
    let Some(t) = normalize(constant(vectors[0]))? else {
        return Ok(out);
    };
    let Some(b) = normalize(constant(cross(t.v, vectors[1])?))? else {
        return Ok(out);
    };
    let n = cross(b.v, t.v)?;
    let tr = super::trigonometric_certificate::certify([theta.lo, theta.hi])?;
    let rotated = add(
        mul(n, I::new(tr.cos[0], tr.cos[1])?)?,
        mul(b.v, I::new(tr.sin[0], tr.sin[1])?)?,
    )?;
    out.longitudinal = Some(t.v.map(|x| [x.lo, x.hi]));
    out.transverse = Some(rotated.map(|x| [x.lo, x.hi]));
    out.binormal = Some(cross(t.v, rotated)?.map(|x| [x.lo, x.hi]));
    out.status = Status::Certified;
    Ok(out)
}
#[derive(Clone, Copy)]
struct Jet {
    v: V,
    d: V,
    dd: V,
}
#[derive(Clone, Debug)]
pub struct FrameJet {
    pub value: [[f64; 2]; 3],
    pub first: [[f64; 2]; 3],
    pub second: [[f64; 2]; 3],
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    pub longitudinal: Option<FrameJet>,
    pub transverse: Option<FrameJet>,
    pub binormal: Option<FrameJet>,
    pub single_span: bool,
    pub reason: Option<&'static str>,
}
fn cross_jet(a: Jet, b: Jet) -> Result<Jet> {
    Ok(Jet {
        v: cross(a.v, b.v)?,
        d: add(cross(a.d, b.v)?, cross(a.v, b.d)?)?,
        dd: add(
            add(cross(a.dd, b.v)?, mul(cross(a.d, b.d)?, I::point(2.))?)?,
            cross(a.v, b.dd)?,
        )?,
    })
}
// Component squares use their actual interval range rather than x*x's
// dependency-lost lower bound, preserving nonzero component witnesses.
fn square(a: I) -> Result<I> {
    let lo = if a.lo <= 0. && a.hi >= 0. {
        0.
    } else {
        a.lo.abs().min(a.hi.abs())
    };
    let hi = a.lo.abs().max(a.hi.abs());
    I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())
}
fn normalize(a: Jet) -> Result<Option<Jet>> {
    let s = square(a.v[0])?.add(square(a.v[1])?)?.add(square(a.v[2])?)?;
    let n = I::new(
        s.lo.max(0.).sqrt().next_down().max(0.),
        s.hi.sqrt().next_up(),
    )?;
    if n.lo <= 0. {
        return Ok(None);
    }
    let nd = dot(a.v, a.d)?.div(n)?;
    let ndd = dot(a.d, a.d)?
        .add(dot(a.v, a.dd)?)?
        .sub(square(nd)?)?
        .div(n)?;
    let v = div(a.v, n)?;
    let d = div(sub(a.d, mul(v, nd)?)?, n)?;
    let dd = div(
        sub(sub(a.dd, mul(v, ndd)?)?, mul(d, nd.mul(I::point(2.))?)?)?,
        n,
    )?;
    Ok(Some(Jet { v, d, dd }))
}
fn decode(a: [[f64; 2]; 3]) -> Result<V> {
    Ok([
        I::new(a[0][0], a[0][1])?,
        I::new(a[1][0], a[1][1])?,
        I::new(a[2][0], a[2][1])?,
    ])
}
fn jet(c: &Curve, r: &vector_certificate::Report) -> Result<Jet> {
    let [lo, hi] = c.domain();
    let span = I::point(hi).sub(I::point(lo))?;
    Ok(Jet {
        v: decode(r.value.unwrap())?,
        d: mul(decode(r.first.unwrap())?, span)?,
        dd: mul(decode(r.second.unwrap())?, span.mul(span)?)?,
    })
}
fn encode(a: Jet) -> FrameJet {
    FrameJet {
        value: a.v.map(|x| [x.lo, x.hi]),
        first: a.d.map(|x| [x.lo, x.hi]),
        second: a.dd.map(|x| [x.lo, x.hi]),
    }
}
fn decode_jet(a: &FrameJet) -> Result<Jet> {
    Ok(Jet {
        v: decode(a.value)?,
        d: decode(a.first)?,
        dd: decode(a.second)?,
    })
}
fn scalar_product(a: Jet, value: I, first: I, second: I) -> Result<Jet> {
    Ok(Jet {
        v: mul(a.v, value)?,
        d: add(mul(a.d, value)?, mul(a.v, first)?)?,
        dd: add(
            add(mul(a.dd, value)?, mul(a.d, first.mul(I::point(2.))?)?)?,
            mul(a.v, second)?,
        )?,
    })
}
fn add_jet(a: Jet, b: Jet) -> Result<Jet> {
    Ok(Jet {
        v: add(a.v, b.v)?,
        d: add(a.d, b.d)?,
        dd: add(a.dd, b.dd)?,
    })
}
/// Twist rotates about the authored longitudinal axis, independently of path
/// transport. Both vector laws and the scalar twist share one cell budget.
pub fn certify_twisted(
    longitudinal: &Curve,
    transverse: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    super::validate_law(twist, false)?;
    let out = certify(longitudinal, transverse, traversal, max_cells)?;
    apply_twist(out, twist, traversal, max_cells)
}
fn apply_twist(
    mut out: Report,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    if out.status != Status::Certified {
        return Ok(out);
    }
    let theta =
        super::scalar_certificate::certify_traversal(twist, traversal, max_cells - out.cells)?;
    out.cells += theta.cells;
    if theta.status != Status::Certified {
        out.status = Status::Unresolved;
        out.reason = Some("twist-law-enclosure-unresolved");
        out.longitudinal = None;
        out.transverse = None;
        out.binormal = None;
        out.single_span = false;
        return Ok(out);
    }
    let [lo, hi] = twist.domain();
    let span = I::point(hi).sub(I::point(lo))?;
    let td = I::new(theta.first.unwrap()[0], theta.first.unwrap()[1])?.mul(span)?;
    let tdd = I::new(theta.second.unwrap()[0], theta.second.unwrap()[1])?.mul(span.mul(span)?)?;
    let tr = super::trigonometric_certificate::certify(theta.value.unwrap())?;
    let s = I::new(tr.sin[0], tr.sin[1])?;
    let c = I::new(tr.cos[0], tr.cos[1])?;
    let sd = c.mul(td)?;
    let cd = I::point(0.).sub(s.mul(td)?)?;
    let sdd = c.mul(tdd)?.sub(s.mul(square(td)?)?)?;
    let cdd = I::point(0.).sub(s.mul(tdd)?)?.sub(c.mul(square(td)?)?)?;
    let t = decode_jet(out.longitudinal.as_ref().unwrap())?;
    let n = decode_jet(out.transverse.as_ref().unwrap())?;
    let b = decode_jet(out.binormal.as_ref().unwrap())?;
    let normal = add_jet(
        scalar_product(n, c, cd, cdd)?,
        scalar_product(b, s, sd, sdd)?,
    )?;
    out.transverse = Some(encode(normal));
    out.binormal = Some(encode(cross_jet(t, normal)?));
    out.single_span &= theta.single_span;
    Ok(out)
}
/// Point-safe guide frame values for stored station comparisons. Work uses
/// the same conservative active-span accounting as authored frame values.
pub fn certify_guide_values(
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    tangent: [[f64; 2]; 3],
    path: [[f64; 2]; 3],
    max_cells: usize,
) -> Result<ValuesReport> {
    certify_guide_values_at(guide,twist,traversal,traversal,tangent,path,max_cells)
}
fn certify_guide_values_at(
    guide:&Curve,twist:&Curve,traversal:[f64;2],law_traversal:[f64;2],
    tangent:[[f64;2];3],path:[[f64;2];3],max_cells:usize,
)->Result<ValuesReport>{
    super::validate_law(twist, false)?;
    let rail = vector_certificate::certify_values_traversal(guide, traversal, max_cells, false)?;
    let mut out = ValuesReport {
        status: Status::Unresolved,
        cells: rail.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
    };
    let Some(rail) = rail.value else {
        return Ok(out);
    };
    let offset=sub(decode(rail)?,decode(path)?)?;
    let mut frame=certify_guide_values_from_offset(twist,law_traversal,tangent,offset,max_cells-out.cells)?;
    frame.cells+=out.cells;
    Ok(frame)
}
fn certify_guide_values_from_offset(
    twist:&Curve,law_traversal:[f64;2],tangent:[[f64;2];3],offset:V,max_cells:usize,
)->Result<ValuesReport>{
    super::validate_law(twist,false)?;
    let mut out=ValuesReport {status:Status::Unresolved,cells:0,
        longitudinal:None,transverse:None,binormal:None};
    let charge = (twist.degree..twist.control_points.len())
        .filter(|&i| twist.knots[i] < twist.knots[i + 1])
        .count();
    if charge > max_cells - out.cells {
        return Ok(out);
    }
    let theta = super::scalar_certificate::value_traversal(twist, law_traversal, charge)?;
    out.cells += charge;
    let Some(theta) = theta else {
        return Ok(out);
    };
    let constant = |v| Jet {
        v,
        d: [I::point(0.); 3],
        dd: [I::point(0.); 3],
    };
    let Some(t) = normalize(constant(decode(tangent)?))? else {
        return Ok(out);
    };
    // Positive speed cancels in binormal normalization. Use original
    // velocity directly instead of repeating the interval tangent quotient.
    let Some(b) = normalize(constant(cross(decode(tangent)?, offset)?))? else {
        return Ok(out);
    };
    let n = cross(b.v, t.v)?;
    let tr = super::trigonometric_certificate::certify(theta)?;
    let rotated = add(
        mul(n, I::new(tr.cos[0], tr.cos[1])?)?,
        mul(b.v, I::new(tr.sin[0], tr.sin[1])?)?,
    )?;
    out.longitudinal = Some(t.v.map(|x| [x.lo, x.hi]));
    out.transverse = Some(rotated.map(|x| [x.lo, x.hi]));
    out.binormal = Some(cross(t.v, rotated)?.map(|x| [x.lo, x.hi]));
    out.status = Status::Certified;
    Ok(out)
}
/// A second spatial rail controls the normal toward guide(t)-path(t),
/// projected perpendicular to the constant tangent of one polyline edge.
/// Path position and normalized-traversal velocity are outward enclosures;
/// the path acceleration on this edge is zero. No contact fitting is implied.
pub fn certify_guide(
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    tangent: [[f64; 2]; 3],
    path: [[f64; 2]; 3],
    velocity: [[f64; 2]; 3],
    max_cells: usize,
) -> Result<Report> {
    super::validate_law(twist, false)?;
    let rail = vector_certificate::certify_traversal(guide, traversal, max_cells, false)?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: rail.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some("guide-law-enclosure-unresolved"),
    };
    if rail.status != Status::Certified {
        return Ok(out);
    }
    let g = jet(guide, &rail)?;
    let zero = [I::point(0.); 3];
    out.reason = Some("path-tangent-nonzero-unproved");
    let Some(t) = normalize(Jet {
        v: decode(tangent)?,
        d: zero,
        dd: zero,
    })?
    else {
        return Ok(out);
    };
    let offset = Jet {
        v: sub(g.v, decode(path)?)?,
        d: sub(g.d, decode(velocity)?)?,
        dd: g.dd,
    };
    out.reason = Some("guide-transverse-direction-unproved");
    let Some(b) = normalize(cross_jet(t, offset)?)? else {
        return Ok(out);
    };
    let n = cross_jet(b, t)?;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = rail.single_span;
    out.reason = None;
    out.status = Status::Certified;
    apply_twist(out, twist, traversal, max_cells)
}
/// Right-handed frame: t=unit(longitudinal), b=unit(t×transverse),
/// n=b×t. Both laws map their own knot domains to the same traversal.
pub fn certify(
    longitudinal: &Curve,
    transverse: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    check(
        max_cells <= 100000,
        "Authored frame budget exceeds100000 cells",
    )?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: 0,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some("vector-law-enclosure-unresolved"),
    };
    let a = vector_certificate::certify_traversal(longitudinal, traversal, max_cells, false)?;
    out.cells += a.cells;
    if a.status != Status::Certified {
        return Ok(out);
    }
    let b =
        vector_certificate::certify_traversal(transverse, traversal, max_cells - out.cells, false)?;
    out.cells += b.cells;
    if b.status != Status::Certified {
        return Ok(out);
    }
    out.reason = Some("longitudinal-nonzero-unproved");
    let Some(t) = normalize(jet(longitudinal, &a)?)? else {
        return Ok(out);
    };
    out.reason = Some("transverse-nonparallel-unproved");
    let Some(binormal) = normalize(cross_jet(t, jet(transverse, &b)?)?)? else {
        return Ok(out);
    };
    let normal = cross_jet(binormal, t)?;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(normal));
    out.binormal = Some(encode(binormal));
    out.single_span = a.single_span && b.single_span;
    out.status = Status::Certified;
    out.reason = None;
    Ok(out)
}

/// Whole-traversal frame regularity, with one shared budget across adaptive
/// restrictions of the original laws. This proves nonzero longitudinal and
/// nonparallel transverse directions, not sweep error or global embedding.
#[derive(Clone, Debug)]
pub struct RegularityReport {
    pub status: Status,
    pub cells: usize,
    pub certified_intervals: usize,
    pub reason: Option<&'static str>,
    /// Complete ordered traversal cover only; partial work is never published
    /// as a family of usable frame jets.
    pub intervals: Option<Vec<RegularFrameInterval>>,
}
#[derive(Clone, Debug)]
pub struct RegularFrameInterval {
    pub traversal: [f64; 2],
    pub longitudinal: FrameJet,
    pub transverse: FrameJet,
    pub binormal: FrameJet,
}
pub fn certify_regularity(
    longitudinal: &Curve,
    transverse: &Curve,
    max_cells: usize,
) -> Result<RegularityReport> {
    check(max_cells <= 100000, "Authored frame budget exceeds100000 cells")?;
    longitudinal.validate()?;
    transverse.validate()?;
    certify_frame_cover(max_cells, |interval, remaining| {
        certify(longitudinal, transverse, interval, remaining)
    })
}
/// Complete twisted authored-frame jet cover. Twist and both direction laws
/// charge the same adaptive budget; all derivatives use normalized traversal.
pub fn certify_twisted_cover(
    longitudinal: &Curve,
    transverse: &Curve,
    twist: &Curve,
    max_cells: usize,
) -> Result<RegularityReport> {
    check(max_cells <= 100000, "Authored frame budget exceeds100000 cells")?;
    longitudinal.validate()?;
    transverse.validate()?;
    super::validate_law(twist, false)?;
    certify_frame_cover(max_cells, |interval, remaining| {
        certify_twisted(longitudinal, transverse, twist, interval, remaining)
    })
}
/// Whole-traversal fixed frame and independent twist regularity.
pub fn certify_fixed_cover(path:&Curve,normal:[f64;3],twist:&Curve,max_cells:usize)->Result<RegularityReport>{
    check(max_cells<=100000,"Fixed frame cover work exceeds100000 cells")?;
    path.validate()?;super::validate_law(twist,false)?;
    check(normal.iter().all(|x|x.is_finite())&&path.control_points.iter().all(|p|p.len()==3),"Fixed cover requires finite XYZ source and normal")?;
    certify_frame_cover(max_cells,|interval,remaining|certify_fixed_path(path,normal,twist,interval,remaining))
}
/// Original same-traversal guided frame cover. Independent arc inversions
/// are not supplied by this cover.
pub fn certify_guided_cover(path:&Curve,guide:&Curve,twist:&Curve,max_cells:usize)->Result<RegularityReport>{
    check(max_cells<=100000,"Guided frame cover work exceeds100000 cells")?;
    path.validate()?;guide.validate()?;super::validate_law(twist,false)?;
    check([path,guide].iter().all(|c|c.control_points.iter().all(|p|p.len()==3)),"Guided cover requires XYZ source curves")?;
    certify_frame_cover(max_cells,|interval,remaining|certify_path_guide(path,guide,twist,interval,remaining))
}
fn certify_frame_cover(
    max_cells: usize,
    mut certify_interval: impl FnMut([f64; 2], usize) -> Result<Report>,
) -> Result<RegularityReport> {
    let mut out = RegularityReport {
        status: Status::Unresolved,
        cells: 0,
        certified_intervals: 0,
        reason: Some("frame-regularity-work-limit"),
        intervals: None,
    };
    let mut pending = vec![[0., 1.]];
    let mut intervals = Vec::new();
    while let Some(interval) = pending.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        let report = certify_interval(interval, max_cells - out.cells)?;
        out.cells += report.cells;
        if report.status == Status::Certified {
            out.certified_intervals += 1;
            intervals.push(RegularFrameInterval {
                traversal: interval,
                longitudinal: report.longitudinal.unwrap(),
                transverse: report.transverse.unwrap(),
                binormal: report.binormal.unwrap(),
            });
            continue;
        }
        if report.cells == 0 || out.cells == max_cells {
            return Ok(out);
        }
        let middle = interval[0] + (interval[1] - interval[0]) * 0.5;
        if middle == interval[0] || middle == interval[1] {
            out.reason = Some("frame-regularity-subdivision-unresolved");
            return Ok(out);
        }
        pending.push([middle, interval[1]]);
        pending.push([interval[0], middle]);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.intervals = Some(intervals);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn law(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![domain[0], domain[0], domain[1], domain[1]],
            weights: vec![1.; points.len()],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn rotating_frame_has_normalized_traversal_jets_and_shared_budget() {
        let a = law(vec![vec![1., 0., 0.], vec![1., 1., 0.]], [2., 5.]);
        let b = law(vec![vec![0., 0., 2.], vec![0., 0., 2.]], [7., 9.]);
        let before = (a.clone(), b.clone());
        let r = certify(&a, &b, [0., 1.], 6).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert_eq!(r.cells, 6);
        for i in 0..=20 {
            let f = i as f64 / 20.;
            let h = (1. + f * f).sqrt();
            let v = [1. / h, f / h, 0.];
            let d = [-f / h.powi(3), 1. / h.powi(3), 0.];
            let dd = [(2. * f * f - 1.) / h.powi(5), -3. * f / h.powi(5), 0.];
            let t = r.longitudinal.as_ref().unwrap();
            for k in 0..3 {
                for (range, x) in [(t.value[k], v[k]), (t.first[k], d[k]), (t.second[k], dd[k])] {
                    assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
                }
            }
        }
        for budget in [0, 3, 5] {
            let r = certify(&a, &b, [0., 1.], budget).unwrap();
            assert_eq!(r.status, Status::Unresolved);
            assert!(r.cells <= budget);
            assert!(r.longitudinal.is_none() && r.transverse.is_none() && r.binormal.is_none());
        }
        assert_eq!(a.control_points, before.0.control_points);
        assert_eq!(b.knots, before.1.knots);
    }
    #[test]
    fn rational_frame_encloses_all_axes_and_chain_rule_derivatives() {
        let mut a = law(vec![vec![1., 0., 0.], vec![1., 1., 0.]], [2., 5.]);
        a.weights = vec![1., 0.5];
        let b = law(vec![vec![0., 0., 2.], vec![0., 0., 2.]], [7., 9.]);
        let r = certify(&a, &b, [0., 1.], 6).unwrap();
        assert_eq!(r.status, Status::Certified);
        for i in 0..=20 {
            let f = i as f64 / 20.;
            let denominator = 1. - 0.5 * f;
            let g = 0.5 * f / denominator;
            let gd = 0.5 / denominator.powi(2);
            let gdd = 0.5 / denominator.powi(3);
            let h = (1. + g * g).sqrt();
            let v = [1. / h, g / h, 0.];
            let d = [-g / h.powi(3) * gd, gd / h.powi(3), 0.];
            let dd = [
                (2. * g * g - 1.) / h.powi(5) * gd * gd - g / h.powi(3) * gdd,
                -3. * g / h.powi(5) * gd * gd + gdd / h.powi(3),
                0.,
            ];
            for (axis, values, first, second) in [
                (r.longitudinal.as_ref().unwrap(), v, d, dd),
                (
                    r.transverse.as_ref().unwrap(),
                    [0., 0., 1.],
                    [0.; 3],
                    [0.; 3],
                ),
                (
                    r.binormal.as_ref().unwrap(),
                    [v[1], -v[0], 0.],
                    [d[1], -d[0], 0.],
                    [dd[1], -dd[0], 0.],
                ),
            ] {
                for k in 0..3 {
                    for (range, x) in [
                        (axis.value[k], values[k]),
                        (axis.first[k], first[k]),
                        (axis.second[k], second[k]),
                    ] {
                        assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
                    }
                }
            }
        }
    }
    #[test]
    fn twist_uses_independent_domain_and_shared_frame_budget() {
        let a = law(vec![vec![0., 0., 1.], vec![0., 0., 1.]], [2., 5.]);
        let b = law(vec![vec![1., 0., 0.], vec![1., 0., 0.]], [7., 9.]);
        let mut twist = law(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [11., 15.]);
        twist.weights = vec![1., 0.5];
        let r = certify_twisted(&a, &b, &twist, [0., 1.], 7).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert_eq!(r.cells, 7);
        for i in 0..=20 {
            let f = i as f64 / 20.;
            let denominator = 1. - 0.5 * f;
            let theta = 0.5 * f / denominator;
            let td = 0.5 / denominator.powi(2);
            let tdd = 0.5 / denominator.powi(3);
            let (s, c) = theta.sin_cos();
            let values = certify_twisted_values(&a, &b, &twist, [f, f], 7).unwrap();
            assert_eq!(values.status, Status::Certified);
            assert_eq!(values.cells, 7);
            for (range, x) in values.transverse.unwrap().into_iter().zip([c, s, 0.]) {
                assert!(range[0] <= x && x <= range[1]);
            }
            let v = [c, s, 0.];
            let d = [-s * td, c * td, 0.];
            let dd = [-c * td * td - s * tdd, -s * td * td + c * tdd, 0.];
            for (axis, values, first, second) in [
                (r.transverse.as_ref().unwrap(), v, d, dd),
                (
                    r.binormal.as_ref().unwrap(),
                    [-v[1], v[0], 0.],
                    [-d[1], d[0], 0.],
                    [-dd[1], dd[0], 0.],
                ),
                (
                    r.longitudinal.as_ref().unwrap(),
                    [0., 0., 1.],
                    [0.; 3],
                    [0.; 3],
                ),
            ] {
                for k in 0..3 {
                    for (range, x) in [
                        (axis.value[k], values[k]),
                        (axis.first[k], first[k]),
                        (axis.second[k], second[k]),
                    ] {
                        assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
                    }
                }
            }
        }
        let refused = certify_twisted(&a, &b, &twist, [0., 1.], 6).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert_eq!(refused.cells, 6);
        assert!(
            refused.longitudinal.is_none()
                && refused.transverse.is_none()
                && refused.binormal.is_none()
        );
        assert_eq!(refused.reason, Some("twist-law-enclosure-unresolved"));
        let values = certify_twisted_values(&a, &b, &twist, [0., 0.], 6).unwrap();
        assert_eq!(values.status, Status::Unresolved);
        assert_eq!(values.cells, 6);
        assert!(values.transverse.is_none() && values.binormal.is_none());
    }
    #[test]
    fn guide_projection_jets_follow_the_original_rail_and_shared_budget() {
        let guide = law(vec![vec![1., 0., 0.], vec![1., 1., 10.]], [7., 9.]);
        let twist = law(vec![vec![0., 0., 0.]; 2], [2., 5.]);
        let tangent = [[0., 0.], [0., 0.], [1., 1.]];
        let path = [[0., 0.], [0., 0.], [0., 10.]];
        let velocity = [[0., 0.], [0., 0.], [10., 10.]];
        let r = certify_guide(&guide, &twist, [0., 1.], tangent, path, velocity, 4).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert_eq!(r.cells, 4);
        let n = r.transverse.as_ref().unwrap();
        for i in 0..=20 {
            let f = i as f64 / 20.;
            let h = (1. + f * f).sqrt();
            let position = [[0., 0.], [0., 0.], [10. * f, 10. * f]];
            let values =
                certify_guide_values(&guide, &twist, [f, f], tangent, position, 4).unwrap();
            assert_eq!(values.status, Status::Certified);
            assert_eq!(values.cells, 4);
            for (range, x) in values
                .transverse
                .unwrap()
                .into_iter()
                .zip([1. / h, f / h, 0.])
            {
                assert!(range[0] <= x && x <= range[1]);
            }

            let v = [1. / h, f / h, 0.];
            let d = [-f / h.powi(3), 1. / h.powi(3), 0.];
            let dd = [(2. * f * f - 1.) / h.powi(5), -3. * f / h.powi(5), 0.];
            for k in 0..3 {
                for (range, x) in [(n.value[k], v[k]), (n.first[k], d[k]), (n.second[k], dd[k])] {
                    assert!(range[0] <= x && x <= range[1]);
                }
            }
        }
        let r = certify_guide(&guide, &twist, [0., 1.], tangent, path, velocity, 3).unwrap();
        assert_eq!(r.status, Status::Unresolved);
        assert_eq!(r.cells, 3);
        assert!(r.transverse.is_none());
        let crossing = law(vec![vec![1., 0., 0.], vec![-1., 0., 10.]], [7., 9.]);
        let r = certify_guide(&crossing, &twist, [0., 1.], tangent, path, velocity, 4).unwrap();
        assert_eq!(r.reason, Some("guide-transverse-direction-unproved"));
        assert!(r.longitudinal.is_none());
    }
    #[test]
    fn singular_and_parallel_directions_refuse_without_partial_frame() {
        let a = law(vec![vec![1., 0., 0.], vec![-1., 0., 0.]], [0., 1.]);
        let b = law(vec![vec![0., 1., 0.], vec![0., 1., 0.]], [0., 1.]);
        let r = certify(&a, &b, [0., 1.], 6).unwrap();
        assert_eq!(r.reason, Some("longitudinal-nonzero-unproved"));
        assert!(r.longitudinal.is_none());
        let a = law(vec![vec![1., 0., 0.], vec![1., 0., 0.]], [0., 1.]);
        let r = certify(&a, &a, [0., 1.], 6).unwrap();
        assert_eq!(r.reason, Some("transverse-nonparallel-unproved"));
        assert_eq!(r.status, Status::Unresolved);
    }

    #[test]
    fn interior_singularity_between_preview_stations_is_not_a_regular_frame() {
        // z(t)=(t-3/8)^2: every quarter-grid station is nonzero, while
        // the original polynomial has an exact zero between those stations.
        let axis = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 10., 10., 10.],
            control_points: vec![
                vec![0., 0., 9. / 64.],
                vec![0., 0., -15. / 64.],
                vec![0., 0., 25. / 64.],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        let normal = law(vec![vec![1., 0., 0.]; 2], [31., 41.]);
        for k in 0..=4 {
            assert!(axis.evaluate(2. + 2. * k as f64).unwrap().point[2] > 0.);
        }
        assert_eq!(axis.evaluate(5.).unwrap().point, vec![0., 0., 0.]);
        let report = certify(&axis, &normal, [0., 1.], 100).unwrap();
        assert_eq!(report.status, Status::Unresolved);
        assert_eq!(report.reason, Some("longitudinal-nonzero-unproved"));
        assert!(report.longitudinal.is_none() && report.binormal.is_none());

        // A nonzero transverse vector can likewise become parallel to the
        // fixed axis between the same preview stations.
        let mut transverse = axis.clone();
        for pole in &mut transverse.control_points {
            pole[0] = pole[2];
            pole[2] = 1.;
        }
        let fixed_axis = law(vec![vec![0., 0., 1.]; 2], [7., 9.]);
        let report = certify(&fixed_axis, &transverse, [0., 1.], 100).unwrap();
        assert_eq!(report.status, Status::Unresolved);
        assert_eq!(report.reason, Some("transverse-nonparallel-unproved"));
        assert!(report.transverse.is_none() && report.binormal.is_none());
        let whole = certify_regularity(&fixed_axis, &transverse, 100).unwrap();
        assert_eq!(whole.status, Status::Unresolved);
        assert!(whole.cells <= 100);
    }

    #[test]
    fn whole_frame_regularity_subdivides_original_laws_with_shared_work() {
        let axis = law(vec![vec![1., 0., 0.], vec![-1., 1., 0.]], [2., 5.]);
        let normal = law(vec![vec![0., 0., 1.]; 2], [31., 41.]);
        assert_eq!(certify(&axis, &normal, [0., 1.], 100).unwrap().status, Status::Unresolved);
        let whole = certify_regularity(&axis, &normal, 1000).unwrap();
        assert_eq!(whole.status, Status::Certified);
        assert!(whole.certified_intervals > 1);
        assert!(whole.cells <= 1000);
        let cover = whole.intervals.as_ref().unwrap();
        assert_eq!(cover.len(), whole.certified_intervals);
        assert_eq!(cover.first().unwrap().traversal[0], 0.);
        assert_eq!(cover.last().unwrap().traversal[1], 1.);
        for pair in cover.windows(2) {
            assert_eq!(pair[0].traversal[1], pair[1].traversal[0]);
        }
        let limited = certify_regularity(&axis, &normal, whole.cells - 1).unwrap();
        assert_eq!(limited.status, Status::Unresolved);
        assert!(limited.cells < whole.cells);
        assert!(limited.intervals.is_none());
        assert_eq!(certify_regularity(&axis, &normal, 0).unwrap().status, Status::Unresolved);
        #[cfg(feature="transport")]
        for budget in [0, 1000] {
            let request = value_codec::json!({
                "op": "sweep_authored_frame_regularity",
                "longitudinal": axis,
                "transverse": normal,
                "maxCells": budget
            });
            let result = crate::transport::dispatch(request).unwrap();
            assert_eq!(result["regularityCertified"], value_codec::json!(budget > 0));
            assert_eq!(result["continuousBound"], value_codec::json!(false));
        }
    }

    #[test]
    fn twisted_cover_retains_normalized_second_jets_and_shared_work() {
        let axis = law(vec![vec![0., 0., 1.]; 2], [2., 5.]);
        let normal = law(vec![vec![1., 0., 0.]; 2], [31., 41.]);
        let twist = law(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [17., 19.]);
        let cover = certify_twisted_cover(&axis, &normal, &twist, 100).unwrap();
        assert_eq!(cover.status, Status::Certified);
        assert_eq!(cover.cells, 7);
        let intervals = cover.intervals.unwrap();
        assert_eq!(intervals.len(), 1);
        let jet = &intervals[0].transverse;
        for k in 0..=16 {
            let t = k as f64 / 16.;
            let (s, c) = t.sin_cos();
            for (bounds, expected) in [
                (jet.value, [c, s, 0.]),
                (jet.first, [-s, c, 0.]),
                (jet.second, [-c, -s, 0.]),
            ] {
                for i in 0..3 {
                    assert!(bounds[i][0] <= expected[i] && expected[i] <= bounds[i][1]);
                }
            }
        }
        for budget in [0, 6] {
            let refused = certify_twisted_cover(&axis, &normal, &twist, budget).unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cells <= budget);
            assert!(refused.intervals.is_none());
        }
    }
}

pub(crate) use frenet_path_values::certify_frenet_relative_values;

pub(crate) use guided_path_values::{certify_path_guide_relative_values,certify_contact_relative_values};

pub(crate) use guided_path_values::certify_independent_guided_cover;

/// Internal fresh-source frame owner supplies its already charged frame.
pub(crate) fn relative_control_values_from_frame(
 scale:&Curve, affine:Option<(&Curve,&Curve)>, qs:&[[[f64;2];3]],
 station:[f64;2], max_cells:usize, frame:ValuesReport,
)->Result<ControlValuesReport>{
 let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
 trajectory::control_values_with_fit(&zero,scale,affine,qs,station,max_cells,frame,None)
}
