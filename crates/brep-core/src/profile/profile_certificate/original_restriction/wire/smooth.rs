//! Exact regularity over every rational span, and oriented G1 at every join
//! touching an edited piece. Unedited source corners retain their C0 semantics.
use super::*;
use crate::profile_certificate::arithmetic;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Junction {
    pub left: [usize; 2],
    pub right: [usize; 2],
    pub sine_squared: ExactRatio,
}
#[derive(Clone, Debug)]
pub struct SmoothEdits {
    wire: Wire,
    limit: Q,
    junctions: Vec<Junction>,
    regular_spans: usize,
}
pub(super) fn regular(net: &[[Q; 3]]) -> Result<()> {
    let w = arithmetic::power(net, 2);
    let jets: [Vec<Q>; 2] = std::array::from_fn(|a| {
        let x = arithmetic::power(net, a);
        arithmetic::sub(
            &arithmetic::product(&arithmetic::derivative(&x), &w),
            &arithmetic::product(&x, &arithmetic::derivative(&w)),
        )
    });
    let common = arithmetic::gcd(&jets[0], &jets[1])?;
    if common.is_empty()
        || arithmetic::at(&common, &Q::zero()).is_zero()
        || arithmetic::at(&common, &Q::one()).is_zero()
        || arithmetic::algebraic_events::count(&common, &Q::zero(), &Q::one())? > 0
    {
        return Err(refused("Exact wire has a stationary rational span"));
    }
    Ok(())
}
fn tangent(span: &Span, end: bool) -> [Q; 2] {
    let n = span.net.len();
    let (a, b) = if end {
        (&span.net[n - 2], &span.net[n - 1])
    } else {
        (&span.net[0], &span.net[1])
    };
    // Positive degree and endpoint denominator scales do not affect direction.
    std::array::from_fn(|axis| &b[axis] * &a[2] - &a[axis] * &b[2])
}
pub(super) fn join(
    wire: &Wire,
    left: [usize; 2],
    right: [usize; 2],
    limit: &Q,
) -> Result<Junction> {
    let a = tangent(&wire.pieces[left[0]][left[1]], true);
    let b = tangent(&wire.pieces[right[0]][right[1]], false);
    let aa = &a[0] * &a[0] + &a[1] * &a[1];
    let bb = &b[0] * &b[0] + &b[1] * &b[1];
    let dot = &a[0] * &b[0] + &a[1] * &b[1];
    if aa.is_zero() || bb.is_zero() || dot <= Q::zero() {
        return Err(refused("Exact wire G1 has a zero or reversed tangent"));
    }
    let cross = &a[0] * &b[1] - &a[1] * &b[0];
    let sine_squared = &cross * &cross / (aa * bb);
    if sine_squared > *limit {
        return Err(refused("Exact wire G1 angle exceeds its bound"));
    }
    Ok(Junction {
        left,
        right,
        sine_squared: ratio(&sine_squared),
    })
}
fn derive(wire: &Wire, limit: &Q) -> Result<(Vec<Junction>, usize)> {
    wire.reverify()?;
    if limit < &Q::zero() || limit >= &Q::one() {
        return Err(refused("Invalid exact wire G1 angle bound"));
    }
    let mut junctions = Vec::new();
    let mut count = 0;
    for (piece, spans) in wire.pieces.iter().enumerate() {
        for span in spans {
            regular(&span.net)?;
            count += 1;
        }
        if wire.editable[piece] {
            for i in 1..spans.len() {
                junctions.push(join(wire, [piece, i - 1], [piece, i], limit)?);
            }
        }
        let next = (piece + 1) % wire.pieces.len();
        if wire.editable[piece] || wire.editable[next] {
            junctions.push(join(wire, [piece, spans.len() - 1], [next, 0], limit)?);
        }
    }
    Ok((junctions, count))
}
impl Wire {
    pub fn qualify_smooth_edits(&self, limit: ExactRatio) -> Result<SmoothEdits> {
        let limit = decode_ratio(&limit)?;
        let (junctions, regular_spans) = derive(self, &limit)?;
        Ok(SmoothEdits {
            wire: self.clone(),
            limit,
            junctions,
            regular_spans,
        })
    }
}
impl SmoothEdits {
    pub fn wire(&self) -> &Wire {
        &self.wire
    }
    pub fn junctions(&self) -> &[Junction] {
        &self.junctions
    }
    pub fn regular_span_count(&self) -> usize {
        self.regular_spans
    }
    pub fn sine_squared_limit(&self) -> ExactRatio {
        ratio(&self.limit)
    }
    pub fn reverify(&self) -> Result<()> {
        let (junctions, count) = derive(&self.wire, &self.limit)?;
        if junctions != self.junctions || count != self.regular_spans {
            return Err(refused("Exact wire smoothness replay changed"));
        }
        Ok(())
    }
}
