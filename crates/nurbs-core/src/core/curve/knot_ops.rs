use super::{Curve, Homogeneous, budget, find_span, multiplicity};
use crate::{Result, check, numeric, numeric_err};

impl Curve {
    pub fn insert(&self, u: f64, count: usize) -> Result<Self> {
        self.validate()?;
        check(
            count >= 1 && count <= self.degree + 1,
            "Insertion count must be in [1, degree+1]",
        )?;
        let [a, b] = self.domain();
        check(
            u.is_finite() && u >= a && u <= b,
            "Inserted knot must be inside the active domain",
        )?;
        let base = if self.periodic {
            self.clamp(a, b)?
        } else {
            self.clone()
        };
        let max = if u == a || u == b {
            self.degree + 1
        } else {
            self.degree
        };
        check(
            multiplicity(&base.knots, u) + count <= max,
            "Insertion would exceed the permitted knot multiplicity",
        )?;
        budget(base.control_points.len() + count)?;
        let mut work = Homogeneous::from(&base);
        for _ in 0..count {
            work = work.insert(u)?;
        }
        work.to_curve()
    }
    fn clamp(&self, a: f64, b: f64) -> Result<Self> {
        let mut work = Homogeneous::from(self);
        while multiplicity(&work.knots, a) < self.degree + 1 {
            work = work.insert(a)?;
        }
        while multiplicity(&work.knots, b) < self.degree + 1 {
            work = work.insert(b)?;
        }
        let start = work
            .knots
            .iter()
            .position(|v| *v == a)
            .ok_or_else(|| numeric_err("Clamped start knot is missing after insertion"))?;
        let end = work
            .knots
            .iter()
            .rposition(|v| *v == b)
            .ok_or_else(|| numeric_err("Clamped end knot is missing after insertion"))?;
        let knots = work.knots[start..=end].to_vec();
        let count = knots.len() - self.degree - 1;
        Homogeneous {
            degree: self.degree,
            knots,
            controls: work.controls[start..start + count].to_vec(),
        }
        .to_curve()
    }
    pub fn trim(&self, a: f64, b: f64) -> Result<Self> {
        self.validate()?;
        let domain = self.domain();
        check(
            a.is_finite() && b.is_finite() && domain[0] <= a && a < b && b <= domain[1],
            "Trim must be a nonempty ordered subdomain of the active knot domain",
        )?;
        self.clamp(a, b)
    }
    pub fn split(&self, u: f64) -> Result<[Self; 2]> {
        self.validate()?;
        let [a, b] = self.domain();
        check(
            u.is_finite() && a < u && u < b,
            "Split parameter must lie strictly inside the active domain",
        )?;
        Ok([self.trim(a, u)?, self.trim(u, b)?])
    }
    pub fn reverse(&self) -> Result<Self> {
        self.validate()?;
        let [a, b] = self.domain();
        let mut result = self.clone();
        result.knots = self.knots.iter().rev().map(|k| a + b - k).collect();
        check(
            self.knots
                .windows(2)
                .rev()
                .zip(result.knots.windows(2))
                .all(|(source, reversed)| source[0] == source[1] || reversed[0] < reversed[1]),
            "Reversal collapsed distinct knots at coordinate precision",
        )?;
        result.control_points.reverse();
        result.weights.reverse();
        result.validate()?;
        Ok(result)
    }
    pub fn elevate(&self, degree: usize) -> Result<Self> {
        self.validate()?;
        check(
            degree >= self.degree && degree <= 25,
            "Elevation degree must be between the current degree and 25",
        )?;
        if degree == self.degree {
            return Ok(self.clone());
        }
        let [a, b] = self.domain();
        let mut breaks: Vec<f64> = self
            .knots
            .iter()
            .copied()
            .filter(|k| *k >= a && *k <= b)
            .collect();
        breaks.dedup();
        budget((breaks.len() - 1) * degree + 1)?;
        let segments = self.decompose()?;
        let mut controls = Vec::new();
        let mut knots = vec![a; degree + 1];
        for (index, segment) in segments.iter().enumerate() {
            let mut points = Homogeneous::from(&segment.curve).controls;
            for order in self.degree..degree {
                let mut next = vec![points[0].clone()];
                for i in 1..=order {
                    let alpha = i as f64 / (order + 1) as f64;
                    next.push(
                        points[i]
                            .iter()
                            .enumerate()
                            .map(|(axis, x)| alpha * points[i - 1][axis] + (1. - alpha) * x)
                            .collect(),
                    );
                }
                next.push(points.last().unwrap().clone());
                points = next;
            }
            controls.extend_from_slice(&points[usize::from(index != 0)..]);
            knots.extend(vec![
                segment.domain[1];
                if index == segments.len() - 1 {
                    degree + 1
                } else {
                    degree
                }
            ]);
        }
        Homogeneous {
            degree,
            knots,
            controls,
        }
        .to_curve()
    }
    /// Batch knot refinement per The NURBS Book A5.4: inserts every knot of
    /// the sorted `new_knots` slice in a single pass through the merged knot
    /// vector, in homogeneous coordinates.
    pub fn refine(&self, new_knots: &[f64]) -> Result<Self> {
        self.validate()?;
        if new_knots.is_empty() {
            return Ok(self.clone());
        }
        let p = self.degree;
        let n = self.control_points.len() - 1;
        let [a, b] = self.domain();
        for (i, &x) in new_knots.iter().enumerate() {
            check(
                x.is_finite() && x >= a && x <= b,
                "Refinement knots must lie inside the active domain",
            )?;
            check(
                i == 0 || new_knots[i - 1] <= x,
                "Refinement knots must be sorted nondecreasing",
            )?;
        }
        let mut i = 0;
        while i < new_knots.len() {
            let mut j = i + 1;
            while j < new_knots.len() && new_knots[j] == new_knots[i] {
                j += 1;
            }
            let max = if new_knots[i] == a || new_knots[i] == b {
                p + 1
            } else {
                p
            };
            check(
                multiplicity(&self.knots, new_knots[i]) + (j - i) <= max,
                "Refinement would exceed the permitted knot multiplicity",
            )?;
            i = j;
        }
        budget(self.control_points.len() + new_knots.len())?;
        let r = new_knots.len() - 1;
        let m = n + p + 1;
        let work = Homogeneous::from(self);
        let a = find_span(p, &self.knots, n + 1, new_knots[0])?;
        let b = find_span(p, &self.knots, n + 1, new_knots[r])? + 1;
        let mut controls = vec![Vec::new(); n + r + 2];
        let mut knots = vec![0.; m + r + 2];
        for j in 0..=a - p {
            controls[j] = work.controls[j].clone();
        }
        for j in b - 1..=n {
            controls[j + r + 1] = work.controls[j].clone();
        }
        for j in 0..=a {
            knots[j] = self.knots[j];
        }
        for j in b + p..=m {
            knots[j + r + 1] = self.knots[j];
        }
        let mut i = b + p - 1;
        let mut k = b + p + r;
        for j in (0..=r).rev() {
            while new_knots[j] <= self.knots[i] && i > a {
                controls[k - p - 1] = work.controls[i - p - 1].clone();
                knots[k] = self.knots[i];
                k -= 1;
                i -= 1;
            }
            controls[k - p - 1] = controls[k - p].clone();
            for l in 1..=p {
                let ind = k - p + l;
                let mut alpha = knots[k + l] - new_knots[j];
                if alpha == 0. {
                    controls[ind - 1] = controls[ind].clone();
                } else {
                    alpha /= knots[k + l] - self.knots[i - p + l];
                    let (before, after) = controls.split_at_mut(ind);
                    for axis in 0..before[ind - 1].len() {
                        before[ind - 1][axis] =
                            alpha * before[ind - 1][axis] + (1. - alpha) * after[0][axis];
                    }
                }
            }
            knots[k] = new_knots[j];
            if j > 0 {
                k -= 1;
            }
        }
        Homogeneous {
            degree: p,
            knots,
            controls,
        }
        .to_curve()
    }
}

