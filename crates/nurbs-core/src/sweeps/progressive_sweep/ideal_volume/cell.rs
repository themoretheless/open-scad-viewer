//! One reusable original-volume differential enclosure. Failed trials and
//! discarded subdivision proposals spend the SAME owner ledger.
use super::*;
use std::{cell::RefCell, collections::BTreeMap};
#[derive(Clone, Copy)]
pub(super) enum Bound {
    Lower(f64),
    Unavailable,
    Unsupported(&'static str),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Enclosure {
    pub(crate) value: V3,
    pub(crate) columns: [V3; 3],
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Enclosed {
    Value(Enclosure),
    Unavailable(Option<usize>),
    Unsupported(&'static str),
}
pub(super) struct Context<'a, 'b> {
    pub(super) s: &'b Sweep<'a>,
    pub(super) caps: [&'b Surface; 2],
    pub(super) cap_identity: Option<cap_identity::Identity<'b>>,
    pub(super) cap_difference: Option<cap_difference::Difference<'b>>,
    pub(super) basis: [V3; 3],
    pub(super) start: V3,
    pub(super) zero: Curve,
    pub(super) offset: Option<Curve>,
    pub(super) anchor: Option<[f64; 2]>,
    pub(super) transport: Option<OriginalRmfTransportReport>,
    pub(super) arc: Option<arc::ArcMap>,
    pub(super) guide_arc: Option<arc::ArcMap>,
    pub(super) shared_guide: bool,
    // Allocated only for ONE atlas certificate and its ONE aggregate ledger.
    // No source proof or cache credit escapes into another call/budget.
    pub(super) poses: Option<RefCell<BTreeMap<[u64; 2], pose::PairedPose>>>,
    pub(super) cap_jets: Option<RefCell<BTreeMap<(usize, [[u64; 2]; 2]), [V3; 3]>>>,
}
impl Context<'_, '_> {
    pub(super) fn same_cap(&self) -> bool {
        self.cap_identity.as_ref().is_some_and(|p| p.matches(self.caps))
    }
    fn cached_cap_jets(
        &self,
        index: usize,
        uv: [[f64; 2]; 2],
        out: &mut IdealVolumeReport,
        max: usize,
    ) -> Result<Option<[V3; 3]>> {
        let key = (index, uv.map(|d| d.map(f64::to_bits)));
        if let Some(cache) = &self.cap_jets {
            if let Some(jets) = cache.borrow().get(&key).copied() {
                return Ok(Some(jets));
            }
        }
        let jets = cap_jets(self.caps[index], uv, &mut out.cells, max)?;
        if let (Some(cache), Some(jets)) = (&self.cap_jets, jets) {
            cache.borrow_mut().insert(key, jets);
        }
        Ok(jets)
    }
    pub(super) fn bound(
        &self,
        b: Box3,
        projection: [[f64; 3]; 3],
        out: &mut IdealVolumeReport,
        max_cells: usize,
    ) -> Result<Bound> {
        Ok(match self.enclose(b, out, max_cells)? {
            Enclosed::Value(e) => Bound::Lower(margin(e.columns, projection)?),
            Enclosed::Unavailable(_) => Bound::Unavailable,
            Enclosed::Unsupported(reason) => Bound::Unsupported(reason),
        })
    }
    pub(super) fn enclose(
        &self,
        b: Box3,
        out: &mut IdealVolumeReport,
        max_cells: usize,
    ) -> Result<Enclosed> {
        // A larger aggregate atlas ledger never widens primitive query limits.
        let max_cells = max_cells.min(out.cells.saturating_add(100000));
        let caps = self.caps;
        let s = self.s;
        let basis = self.basis;
        let start = self.start;
        let zero = &self.zero;
        let anchor = self.anchor;
        if out.cells == max_cells {
            return Ok(Enclosed::Unavailable(None));
        }
        out.cells += 1;
        out.boxes += 1;
        let Some(a) = self.cached_cap_jets(0, [b[0], b[1]], out, max_cells)? else {
            return Ok(Enclosed::Unavailable(None));
        };
        let z = if self.same_cap() {
            a
        } else {
            let Some(z) = self.cached_cap_jets(1, [b[0], b[1]], out, max_cells)? else {
                return Ok(Enclosed::Unavailable(None));
            };
            z
        };
        let mut difference = [[I::point(0.);3];3];
        if !self.same_cap() {
            for j in 0..3 { difference[j] = iv::sub(z[j],a[j])?; }
            if let Some(owner) = &self.cap_difference {
                if let Some(bound) = owner.enclose(caps,a,[b[0],b[1]],&mut out.cells,max_cells)? {
                    for j in 0..3 {for k in 0..3 {
                        difference[j][k] = difference[j][k].intersect(bound[j][k].lo,bound[j][k].hi)?;
                    }}
                }
            }
        }
        let t = I::new(b[2][0], b[2][1])?;
        let one = I::point(1.).sub(t)?;
        let mut q = [[I::point(0.); 3]; 3];
        let mut world_q = q;
        for j in 0..3 {
            let blended = if self.same_cap() { a[j] } else {
                let former = iv::add(iv::scale(a[j],one)?,iv::scale(z[j],t)?)?;
                let correlated = iv::add(a[j],iv::scale(difference[j],t)?)?;
                let mut result = former;
                for k in 0..3 {result[k]=former[k].intersect(correlated[k].lo,correlated[k].hi)?;}
                result
            };
            world_q[j] = blended;
            q[j] = local(blended,basis)?;
        }
        q[0] = iv::sub(q[0],local(start,basis)?)?;
        world_q[0] = iv::sub(world_q[0],start)?;
        let world_qt = difference[0];
        let qt = if self.same_cap() { [I::point(0.);3] } else { local(world_qt,basis)? };
        if let Some(cache) = &self.poses {
            if let Some(pose) = cache.borrow().get(&b[2].map(f64::to_bits)) {
                return Ok(Enclosed::Value(pose.enclose(q, qt, world_q, world_qt)?));
            }
        }
        let phase = if let Some(map) = &self.arc {
            let Some(p) = map.phase(b[2], &mut out.cells, max_cells)? else {
                return Ok(Enclosed::Unavailable(Some(2)));
            };
            p
        } else {
            frame::PathPhase::identity(b[2])
        };
        let guide_phase = if s.orientation_guide.is_some() {
            if let Some(map) = &self.guide_arc {
                let Some(g) = map.phase(b[2], &mut out.cells, max_cells)? else {
                    return Ok(Enclosed::Unavailable(Some(2)));
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
                return Ok(Enclosed::Unavailable(None));
            }
            let value = fit.value.unwrap();
            if value[0] <= 0. {
                out.reason = Some("ideal-volume-contact-positivity-unproved");
                return Ok(Enclosed::Unavailable(None));
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
            return Ok(Enclosed::Unsupported(
                "ideal-volume-original-frame-jets-unproved",
            ));
        };
        if f.status != Status::Certified {
            out.cells += f.cells;
            return Ok(Enclosed::Unavailable(Some(2)));
        }
        out.cells += f.cells;
        f.cells = 0;
        if let Some(cache) = &self.poses {
            let origin = frame::control_trajectories_with_phase(
                s.path,
                s.scale,
                s.affine_laws,
                &[[[0., 0.]; 3]],
                b[2],
                max_cells - out.cells,
                f.clone(),
                fit,
                phase,
            )?;
            out.cells += origin.cells;
            let Some(origin) = origin.jets else {
                return Ok(Enclosed::Unavailable(None));
            };
            let axes = std::array::from_fn::<_, 3, _>(|j| {
                std::array::from_fn::<_, 3, _>(|i| {
                    let x = if i == j { 1. } else { 0. };
                    [x, x]
                })
            });
            let linear = frame::control_trajectories_with_fit(
                zero,
                s.scale,
                s.affine_laws.map(|(a, _)| (a, zero)),
                &axes,
                b[2],
                max_cells - out.cells,
                f,
                fit,
            )?;
            out.cells += linear.cells;
            let Some(linear) = linear.jets else {
                return Ok(Enclosed::Unavailable(None));
            };
            let pose = pose::Pose {
                origin: [decode(origin[0].value)?, decode(origin[0].first)?],
                linear: [
                    [decode(linear[0].value)?, decode(linear[0].first)?],
                    [decode(linear[1].value)?, decode(linear[1].first)?],
                    [decode(linear[2].value)?, decode(linear[2].first)?],
                ],
            };
            let pose = pose::PairedPose::new(pose, basis)?;
            let e = pose.enclose(q, qt, world_q, world_qt)?;
            cache.borrow_mut().insert(b[2].map(f64::to_bits), pose);
            return Ok(Enclosed::Value(e));
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
            return Ok(Enclosed::Unavailable(None));
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
            return Ok(Enclosed::Unavailable(None));
        };
        let columns = [
            decode(linear[0].value)?,
            decode(linear[1].value)?,
            iv::add(decode(actual[0].first)?, decode(linear[2].value)?)?,
        ];
        Ok(Enclosed::Value(Enclosure {
            value: decode(actual[0].value)?,
            columns,
        }))
    }
}
