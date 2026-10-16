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
    if let Some(transport) = transport {
        return if transport.closing_angle.is_some() {
            super::super::rmf_transport::closed_phase_jets(
                transport, s.path, s.twist, station, p, max,
            )
        } else {
            super::super::rmf_transport::phase_jets(transport, s.path, s.twist, station, p, max)
        }
        .map(Some);
    }
    let raw = {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_inverse_arc_phase_composes_both_correction_jets_in_station_units() {
        // Analytic nonuniform forward line; the correction is a specified open
        // angular law, not a claim of closed holonomy for this line.
        let path = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let zero = constant_vector_law([0.; 3]).unwrap();
        let mut spent = 0;
        let map = arc::ArcMap::build(&path, &mut spent, 100000)
            .unwrap()
            .unwrap();
        for station in [0.125_f64, 0.5, 0.875] {
            let range = [station - 0.0001, station + 0.0001];
            let phase = map.phase(range, &mut spent, 100000).unwrap().unwrap();
            let t = I::new(phase.traversal[0], phase.traversal[1]).unwrap();
            let alpha = t.add(t.mul(t).unwrap()).unwrap().div(I::point(4.)).unwrap();
            let trig = crate::sweeps::progressive_miter::trigonometric_certificate::certify([
                alpha.lo, alpha.hi,
            ])
            .unwrap();
            let raw = frame::certify_arc_corrected_bishop_jets(
                &path,
                [trig.cos, trig.sin, [0., 0.]],
                &zero,
                phase.traversal,
                10000,
                [0.5, 0.5],
                [2., 2.],
            )
            .unwrap();
            let composed = frame::compose_and_twist(raw, phase, &zero, range, 10000).unwrap();
            assert_eq!(composed.status, Status::Certified);
            let out = frame::certify_bishop_phase_jets(
                &path,
                [trig.cos, trig.sin, [0., 0.]],
                &zero,
                range,
                phase,
                10000,
                Some(frame::BishopCorrection::OriginalArc {
                    angle: I::point(0.5),
                }),
            )
            .unwrap();
            assert_eq!(out.status, Status::Certified);
            let angle = 0.5 * station;
            let n = out.transverse.unwrap();
            for (ranges, values) in [n.value, n.first, n.second].into_iter().zip([
                [angle.cos(), angle.sin(), 0.],
                [-0.5 * angle.sin(), 0.5 * angle.cos(), 0.],
                [-0.25 * angle.cos(), -0.25 * angle.sin(), 0.],
            ]) {
                for k in 0..3 {
                    assert!(
                        ranges[k][0] <= values[k] && values[k] <= ranges[k][1],
                        "{ranges:?} vs {values:?}"
                    );
                    assert!(ranges[k][1] - ranges[k][0] < 0.01);
                }
            }
        }
    }
}
