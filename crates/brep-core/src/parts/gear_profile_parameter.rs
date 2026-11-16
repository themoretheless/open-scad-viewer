//! Full nominal parameter correspondence for saved gear arcs and radial lines.
use super::*;
use nurbs_core::{interval_eval::Interval as I, involute::GearParameterBounds};
fn interval(p: [f64; 2]) -> Result<I> {
    I::new(p[0], p[1])
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
/// Curves per tooth with and without radial lines below the base circle.
const ITEMS_WITH_RADIAL_LINES: usize = 6;
const ITEMS_WITHOUT_RADIAL_LINES: usize = 4;
/// Weight deviation below which a radial-line transition is polynomial.
const UNIT_WEIGHT_EPS: f64 = 1e-15;

/// Nominal tooth parameters shared by every transition check.
struct Transitions {
    internal: bool,
    radial_lines: bool,
    backlash: f64,
    count: usize,
    base: I,
    low: I,
    high: [f64; 2],
    step: I,
    low_half: I,
    high_half: I,
}

impl Transitions {
    fn new(spec: &GearSpec, bounds: &GearParameterBounds, radial_lines: bool) -> Result<Self> {
        let base = interval(bounds.base_radius)?;
        let low = interval(if spec.internal {
            bounds.tip_radius
        } else {
            bounds.root_radius
        })?;
        let high = if spec.internal {
            bounds.root_radius
        } else {
            bounds.tip_radius
        };
        if (radial_lines && base.lo <= low.hi) || (!radial_lines && low.lo < base.hi) {
            return Err(invalid("Nominal root/base branch is not separated"));
        }
        let step = interval(bounds.tooth_step)?;
        let low_half = interval(bounds.half_low_angle)?;
        let high_half = interval(bounds.half_high_angle)?;
        if high_half.lo <= 0.
            || low_half.hi >= step.div(I::point(2.))?.lo
            || low_half.lo <= high_half.hi
        {
            return Err(invalid("Nominal gear angular ordering is not separated"));
        }
        Ok(Self {
            internal: spec.internal,
            radial_lines,
            backlash: spec.backlash,
            count: if radial_lines {
                ITEMS_WITH_RADIAL_LINES
            } else {
                ITEMS_WITHOUT_RADIAL_LINES
            },
            base,
            low,
            high,
            step,
            low_half,
            high_half,
        })
    }

    fn orient<T>(&self, from: T, to: T) -> (T, T) {
        if self.internal {
            (to, from)
        } else {
            (from, to)
        }
    }

    /// Error of tooth-wire curve `ci`, or `None` when it is not certified here.
    fn tooth_curve(&self, c: &Curve, ci: usize, wire_len: usize) -> Result<Option<f64>> {
        let original = if self.internal { wire_len - 1 - ci } else { ci };
        let tooth = original / self.count;
        let item = original % self.count;
        let center = self.step.mul(I::point(
            tooth as f64 + if self.internal { 0.5 } else { 0. },
        ))?;
        if c.degree == 1 {
            self.radial_line(c, item, center).map(Some)
        } else {
            self.tooth_arc(c, item, center)
        }
    }

    fn radial_line(&self, c: &Curve, item: usize, center: I) -> Result<f64> {
        let (from, to, sign) = match item {
            0 if self.radial_lines => (pair(self.low), pair(self.base), -1.),
            4 if self.radial_lines => (pair(self.base), pair(self.low), 1.),
            _ => return Err(invalid("Unknown radial transition identity")),
        };
        let angle = center.add(self.low_half.mul(I::point(sign))?)?;
        let (from, to) = self.orient(from, to);
        Ok(nurbs_core::primitives::certify_radial_segment_radians(
            c,
            [0.; 3],
            from,
            to,
            pair(angle),
        )?
        .error_upper)
    }

    fn tooth_arc(&self, c: &Curve, item: usize, center: I) -> Result<Option<f64>> {
        let tip_item = if self.radial_lines { 2 } else { 1 };
        let (radius, from, to) = if item == tip_item {
            if self.backlash > 0. {
                return Ok(None);
            }
            (
                self.high,
                center.sub(self.high_half)?,
                center.add(self.high_half)?,
            )
        } else if item == self.count - 1 {
            if self.backlash > 0. {
                return Ok(None);
            }
            (
                pair(self.low),
                center.add(self.low_half)?,
                center.add(self.step)?.sub(self.low_half)?,
            )
        } else if self.radial_lines
            && (item == 0 || item == 4)
            && c.weights.iter().all(|&w| (w - 1.).abs() <= UNIT_WEIGHT_EPS)
        {
            return Ok(None);
        } else {
            return Err(invalid("Unknown tooth arc identity"));
        };
        let (from, to) = self.orient(from, to);
        Ok(Some(
            nurbs_core::primitives::certify_circle_arc_radians(
                c,
                [0.; 3],
                radius,
                pair(from),
                pair(to),
            )?
            .error_upper,
        ))
    }
}

/// Error of curve `ci` of the plain circle wire (rim or bore).
fn circle_wire_curve(
    spec: &GearSpec,
    bounds: &GearParameterBounds,
    step: I,
    c: &Curve,
    ci: usize,
    wire_len: usize,
) -> Result<f64> {
    let radius = if spec.internal {
        interval(bounds.root_radius)?.add(I::point(spec.rim_width))?
    } else {
        I::point(spec.bore).div(I::point(2.))?
    };
    let pi_step = step
        .mul(I::point(spec.teeth as f64))?
        .div(I::point(wire_len as f64))?;
    let index = if spec.internal { ci } else { wire_len - 1 - ci };
    let from = pi_step.mul(I::point(index as f64))?;
    let to = pi_step.mul(I::point((index + 1) as f64))?;
    let (from, to) = if spec.internal {
        (from, to)
    } else {
        (to, from)
    };
    Ok(nurbs_core::primitives::certify_circle_arc_radians(
        c,
        [0.; 3],
        pair(radius),
        pair(from),
        pair(to),
    )?
    .error_upper)
}

pub(super) fn certify(
    spec: &GearSpec,
    wires: &[Vec<Curve>],
    bounds: &GearParameterBounds,
    radial_lines: bool,
) -> Result<f64> {
    let tooth_wire = usize::from(spec.internal);
    let transitions = Transitions::new(spec, bounds, radial_lines)?;
    let mut maximum = 0_f64;
    for (wi, wire) in wires.iter().enumerate() {
        for (ci, c) in wire.iter().enumerate() {
            if c.degree == 5 {
                continue;
            }
            let error = if wi == tooth_wire {
                match transitions.tooth_curve(c, ci, wire.len())? {
                    Some(error) => error,
                    None => continue,
                }
            } else {
                circle_wire_curve(spec, bounds, transitions.step, c, ci, wire.len())?
            };
            maximum = maximum.max(error);
        }
    }
    Ok(maximum)
}
