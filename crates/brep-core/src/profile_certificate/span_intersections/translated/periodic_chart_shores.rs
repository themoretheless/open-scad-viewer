//! Exact native side layout. This is not material classification or face sewing.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio, ratio};
use std::{cmp::Ordering, collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicNativeShoreRole {
    NaturalSupport,
    AuxiliarySeam,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicNativeShorePoint {
    Corner(usize),
    /// Transport is authorized by the ACTUAL Surface seam identity, not by
    /// creating an authored curve use at the opposite canonical shore.
    SourceCopy {
        copy: usize,
        transported: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNativeShore {
    pub fixed_axis: usize,
    pub end: usize,
    pub role: OriginalPeriodicNativeShoreRole,
    /// CCW boundary traversal relative to INCREASING native varying coordinate.
    pub direction: i8,
    pub points: Vec<OriginalPeriodicNativeShorePoint>,
    pub intervals: Vec<[usize; 2]>,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicNativeShoreLimits {
    pub max_endpoint_uses: usize,
    pub max_intervals: usize,
    /// Each ordering/point admission pass has this cap; owner replay is separate.
    pub max_comparisons: usize,
}
impl Default for PeriodicNativeShoreLimits {
    fn default() -> Self {
        Self {
            max_endpoint_uses: 100000,
            max_intervals: 100000,
            max_comparisons: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    shores: Vec<OriginalPeriodicNativeShore>,
    endpoint_uses: usize,
    intervals: usize,
    comparisons: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicNativeChartShores {
    owner: Arc<OwnedPeriodicNativeChartCorners>,
    parameter_limits: ParameterOrderLimits,
    limits: PeriodicNativeShoreLimits,
    data: Data,
}
struct Scalar {
    f: Vec<Q>,
    root: [Q; 2],
    n: Vec<Q>,
    d: Vec<Q>,
}
struct Compare {
    limits: ParameterOrderLimits,
    cap: usize,
    work: usize,
}
impl Compare {
    fn compare(&mut self, a: &Scalar, b: &Scalar) -> Result<Ordering> {
        self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| refused("Native shore root comparison allowance exhausted"))?;
        arithmetic::parameter_order::compare_rational(
            &a.f,
            [&a.n, &a.d],
            &a.root,
            &b.f,
            [&b.n, &b.d],
            &b.root,
            self.limits.max_refinements,
            self.limits.max_resultant_size,
        )
    }
}
fn support(corners: &OwnedPeriodicNativeChartCorners) -> &Arc<OwnedPeriodicSupportBoundaries> {
    let copies = corners.owner();
    let incidence = copies.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    lifts.owner().owner().rule().support()
}



impl OwnedPeriodicNativeChartShores {
    pub fn owner(&self) -> &Arc<OwnedPeriodicNativeChartCorners> {
        &self.owner
    }
    pub fn shores(&self) -> &[OriginalPeriodicNativeShore] {
        &self.data.shores
    }
    pub fn interval_count(&self) -> usize {
        self.data.intervals
    }
    pub fn comparisons(&self) -> usize {
        self.data.comparisons
    }
    // Exact whole-interval sewing: identical native varying coordinates on
    // actual opposite auxiliary shores of the SAME admitted source Surface.
    pub(super) fn matching_opposite_interval(&self, a: [usize; 2], b: [usize; 2]) -> Result<bool> {
        let first = self
            .shores()
            .get(a[0])
            .ok_or_else(|| refused("Unknown sewing shore"))?;
        let second = self
            .shores()
            .get(b[0])
            .ok_or_else(|| refused("Unknown opposite sewing shore"))?;
        if first.fixed_axis != second.fixed_axis
            || first.end == second.end
            || first.role != OriginalPeriodicNativeShoreRole::AuxiliarySeam
            || second.role != OriginalPeriodicNativeShoreRole::AuxiliarySeam
        {
            return Ok(false);
        }
        let supporting = support(&self.owner).augmented().surface();
        if supporting.seam_proofs()[first.fixed_axis].is_none() {
            return Err(refused(
                "Sewing lacks actual original Surface seam identity",
            ));
        }
        let ai = first
            .intervals
            .get(a[1])
            .ok_or_else(|| refused("Unknown sewing interval"))?;
        let bi = second
            .intervals
            .get(b[1])
            .ok_or_else(|| refused("Unknown opposite sewing interval"))?;
        let mut cmp = Compare {
            limits: self.parameter_limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        for end in 0..2 {
            if cmp.compare(
                &scalar(&self.owner, first.points[ai[end]], 1 - first.fixed_axis)?,
                &scalar(&self.owner, second.points[bi[end]], 1 - second.fixed_axis)?,
            )? != Ordering::Equal
            {
                return Ok(false);
            }
        }
        if first.direction != -second.direction {
            return Err(refused(
                "Sewn original native edges lack opposite orientation",
            ));
        }
        Ok(true)
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Native shore source,ordering,transport or role changed",
            ));
        }
        Ok(())
    }
    // Chart-plane ray contribution; wrapped coordinates are NOT normalized.
    // The enclosing immutable batch has already replayed this actual owner.
    pub(super) fn ray_contribution(
        &self,
        shore: usize,
        interval: usize,
        point: &[Q; 2],
        limits: ParameterOrderLimits,
    ) -> Result<Option<i64>> {
        if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
            return Err(refused("Invalid native frame ray allowance"));
        }
        let s = self
            .data
            .shores
            .get(shore)
            .ok_or_else(|| refused("Unknown native ray shore"))?;
        let at = s
            .intervals
            .get(interval)
            .ok_or_else(|| refused("Unknown native ray interval"))?;
        if s.direction != 1 && s.direction != -1 {
            return Err(refused("Invalid native ray direction"));
        }
        let chart = support(&self.owner).augmented().surface().pieces().chart();
        let fixed = Q::from_float(chart[s.fixed_axis][s.end]).unwrap();
        if (s.fixed_axis == 0 && point[0] > fixed) || (s.fixed_axis == 1 && point[1] != fixed) {
            return Ok(Some(0));
        }
        let q = Scalar {
            f: vec![Q::zero(), Q::one()],
            root: [Q::zero(), Q::zero()],
            n: vec![point[1 - s.fixed_axis].clone()],
            d: vec![Q::one()],
        };
        let mut cmp = Compare {
            limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        let lower = cmp.compare(&q, &scalar(&self.owner, s.points[at[0]], 1 - s.fixed_axis)?)?;
        let upper = cmp.compare(&q, &scalar(&self.owner, s.points[at[1]], 1 - s.fixed_axis)?)?;
        let closed = lower != Ordering::Less && upper != Ordering::Greater;
        if s.fixed_axis == 1 {
            return Ok(if closed { None } else { Some(0) });
        }
        if point[0] == fixed && closed {
            return Ok(None);
        }
        Ok(Some(
            if lower != Ordering::Less && upper == Ordering::Less {
                i64::from(s.direction)
            } else {
                0
            },
        ))
    }
    // Freshly replayed enclosing owners use this to check the COMPLETE contact
    // census. Outside values are clipped by the actual native side endpoints.
    pub(super) fn admit_cut(
        &self,
        shore: usize,
        primitive: &[Q],
        root: &[Q; 2],
        position: &[Vec<Q>; 2],
    ) -> Result<Option<usize>> {
        let s = self
            .data
            .shores
            .get(shore)
            .ok_or_else(|| refused("Unknown native shore contact"))?;
        let q = Scalar {
            f: primitive.to_vec(),
            root: root.clone(),
            n: position[0].clone(),
            d: position[1].clone(),
        };
        let mut cmp = Compare {
            limits: self.parameter_limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        let first = scalar(&self.owner, s.points[0], 1 - s.fixed_axis)?;
        let last = scalar(
            &self.owner,
            *s.points
                .last()
                .ok_or_else(|| refused("Empty native shore contact frame"))?,
            1 - s.fixed_axis,
        )?;
        if cmp.compare(&q, &first)? == Ordering::Less
            || cmp.compare(&q, &last)? == Ordering::Greater
        {
            return Ok(None);
        }
        let (mut lo, mut hi) = (0, s.points.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match cmp.compare(&q, &scalar(&self.owner, s.points[mid], 1 - s.fixed_axis)?)? {
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
                Ordering::Equal => return Ok(Some(mid)),
            }
        }
        Err(refused(
            "Original source contact lies strictly inside an unsplit native shore interval",
        ))
    }
    // The enclosing batch must freshly replay this immutable owner first.
    pub(super) fn interior_contains(
        &self,
        shore: usize,
        interval: usize,
        value: &Q,
    ) -> Result<bool> {
        let s = self
            .data
            .shores
            .get(shore)
            .ok_or_else(|| refused("Unknown native shore sample"))?;
        let at = s
            .intervals
            .get(interval)
            .ok_or_else(|| refused("Unknown native shore sample interval"))?;
        let q = Scalar {
            f: vec![Q::zero(), Q::one()],
            root: [Q::zero(), Q::zero()],
            n: vec![value.clone()],
            d: vec![Q::one()],
        };
        let mut cmp = Compare {
            limits: self.parameter_limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        Ok(
            cmp.compare(&q, &scalar(&self.owner, s.points[at[0]], 1 - s.fixed_axis)?)?
                == Ordering::Greater
                && cmp.compare(&q, &scalar(&self.owner, s.points[at[1]], 1 - s.fixed_axis)?)?
                    == Ordering::Less,
        )
    }
    pub(super) fn interior_uv(
        &self,
        shore: usize,
        interval: usize,
        steps: usize,
    ) -> Result<[Q; 2]> {
        if !(1..=512).contains(&steps) {
            return Err(refused("Invalid strict shore witness allowance"));
        }
        let s = self
            .data
            .shores
            .get(shore)
            .ok_or_else(|| refused("Unknown native shore sample"))?;
        let at = s
            .intervals
            .get(interval)
            .ok_or_else(|| refused("Unknown native shore sample interval"))?;
        let chart = support(&self.owner)
            .augmented()
            .surface()
            .pieces()
            .chart()
            .map(|d| d.map(|v| Q::from_float(v).unwrap()));
        let mut range = chart[1 - s.fixed_axis].clone();
        let lower = scalar(&self.owner, s.points[at[0]], 1 - s.fixed_axis)?;
        let upper = scalar(&self.owner, s.points[at[1]], 1 - s.fixed_axis)?;
        let mut cmp = Compare {
            limits: self.parameter_limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        for _ in 0..steps {
            let value = (&range[0] + &range[1]) / Q::from_integer(2.into());
            let q = Scalar {
                f: vec![Q::zero(), Q::one()],
                root: [Q::zero(), Q::zero()],
                n: vec![value.clone()],
                d: vec![Q::one()],
            };
            if cmp.compare(&q, &lower)? != Ordering::Greater {
                range[0] = value;
            } else if cmp.compare(&q, &upper)? != Ordering::Less {
                range[1] = value;
            } else {
                let mut uv = [value.clone(), value];
                uv[s.fixed_axis] = chart[s.fixed_axis][s.end].clone();
                return Ok(uv);
            }
        }
        Err(refused(
            "Strict native shore witness is unresolved within allowance",
        ))
    }
    /// Exact world geometry of a native frame interval, independent of whether
    /// material occupies its chart-inside side. The supplied clock is the
    /// ORIGINAL Surface varying coordinate, not a normalized/fitted parameter.
    pub fn point(
        &self,
        shore: usize,
        interval: usize,
        native: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        self.reverify_source()?;
        let s = self
            .data
            .shores
            .get(shore)
            .ok_or_else(|| refused("Unknown actual native shore"))?;
        let at = s
            .intervals
            .get(interval)
            .ok_or_else(|| refused("Unknown actual native shore interval"))?;
        let t = decode_ratio(native)?;
        let q = Scalar {
            f: vec![Q::zero(), Q::one()],
            root: [Q::zero(), Q::zero()],
            n: vec![t.clone()],
            d: vec![Q::one()],
        };
        let mut cmp = Compare {
            limits: self.parameter_limits,
            cap: self.limits.max_comparisons,
            work: 0,
        };
        if cmp.compare(&q, &scalar(&self.owner, s.points[at[0]], 1 - s.fixed_axis)?)?
            == Ordering::Less
            || cmp.compare(&q, &scalar(&self.owner, s.points[at[1]], 1 - s.fixed_axis)?)?
                == Ordering::Greater
        {
            return Err(refused(
                "Native frame query escaped its actual algebraic endpoint cuts",
            ));
        }
        let p = support(&self.owner);
        let chart = p.augmented().surface().pieces().chart();
        let mut uv = [t.clone(), t];
        uv[s.fixed_axis] = Q::from_float(chart[s.fixed_axis][s.end]).unwrap();
        p.world_uv_point(&uv.map(|q| ratio(&q)))
    }
}
pub fn own_periodic_native_chart_shores(
    owner: Arc<OwnedPeriodicNativeChartCorners>,
    parameter_limits: ParameterOrderLimits,
    limits: PeriodicNativeShoreLimits,
) -> Result<OwnedPeriodicNativeChartShores> {
    let mut o = OwnedPeriodicNativeChartShores {
        owner,
        parameter_limits,
        limits,
        data: Data {
            shores: Vec::new(),
            endpoint_uses: 0,
            intervals: 0,
            comparisons: 0,
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
#[path = "periodic_chart_shores/tests/tests_fixtures.rs"]
pub(super) mod tests ;

#[path = "periodic_chart_shores/shore_derivation.rs"]
mod shore_derivation;
use shore_derivation::derive;
use shore_derivation::scalar;
use shore_derivation::order;
