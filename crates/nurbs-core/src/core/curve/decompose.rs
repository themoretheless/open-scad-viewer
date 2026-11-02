use super::Curve;
use crate::Result;

pub struct Segment {
    pub(super) curve: Curve,
    pub(super) domain: [f64; 2],
}
impl Segment {
    pub fn definition(&self) -> &Curve {
        &self.curve
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
}

impl Curve {
    pub fn decompose(&self) -> Result<Vec<Segment>> {
        self.validate()?;
        let [a, b] = self.domain();
        let p = self.degree;
        let n = self.control_points.len();
        // A clamped Bernstein-segmented curve already owns its exact poles.
        // Re-entering homogeneous trimming would round (weight * point) /
        // weight even when no insertion is necessary, breaking exact lifts.
        let clamped = !self.periodic
            && self.knots[..=p].iter().all(|&k| k == a)
            && self.knots[n..].iter().all(|&k| k == b);
        let mut segmented = clamped;
        let mut index = p + 1;
        while segmented && index < n {
            let mut end = index + 1;
            while end < n && self.knots[end] == self.knots[index] {
                end += 1;
            }
            segmented &= end - index >= p;
            index = end;
        }
        if segmented {
            return Ok((p..n)
                .filter(|&i| self.knots[i] < self.knots[i + 1])
                .map(|i| {
                    let domain = [self.knots[i], self.knots[i + 1]];
                    Segment {
                        curve: Curve {
                            degree: p,
                            knots: [vec![domain[0]; p + 1], vec![domain[1]; p + 1]].concat(),
                            control_points: self.control_points[i - p..=i].to_vec(),
                            weights: self.weights[i - p..=i].to_vec(),
                            periodic: false,
                        },
                        domain,
                    }
                })
                .collect());
        }
        let mut breaks: Vec<f64> = self
            .knots
            .iter()
            .copied()
            .filter(|k| *k >= a && *k <= b)
            .collect();
        breaks.dedup();
        breaks
            .array_windows()
            .map(|[a, b]| {
                Ok(Segment {
                    curve: self.trim(*a, *b)?,
                    domain: [*a, *b],
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_rational_bezier_decomposition_preserves_original_poles_and_jets() {
        let c = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.8444218515250481, 1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1., 3., 2.],
            periodic: false,
        };
        assert_ne!(c.control_points[1][0] * 3. / 3., c.control_points[1][0]);
        let spans = c.decompose().unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].definition(), &c);
        for t in [-3., -1., 1., 5.] {
            let a = c.evaluate(t).unwrap();
            let b = spans[0].definition().evaluate(t).unwrap();
            assert_eq!(a.point, b.point);
            assert_eq!(a.d1, b.d1);
            assert_eq!(a.d2, b.d2);
        }
    }
    #[test]
    fn literal_segmented_rational_decomposition_owns_every_raw_interval() {
        let c = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 1., 1., 5., 5., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.8444218515250481, 1., 0.],
                vec![1., 0., 0.],
                vec![2., 1., 0.],
                vec![3., 0., 0.],
            ],
            weights: vec![1., 3., 2., 5., 1.],
            periodic: false,
        };
        let before = c.clone();
        let spans = c.decompose().unwrap();
        assert_eq!(spans.len(), 2);
        for (i, s) in spans.iter().enumerate() {
            assert_eq!(s.domain(), if i == 0 { [-3., 1.] } else { [1., 5.] });
            assert_eq!(
                s.definition().control_points,
                c.control_points[2 * i..=2 * i + 2]
            );
            assert_eq!(s.definition().weights, c.weights[2 * i..=2 * i + 2]);
        }
        assert_eq!(c, before);
    }
}
