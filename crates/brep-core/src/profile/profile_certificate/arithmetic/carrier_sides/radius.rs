//! Proposal only: the complete transverse census still authorizes every probe.
use super::*;
const MAX_REFINEMENTS: usize = 512;
/// The normal line of the selected carrier at one parameter.
struct Transversal {
    point: [Q; 2],
    tangent: [Q; 2],
    normal: [Q; 2],
    /// Squared length of the unnormalized normal.
    length: Q,
}
/// `a * x + b * y` as one polynomial.
fn combine(x: &Poly, y: &Poly, a: &Q, b: &Q) -> Poly {
    let mut p = vec![Q::zero(); x.len().max(y.len())];
    for (i, v) in x.iter().enumerate() {
        p[i] += v * a;
    }
    for (i, v) in y.iter().enumerate() {
        p[i] += v * b;
    }
    trim(p)
}
impl Transversal {
    fn new(net: &[[Q; 3]], t: &Q) -> Result<Self> {
        let powers = [power(net, 0), power(net, 1), power(net, 2)];
        let h = powers.clone().map(|p| at(&p, t));
        if h[2] <= Q::zero() {
            return Err(refused("Nonpositive radius proposal denominator"));
        }
        let point = [&h[0] / &h[2], &h[1] / &h[2]];
        let tangent: [Q; 2] = std::array::from_fn(|a| {
            at(&derivative(&powers[a]), t) * &h[2] - &h[a] * at(&derivative(&powers[2]), t)
        });
        let normal = [-tangent[1].clone(), tangent[0].clone()];
        let length = &normal[0] * &normal[0] + &normal[1] * &normal[1];
        if length.is_zero() {
            return Err(refused("Singular transverse radius proposal"));
        }
        Ok(Self {
            point,
            tangent,
            normal,
            length,
        })
    }
    /// Homogeneous offset of a net from the point, measured along the
    /// tangent and along the normal.
    fn offsets(&self, net: &[[Q; 3]]) -> (Poly, Poly) {
        let w = power(net, 2);
        let from_point = |axis: usize| {
            let shift = w.iter().map(|v| v * &self.point[axis]).collect::<Poly>();
            sub(&power(net, axis), &shift)
        };
        let (x, y) = (from_point(0), from_point(1));
        (
            combine(&x, &y, &self.tangent[0], &self.tangent[1]),
            combine(&x, &y, &self.normal[0], &self.normal[1]),
        )
    }
    /// Least lateral distance of a net whose control points all project
    /// onto the normal line, when it stays strictly on one side.
    fn coincident_distance(&self, net: &[[Q; 3]]) -> Result<Q> {
        let values = net
            .iter()
            .map(|h| {
                ((&h[0] - &self.point[0] * &h[2]) * &self.normal[0]
                    + (&h[1] - &self.point[1] * &h[2]) * &self.normal[1])
                    / &h[2]
            })
            .collect::<Vec<_>>();
        let one_sided =
            values.iter().all(|v| v > &Q::zero()) || values.iter().all(|v| v < &Q::zero());
        values
            .iter()
            .map(Q::abs)
            .min()
            .filter(|_| one_sided)
            .ok_or_else(|| refused("Unresolved coincident normal-line radius proposal"))
    }
    /// A radius safely below the given lateral distance.
    fn scale(&self, distance: Q) -> Q {
        distance / (&self.length * Q::from_integer(4.into()))
    }
}
/// Lower bound of the lateral distance at one crossing of the normal line.
fn root_distance(
    f: &Poly,
    lateral: &Poly,
    w: &Poly,
    mut root: [Q; 2],
    refinements: usize,
) -> Result<Q> {
    for step in 0..=refinements {
        let r = parameter_order::range(lateral, &root);
        let lower = if r[0] > Q::zero() {
            Some(r[0].clone())
        } else if r[1] < Q::zero() {
            Some(-&r[1])
        } else {
            None
        };
        if let Some(lower) = lower {
            let weight = parameter_order::range(w, &root)[1].clone();
            if weight > Q::zero() {
                return Ok(lower / weight);
            }
        }
        if step < refinements {
            parameter_order::refine(f, &mut root)?;
        }
    }
    Err(refused("Transverse radius distance remains unresolved"))
}
/// Choose a scale from exact normal-line intersections instead of starting at
/// one world unit. All nonzero transverse distances contribute. Unresolved
/// bounds refuse a proposal; no branch or center ownership is certified here.
pub(in crate::profile::profile_certificate) fn suggest(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    refinements: usize,
) -> Result<Q> {
    if selected >= nets.len() || refinements > MAX_REFINEMENTS || t <= &Q::zero() || t >= &Q::one()
    {
        return Err(refused("Invalid transverse radius proposal"));
    }
    let line = Transversal::new(&nets[selected], t)?;
    let mut radius = Q::one();
    for net in nets {
        if net.iter().any(|h| h[2] <= Q::zero()) {
            return Err(refused("Nonpositive radius proposal weight"));
        }
        let (f, lateral) = line.offsets(net);
        if f.is_empty() {
            radius = radius.min(line.scale(line.coincident_distance(net)?));
            continue;
        }
        let w = power(net, 2);
        for root in
            algebraic_events::isolates(&f, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?
        {
            if zero_at(&lateral, &f, &root)? {
                continue;
            }
            let distance = root_distance(&f, &lateral, &w, root, refinements)?;
            radius = radius.min(line.scale(distance));
        }
    }
    Ok(radius)
}
