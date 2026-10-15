//! Original frame jets in the station coordinate, with independently proved
//! normalized source-parameter maps. No inverse derivative is guessed.
use super::*;
pub(super) fn arc_frame(
    s: &Sweep<'_>,
    station: [f64; 2],
    p: frame::PathPhase,
    g: Option<frame::PathPhase>,
    offset: Option<&Curve>,
    transport: Option<&OriginalRmfTransportReport>,
    shared_guide: bool,
    zero: &Curve,
    max: usize,
) -> Result<Option<frame::Report>> {
    if let Some((a, n)) = s.frame_laws {
        return frame::certify_twisted(a, n, s.twist, station, max).map(Some);
    }
    if let Some(guide) = s.orientation_guide {
        if shared_guide {
            if let Some(offset) = offset {
                let raw =
                    frame::certify_path_guide_from_offset(s.path, offset, zero, p.traversal, max)?;
                return frame::compose_and_twist(raw, p, s.twist, station, max).map(Some);
            }
        }
        return frame::certify_guided_phase_jets(
            s.path,
            guide,
            s.twist,
            station,
            p,
            g.unwrap(),
            max,
        )
        .map(Some);
    }
    if s.options.orientation == Orientation::Fixed {
        return frame::certify_fixed_path(s.path, s.options.normal, s.twist, station, max)
            .map(Some);
    }
    let raw = if let Some(transport) = transport {
        super::super::rmf_transport::frame_jets(transport, s.path, zero, p.traversal, max)?
    } else {
        match s.options.orientation {
            Orientation::FixedNormal => {
                frame::certify_fixed_normal_path(s.path, s.options.normal, zero, p.traversal, max)?
            }
            Orientation::Frenet => frame::certify_frenet_path(s.path, zero, p.traversal, max)?,
            // The owning global frame-continuity proof has already established a
            // forward-line or plane premise for unguided nonspatial RMF.
            Orientation::RotationMinimizing => {
                frame::certify_fixed_normal_path(s.path, s.options.normal, zero, p.traversal, max)?
            }
            Orientation::CorrectedFrenet if super::super::authored_error::original_line(s.path) => {
                frame::certify_fixed_path(s.path, s.options.normal, zero, p.traversal, max)?
            }
            _ => return Ok(None),
        }
    };
    frame::compose_and_twist(raw, p, s.twist, station, max).map(Some)
}
pub(super) fn compose_width(
    mut w: frame::GuideWidthReport,
    p: frame::PathPhase,
) -> Result<frame::GuideWidthReport> {
    if w.status == Status::Certified {
        let first = I::new(w.first.unwrap()[0], w.first.unwrap()[1])?;
        let second = I::new(w.second.unwrap()[0], w.second.unwrap()[1])?;
        let d = first.mul(p.first)?;
        let dd = second
            .mul(p.first.mul(p.first)?)?
            .add(first.mul(p.second)?)?;
        w.first = Some([d.lo, d.hi]);
        w.second = Some([dd.lo, dd.hi]);
    }
    Ok(w)
}
