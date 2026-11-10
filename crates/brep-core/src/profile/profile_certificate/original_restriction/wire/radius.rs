//! Whole-span radial deviation bound on exact edited rational geometry.
//! A nonzero bound is not an exact circle identity.
use super::*;
use num_traits::Signed;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanBound {
    pub span: [usize; 2],
    pub radial_error: ExactRatio,
}
#[derive(Clone, Debug)]
pub struct Radius {
    wire: Wire,
    center: [Q; 2],
    radius: Q,
    tolerance: Q,
    spans: Vec<SpanBound>,
}
fn choose(n: usize, k: usize) -> i64 {
    let mut v = 1;
    for i in 0..k {
        v = v * (n - i) as i64 / (i + 1) as i64;
    }
    v
}
fn product(a: &[Q], b: &[Q]) -> Vec<Q> {
    let n = a.len() - 1;
    let m = b.len() - 1;
    let mut out = vec![Q::zero(); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            out[i + j] += &a[i]
                * &b[j]
                * Q::new(
                    (choose(n, i) * choose(m, j)).into(),
                    choose(n + m, i + j).into(),
                );
        }
    }
    out
}
fn derive(wire: &Wire, center: &[Q; 2], radius: &Q, tolerance: &Q) -> Result<Vec<SpanBound>> {
    wire.reverify()?;
    if radius <= &Q::zero() || tolerance < &Q::zero() {
        return Err(refused("Invalid exact radial target or tolerance"));
    }
    let mut bounds = Vec::new();
    for (p, piece) in wire.pieces.iter().enumerate() {
        if !wire.editable[p] {
            continue;
        }
        for (s, span) in piece.iter().enumerate() {
            if !(2..=9).contains(&span.net.len()) {
                return Err(refused("Exact radial span degree unsupported"));
            }
            let w = span.net.iter().map(|h| h[2].clone()).collect::<Vec<_>>();
            let minimum = w.iter().min().unwrap();
            if minimum <= &Q::zero() {
                return Err(refused("Exact radial span has nonpositive denominator"));
            }
            let xy: [Vec<Q>; 2] = std::array::from_fn(|k| {
                span.net
                    .iter()
                    .map(|h| &h[k] - &center[k] * &h[2])
                    .collect()
            });
            let n = product(&xy[0], &xy[0])
                .into_iter()
                .zip(product(&xy[1], &xy[1]))
                .zip(product(&w, &w))
                .map(|((x, y), w)| (x + y - radius * radius * w).abs())
                .max()
                .unwrap();
            // |d-r| = |d²-r²|/(d+r) <= |N|/(min(W)²*r), r>0.
            let error = n / (minimum * minimum * radius);
            if error > *tolerance {
                return Err(refused(
                    "Whole exact edited span exceeds target radius tolerance",
                ));
            }
            bounds.push(SpanBound {
                span: [p, s],
                radial_error: ratio(&error),
            });
        }
    }
    if bounds.is_empty() {
        return Err(refused("Exact radial certificate has no edited spans"));
    }
    Ok(bounds)
}
impl Wire {
    pub fn qualify_edited_radius(
        &self,
        center: [ExactRatio; 2],
        radius: ExactRatio,
        tolerance: ExactRatio,
    ) -> Result<Radius> {
        let center = [decode_ratio(&center[0])?, decode_ratio(&center[1])?];
        let radius = decode_ratio(&radius)?;
        let tolerance = decode_ratio(&tolerance)?;
        let spans = derive(self, &center, &radius, &tolerance)?;
        Ok(Radius {
            wire: self.clone(),
            center,
            radius,
            tolerance,
            spans,
        })
    }
}
impl Radius {
    pub fn wire(&self) -> &Wire {
        &self.wire
    }
    pub fn center(&self) -> [ExactRatio; 2] {
        self.center.each_ref().map(ratio)
    }
    pub fn radius(&self) -> ExactRatio {
        ratio(&self.radius)
    }
    pub fn tolerance(&self) -> ExactRatio {
        ratio(&self.tolerance)
    }
    pub fn spans(&self) -> &[SpanBound] {
        &self.spans
    }
    pub fn exact(&self) -> bool {
        self.spans
            .iter()
            .all(|s| s.radial_error == ["0".to_string(), "1".to_string()])
    }
    pub fn reverify(&self) -> Result<()> {
        if derive(&self.wire, &self.center, &self.radius, &self.tolerance)? != self.spans {
            return Err(refused("Exact edited radius certificate replay changed"));
        }
        Ok(())
    }
}
