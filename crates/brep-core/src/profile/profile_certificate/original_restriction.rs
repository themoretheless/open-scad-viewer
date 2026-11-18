//! Exact original-curve restrictions, without rounding rational coefficients
//! back into binary64 control points. Does not certify a closed wire or Body.
use super::*;
use algebraic_intersections::{decode_ratio, ratio};
pub mod wire;
pub mod plane;
pub mod vertical;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    range: [Q; 2],
    net: Vec<[Q; 3]>,
}
impl Span {
    pub fn parameter_range(&self) -> [ExactRatio; 2] {
        self.range.each_ref().map(ratio)
    }
    pub fn homogeneous_controls(&self) -> Vec<[ExactRatio; 3]> {
        self.net.iter().map(|h| h.each_ref().map(ratio)).collect()
    }
}
#[derive(Clone, Debug)]
pub struct Restriction {
    source: Curve,
    range: [Q; 2],
    spans: Vec<Span>,
}
fn derive(source: &Curve, range: &[Q; 2]) -> Result<Vec<Span>> {
    source_bits(&[vec![source.clone()]], 1e-7)?;
    let domain = source.domain().map(|x| Q::from_float(x).unwrap());
    let lo = range[0].clone().min(range[1].clone());
    let hi = range[0].clone().max(range[1].clone());
    if lo >= hi || lo < domain[0] || hi > domain[1] {
        return Err(refused(
            "Original restriction interval is empty or outside the source",
        ));
    }
    let mut cuts = vec![lo.clone(), hi.clone()];
    cuts.extend(
        source
            .knots
            .iter()
            .map(|x| Q::from_float(*x).unwrap())
            .filter(|k| k > &lo && k < &hi),
    );
    cuts.sort();
    cuts.dedup();
    if cuts.len() > 65 {
        return Err(refused("Original restriction exceeds 64 knot spans"));
    }
    if range[0] > range[1] {
        cuts.reverse();
    }
    let spans = cuts
        .windows(2)
        .map(|p| {
            let net = extraction::interval(source, &p[0], &p[1])?;
            if net.iter().any(|h| h[2] <= Q::zero()) {
                return Err(refused("Original restriction has nonpositive weights"));
            }
            Ok(Span {
                range: [p[0].clone(), p[1].clone()],
                net,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    for pair in spans.windows(2) {
        let a = pair[0].net.last().unwrap();
        let b = &pair[1].net[0];
        if (0..2).any(|axis| &a[axis] * &b[2] != &b[axis] * &a[2]) {
            return Err(refused(
                "Original restriction crosses a discontinuous source knot",
            ));
        }
    }
    Ok(spans)
}
pub fn restrict(source: &Curve, range: [ExactRatio; 2]) -> Result<Restriction> {
    let range = [decode_ratio(&range[0])?, decode_ratio(&range[1])?];
    let spans = derive(source, &range)?;
    Ok(Restriction {
        source: source.clone(),
        range,
        spans,
    })
}
pub fn restrict_binary64(source: &Curve, range: [f64; 2]) -> Result<Restriction> {
    if range.iter().any(|v| !v.is_finite()) {
        return Err(refused("Nonfinite original restriction interval"));
    }
    restrict(source, range.map(|v| ratio(&Q::from_float(v).unwrap())))
}
impl Restriction {
    pub fn source(&self) -> &Curve {
        &self.source
    }
    pub fn parameter_range(&self) -> [ExactRatio; 2] {
        self.range.each_ref().map(ratio)
    }
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
    pub fn reverify(&self) -> Result<()> {
        if derive(&self.source, &self.range)? != self.spans {
            return Err(refused("Original restriction data changed"));
        }
        Ok(())
    }
    pub fn evaluate_natural(&self, parameter: ExactRatio) -> Result<[ExactRatio; 2]> {
        let parameter = decode_ratio(&parameter)?;
        let span = self
            .spans
            .iter()
            .find(|s| {
                parameter >= s.range[0].clone().min(s.range[1].clone())
                    && parameter <= s.range[0].clone().max(s.range[1].clone())
            })
            .ok_or_else(|| refused("Evaluation is outside the original restriction"))?;
        let t = (&parameter - &span.range[0]) / (&span.range[1] - &span.range[0]);
        let mut net = span.net.clone();
        for n in (1..net.len()).rev() {
            for i in 0..n {
                net[i] =
                    std::array::from_fn(|a| &net[i][a] * (Q::one() - &t) + &net[i + 1][a] * &t);
            }
        }
        Ok([
            ratio(&(&net[0][0] / &net[0][2])),
            ratio(&(&net[0][1] / &net[0][2])),
        ])
    }
}
