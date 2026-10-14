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
