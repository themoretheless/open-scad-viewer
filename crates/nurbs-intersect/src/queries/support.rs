/// Outward binary64 interval arithmetic is used only for sign exclusion on the
/// current homogeneous Bernstein data. It does not cover preceding knot edits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Interval {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl Interval {
    pub(crate) fn whole() -> Self {
        Self {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
    }
    pub(crate) fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    pub(crate) fn add(self, b: Self) -> Self {
        if self.lo == 0. && self.hi == 0. {
            return b;
        }
        if b.lo == 0. && b.hi == 0. {
            return self;
        }
        if self.lo == self.hi && b.lo == b.hi && self.lo.is_finite() && self.lo == -b.lo {
            return Self::exact(0.);
        }
        if (self.lo + b.lo).is_nan() || (self.hi + b.hi).is_nan() {
            return Self::whole();
        }
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    pub(crate) fn mul(self, b: Self) -> Self {
        if self.lo == 1. && self.hi == 1. {
            return b;
        }
        if b.lo == 1. && b.hi == 1. {
            return self;
        }
        if self.lo == -1. && self.hi == -1. {
            return Self {
                lo: -b.hi,
                hi: -b.lo,
            };
        }
        if b.lo == -1. && b.hi == -1. {
            return Self {
                lo: -self.hi,
                hi: -self.lo,
            };
        }
        if (self.lo == 0. && self.hi == 0. && b.lo.is_finite() && b.hi.is_finite())
            || (b.lo == 0. && b.hi == 0. && self.lo.is_finite() && self.hi.is_finite())
        {
            return Self::exact(0.);
        }
        let values = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if values.iter().any(|v| v.is_nan()) {
            return Self::whole();
        }
        Self {
            lo: values.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: values
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
    pub(crate) fn sign(self) -> i8 {
        if self.lo > 0. {
            1
        } else if self.hi < 0. {
            -1
        } else {
            0
        }
    }
}

pub(crate) fn spans(knots: &[f64], degree: usize, count: usize) -> Vec<[f64; 2]> {
    knots[degree..=count]
        .windows(2)
        .filter_map(|s| (s[0] < s[1]).then_some([s[0], s[1]]))
        .collect()
}

/// Half-open box ownership shared by every subdivision query in this file:
/// each box owns its interior and its lower-parameter face; the domain's
/// upper end is owned by the last box. Exactly one box therefore owns any
/// parameter, so a root sitting bitwise on a shared subdivision face is
/// reported once by its owning box and halo boxes stay silent.
pub(crate) fn owns_parameter(lo: f64, hi: f64, domain_hi: f64, x: f64) -> bool {
    lo <= x && (x < hi || (x == hi && hi == domain_hi))
}
/// Snap a converged parameter onto a box face when it rounds within two
/// binary64 steps of it. Adjacent boxes share faces bitwise, so Newton images
/// that disagree by one rounding step resolve to the same owned face
/// parameter. This reconciles rounding at a shared face only — it is not a
/// spatial tolerance and never merges distinct parameters.
pub(crate) fn snap_to_face(x: f64, face: f64) -> f64 {
    let near = if x < face {
        [face.next_down(), face.next_down().next_down()]
    } else {
        [face.next_up(), face.next_up().next_up()]
    };
    if x == face || near.contains(&x) {
        face
    } else {
        x
    }
}
