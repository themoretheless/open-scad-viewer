//! Native-owned normalized source-parameter phase jets. A caller must prove
//! the inverse correspondence before using these private composition helpers.
use super::*;
#[derive(Clone, Copy)]
pub(crate) struct PathPhase {
    pub traversal: [f64; 2],
    pub first: I,
    pub second: I,
    pub position_jets: Option<[[I; 3]; 2]>,
}
impl PathPhase {
    pub(crate) fn identity(traversal: [f64; 2]) -> Self {
        Self {
            traversal,
            first: I::point(1.),
            second: I::point(0.),
            position_jets: None,
        }
    }
}
pub(super) fn compose(j: Jet, p: PathPhase) -> Result<Jet> {
    if p.first.lo == 1. && p.first.hi == 1. && p.second.lo == 0. && p.second.hi == 0. {
        return Ok(j);
    }
    Ok(Jet {
        v: j.v,
        d: mul(j.d, p.first)?,
        dd: add(mul(j.dd, p.first.mul(p.first)?)?, mul(j.d, p.second)?)?,
    })
}
/// Original tangent derivatives via skew angular velocity preserve cancellation
/// of longitudinal acceleration, including nonuniform rational line parameters.
pub(super) fn source_tangent(v: V, a: V, j: V) -> Result<Option<Jet>> {
    let Some(mut t) = normalize(Jet {
        v,
        d: [I::point(0.); 3],
        dd: [I::point(0.); 3],
    })?
    else {
        return Ok(None);
    };
    let q = square(v[0])?.add(square(v[1])?)?.add(square(v[2])?)?;
    if q.lo <= 0. {
        return Ok(None);
    }
    let omega = div(cross(v, a)?, q)?;
    let omega_d = sub(
        div(cross(v, j)?, q)?,
        mul(omega, dot(v, a)?.mul(I::point(2.))?.div(q)?)?,
    )?;
    t.d = cross(omega, t.v)?;
    t.dd = add(cross(omega_d, t.v)?, cross(omega, t.d)?)?;
    Ok(Some(t))
}
/// Position derivatives may preserve P_s = L T from the SAME original
/// normalized arc map. They must not be used for a relative vector/frame jet.
pub(super) fn path_position(j: Jet, p: PathPhase) -> Result<Jet> {
    let mut out = compose(j, p)?;
    if let Some([first, second]) = p.position_jets {
        out.d = first;
        out.dd = second;
    }
    Ok(out)
}
pub(crate) fn compose_and_twist(
    mut out: Report,
    p: PathPhase,
    twist: &Curve,
    station: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    if out.status == Status::Certified {
        out.longitudinal = Some(encode(compose(
            decode_jet(out.longitudinal.as_ref().unwrap())?,
            p,
        )?));
        out.transverse = Some(encode(compose(
            decode_jet(out.transverse.as_ref().unwrap())?,
            p,
        )?));
        out.binormal = Some(encode(compose(
            decode_jet(out.binormal.as_ref().unwrap())?,
            p,
        )?));
    }
    apply_twist(out, twist, station, max_cells)
}
/// Separate original path and guide normalized arc maps share only the station
/// coordinate. Tangent jets are composed after normalization, avoiding any need
/// to assume an unproved third derivative of the inverse length map.
pub(crate) fn certify_guided_phase_jets(
    path: &Curve,
    guide: &Curve,
    twist: &Curve,
    station: [f64; 2],
    p: PathPhase,
    g: PathPhase,
    max_cells: usize,
) -> Result<Report> {
    let source = vector_certificate::certify_third_traversal(path, p.traversal, max_cells)?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: source.base.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some("arc-guide-original-path-jets-unproved"),
    };
    if source.base.status != Status::Certified {
        return Ok(out);
    }
    let path_jet = jet(path, &source.base)?;
    let [a, b] = path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let jerk = mul(
        decode(source.third.unwrap())?,
        width.mul(width)?.mul(width)?,
    )?;
    let Some(t) = source_tangent(path_jet.d, path_jet.dd, jerk)? else {
        return Ok(out);
    };
    let t = compose(t, p)?;
    let position = path_position(path_jet, p)?;
    let guide_jet =
        vector_certificate::certify_traversal(guide, g.traversal, max_cells - out.cells, false)?;
    out.cells += guide_jet.cells;
    if guide_jet.status != Status::Certified {
        return Ok(out);
    }
    let rail = path_position(jet(guide, &guide_jet)?, g)?;
    let offset = Jet {
        v: sub(rail.v, position.v)?,
        d: sub(rail.d, position.d)?,
        dd: sub(rail.dd, position.dd)?,
    };
    let Some(b) = normalize(cross_jet(t, offset)?)? else {
        return Ok(out);
    };
    let n = cross_jet(b, t)?;
    out.status = Status::Certified;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = source.base.single_span && guide_jet.single_span;
    out.reason = None;
    apply_twist(out, twist, station, max_cells)
}
/// Original distance jets used by contact fitting at the SAME station phase.
pub(crate) fn certify_phase_guide_width(
    path: &Curve,
    guide: &Curve,
    p: PathPhase,
    g: PathPhase,
    max_cells: usize,
) -> Result<GuideWidthReport> {
    let a = vector_certificate::certify_traversal(path, p.traversal, max_cells, false)?;
    let mut out = GuideWidthReport {
        status: Status::Unresolved,
        cells: a.cells,
        single_span: false,
        value: None,
        first: None,
        second: None,
    };
    if a.status != Status::Certified {
        return Ok(out);
    }
    let b =
        vector_certificate::certify_traversal(guide, g.traversal, max_cells - out.cells, false)?;
    out.cells += b.cells;
    if b.status != Status::Certified {
        return Ok(out);
    }
    let a = path_position(jet(path, &a)?, p)?;
    let b = path_position(jet(guide, &b)?, g)?;
    let v = sub(b.v, a.v)?;
    let d = sub(b.d, a.d)?;
    let dd = sub(b.dd, a.dd)?;
    let speed2 = square(v[0])?.add(square(v[1])?)?.add(square(v[2])?)?;
    let length = I::new(
        speed2.lo.max(0.).sqrt().next_down().max(0.),
        speed2.hi.sqrt().next_up(),
    )?;
    if length.lo <= 0. {
        return Ok(out);
    }
    let first = dot(v, d)?.div(length)?;
    let second = dot(d, d)?
        .add(dot(v, dd)?)?
        .sub(square(first)?)?
        .div(length)?;
    out.status = Status::Certified;
    out.value = Some([length.lo, length.hi]);
    out.first = Some([first.lo, first.hi]);
    out.second = Some([second.lo, second.hi]);
    // This sufficient flag concerns only original knot spans, not polynomiality.
    out.single_span = false;
    Ok(out)
}