/// Knot multiplicity under an absolute tolerance, replacing exact `==`
/// comparisons when the knot vector carries rounding noise.
pub fn multiplicity_eps(knots: &[f64], u: f64, eps: f64) -> usize {
    knots.iter().filter(|k| (**k - u).abs() <= eps).count()
}
/// Collapse runs of consecutive knots that are closer than `eps` to their
/// run mean, turning near-zero-length spans into exact multiplicity. The
/// knot and control counts are preserved; the result is re-validated, so a
/// collapse that would exceed the permitted multiplicity is an error.
pub fn merge_near_knots(curve: &Curve, eps: f64) -> Result<Curve> {
    curve.validate()?;
    check(
        eps.is_finite() && eps > 0.,
        "Merge epsilon must be positive and finite",
    )?;
    let mut knots = curve.knots.clone();
    let mut i = 0;
    while i < knots.len() {
        let mut end = i + 1;
        let mut sum = knots[i];
        while end < knots.len() && knots[end] - knots[end - 1] < eps {
            sum += knots[end];
            end += 1;
        }
        let mean = sum / (end - i) as f64;
        for k in &mut knots[i..end] {
            *k = mean;
        }
        i = end;
    }
    let result = Curve {
        knots,
        ..curve.clone()
    };
    result.validate()?;
    Ok(result)
}
/// Public wrapper around the private clamp logic: fully multiplies the
/// active-domain end knots and trims the exterior knots, dropping any
/// periodic storage.
pub fn clamped(curve: &Curve) -> Result<Curve> {
    curve.validate()?;
    let [a, b] = curve.domain();
    curve.clamp(a, b)
}
/// Affine rescale of the knot vector so the active domain becomes [0, 1].
pub fn normalize_knots(curve: &Curve) -> Result<Curve> {
    curve.validate()?;
    let [a, b] = curve.domain();
    numeric(b > a, "The active knot domain must have positive length")?;
    let mut result = curve.clone();
    for k in &mut result.knots {
        *k = (*k - a) / (b - a);
    }
    result.validate()?;
    Ok(result)
}
