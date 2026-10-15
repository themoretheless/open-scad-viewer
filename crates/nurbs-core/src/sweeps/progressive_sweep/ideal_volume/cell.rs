//! One reusable original-volume differential enclosure. Failed trials and
//! discarded subdivision proposals spend the SAME owner ledger.
use super::*;
#[derive(Clone, Copy)]
pub(super) enum Bound {
    Lower(f64),
    Unavailable,
    Unsupported(&'static str),
}
pub(super) struct Context<'a, 'b> {
    pub(super) s: &'b Sweep<'a>,
    pub(super) caps: [&'b Surface; 2],
    pub(super) basis: [V3; 3],
    pub(super) start: V3,
    pub(super) zero: Curve,
    pub(super) offset: Option<Curve>,
    pub(super) anchor: Option<[f64; 2]>,
    pub(super) transport: Option<OriginalRmfTransportReport>,
    pub(super) arc: Option<arc::ArcMap>,
    pub(super) guide_arc: Option<arc::ArcMap>,
    pub(super) shared_guide: bool,
}
impl Context<'_, '_> {
    pub(super) fn bound(
        &self,
        b: Box3,
        projection: [[f64; 3]; 3],
        out: &mut IdealVolumeReport,
        max_cells: usize,
    ) -> Result<Bound> {
        let caps = self.caps;
        let s = self.s;
        let basis = self.basis;
        let start = self.start;
        let zero = &self.zero;
        let anchor = self.anchor;
        if out.cells == max_cells {
            return Ok(Bound::Unavailable);
        }
        out.cells += 1;
        out.boxes += 1;
        let Some(a) = cap_jets(caps[0], [b[0], b[1]], &mut out.cells, max_cells)? else {
            return Ok(Bound::Unavailable);
        };
        let z = if caps[0] == caps[1] {
            a
        } else {
            let Some(z) = cap_jets(caps[1], [b[0], b[1]], &mut out.cells, max_cells)? else {
                return Ok(Bound::Unavailable);
            };
            z
        };
        let t = I::new(b[2][0], b[2][1])?;
        let one = I::point(1.).sub(t)?;
        let mut q = [[I::point(0.); 3]; 3];
        for j in 0..3 {
            // Literal source identity preserves correlation. Independent
            // boxes for (1-t)C+tC or C-C would invent a moving carrier.
            let blended = if caps[0] == caps[1] {
                a[j]
            } else {
                iv::add(iv::scale(a[j], one)?, iv::scale(z[j], t)?)?
            };
            q[j] = local(blended, basis)?;
        }
        // q[0] is an affine position, q[1..] are linear differential vectors.
        q[0] = iv::sub(q[0], local(start, basis)?)?;
        let qt = if caps[0] == caps[1] {
            [I::point(0.); 3]
        } else {
            local(iv::sub(z[0], a[0])?, basis)?
        };
        let phase = if let Some(map) = &self.arc {
            let Some(p) = map.phase(b[2], &mut out.cells, max_cells)? else {
                return Ok(Bound::Unavailable);
            };
            p
        } else {
            frame::PathPhase::identity(b[2])
        };
        let guide_phase = if s.orientation_guide.is_some() {
            if let Some(map) = &self.guide_arc {
                let Some(g) = map.phase(b[2], &mut out.cells, max_cells)? else {
                    return Ok(Bound::Unavailable);
                };
                Some(g)
            } else {
                Some(phase)
            }
        } else {
            None
        };
        let fit = if let Some(anchor) = anchor {
            let fit = if self.arc.is_some() {
                let width = if self.shared_guide && self.offset.is_some() {
                    let raw = frame::certify_guide_width(
                        zero,
                        self.offset.as_ref().unwrap(),
                        phase.traversal,
                        max_cells - out.cells,
                    )?;
                    arc_frames::compose_width(raw, phase)?
                } else {
                    frame::certify_phase_guide_width(
                        s.path,
                        s.orientation_guide.unwrap(),
                        phase,
                        guide_phase.unwrap(),
                        max_cells - out.cells,
                    )?
                };
                frame::contact_fit_from_width(
                    width,
                    s.scale,
                    s.affine_laws,
                    anchor,
                    b[2],
                    max_cells - out.cells,
                )?
            } else if let Some(offset) = &self.offset {
                frame::certify_contact_fit(
                    zero,
                    offset,
                    s.scale,
                    s.affine_laws,
                    anchor,
                    b[2],
                    max_cells - out.cells,
                )?
            } else {
                frame::certify_contact_fit(
                    s.path,
                    s.orientation_guide.unwrap(),
                    s.scale,
                    s.affine_laws,
                    anchor,
                    b[2],
                    max_cells - out.cells,
                )?
            };
            out.cells += fit.cells;
            if fit.status != Status::Certified {
                return Ok(Bound::Unavailable);
            }
            let value = fit.value.unwrap();
            if value[0] <= 0. {
                out.reason = Some("ideal-volume-contact-positivity-unproved");
                return Ok(Bound::Unavailable);
            }
            Some([
                I::new(value[0], value[1])?,
                I::new(fit.first.unwrap()[0], fit.first.unwrap()[1])?,
                I::new(fit.second.unwrap()[0], fit.second.unwrap()[1])?,
            ])
        } else {
            None
        };
        let frame = if self.arc.is_some() {
            arc_frames::arc_frame(
                s,
                b[2],
                phase,
                guide_phase,
                self.offset.as_ref(),
                self.transport.as_ref(),
                self.shared_guide,
                zero,
                max_cells - out.cells,
            )?
        } else {
            original_jets(
                s,
                b[2],
                max_cells - out.cells,
                self.offset.as_ref(),
                self.transport.as_ref(),
            )?
        };
        let Some(mut f) = frame else {
            return Ok(Bound::Unsupported(
                "ideal-volume-original-frame-jets-unproved",
            ));
        };
        if f.status != Status::Certified {
            out.cells += f.cells;
            return Ok(Bound::Unavailable);
        }
        let actual = frame::control_trajectories_with_phase(
            s.path,
            s.scale,
            s.affine_laws,
            &[encode(q[0])],
            b[2],
            max_cells - out.cells,
            f.clone(),
            fit,
            phase,
        )?;
        out.cells += actual.cells;
        let Some(actual) = actual.jets else {
            return Ok(Bound::Unavailable);
        };
        // Reuse the SAME original frame enclosure without charging its
        // source work twice. UV derivatives have no positional translation
        // or center, so do not subtract independent wide position boxes.
        f.cells = 0;
        let affine = s.affine_laws.map(|(a, _)| (a, zero));
        let linear = frame::control_trajectories_with_fit(
            zero,
            s.scale,
            affine,
            &[encode(q[1]), encode(q[2]), encode(qt)],
            b[2],
            max_cells - out.cells,
            f,
            fit,
        )?;
        out.cells += linear.cells;
        let Some(linear) = linear.jets else {
            return Ok(Bound::Unavailable);
        };
        let columns = [
            decode(linear[0].value)?,
            decode(linear[1].value)?,
            iv::add(decode(actual[0].first)?, decode(linear[2].value)?)?,
        ];
        Ok(Bound::Lower(margin(columns, projection)?))
    }
}
