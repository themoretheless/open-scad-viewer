//! Exact contact-free monotone source branch neighbourhood, before a rational cut.
use super::*;
use event_coordinates as exact;
use parameter_order::{closed_roots, range, refine};
#[derive(Clone)]
pub(in crate::profile::profile_certificate) struct ParameterValue {
    pub primitive: Poly,
    pub value: Poly,
    pub interval: [Q; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub(in crate::profile::profile_certificate) struct Neighbourhood {
    pub root: [Q; 2],
    pub parameter_box: [Q; 2],
    pub cut: Q,
    pub cut_point: [Q; 2],
    pub contact_critical: bool,
    pub steps: usize,
}
/// Source coordinate (0 = x, 1 = y) and the sign it must keep along the branch.
#[derive(Clone, Copy, Debug)]
pub(in crate::profile::profile_certificate) struct MonotoneAxis {
    pub axis: usize,
    pub sign: i8,
}
pub(super) fn local(p: &Poly, domain: &[Q; 2]) -> Poly {
    sub(p, &[domain[0].clone()])
        .iter()
        .map(|q| q / (&domain[1] - &domain[0]))
        .collect()
}
pub(super) fn box_at(p: &Poly, f: &Poly, r: &[Q; 2]) -> Result<[Q; 2]> {
    if exact::zero_at(p, f, r)? {
        return Ok([Q::zero(), Q::zero()]);
    }
    if exact::equals_one(p, f, r)? {
        return Ok([Q::one(), Q::one()]);
    }
    let b = range(p, r);
    Ok([b[0].clone().max(Q::zero()), b[1].clone().min(Q::one())])
}
const MAX_STEPS: usize = 512;
/// Search for a contact-free monotone cut on one side of a contact.
struct CutSearch<'a> {
    domain: &'a [Q; 2],
    powers: [Poly; 3],
    /// Numerator of the derivative of the selected coordinate.
    critical: Poly,
    direction: i8,
    /// Sign the critical polynomial must have at the cut.
    expected: std::cmp::Ordering,
    contact_critical: bool,
    /// Candidate cut in the local parameter of the cell.
    cut: Q,
}
impl CutSearch<'_> {
    /// Moves the cut halfway towards the contact enclosure. False while the
    /// enclosure still reaches the cut.
    fn approach(&mut self, b: &[Q; 2]) -> bool {
        let near = if self.direction > 0 { &b[1] } else { &b[0] };
        let clear = if self.direction > 0 {
            *near < self.cut
        } else {
            *near > self.cut
        };
        if clear {
            self.cut = (self.cut.clone() + near) / Q::from_integer(2.into());
        }
        clear
    }
    fn natural(&self) -> Q {
        &self.domain[0] + (&self.domain[1] - &self.domain[0]) * &self.cut
    }
    /// The cut lies strictly before the neighbouring contact.
    fn separated(&self, natural: &Q, neighbour: Option<&ParameterValue>) -> bool {
        neighbour.is_none_or(|n| {
            let r = range(&n.value, &n.interval);
            if self.direction > 0 {
                *natural < r[0]
            } else {
                *natural > r[1]
            }
        })
    }
    /// Between the contact enclosure and the cut the coordinate has no
    /// critical point other than the contact's own and runs the expected way.
    fn monotone(&self, b: &[Q; 2]) -> Result<bool> {
        let hull = [
            b[0].clone().min(self.cut.clone()),
            b[1].clone().max(self.cut.clone()),
        ];
        Ok(
            closed_roots(&self.critical, &hull)? == usize::from(self.contact_critical)
                && at(&self.critical, &self.cut).cmp(&Q::zero()) == self.expected,
        )
    }
    fn cut_point(&self) -> Result<[Q; 2]> {
        let w = at(&self.powers[2], &self.cut);
        if w <= Q::zero() {
            return Err(refused("Original cut denominator is not positive"));
        }
        Ok([
            at(&self.powers[0], &self.cut) / &w,
            at(&self.powers[1], &self.cut) / w,
        ])
    }
}
pub(in crate::profile::profile_certificate) fn prove(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    contact: ParameterValue,
    mut neighbour: Option<ParameterValue>,
    direction: i8,
    monotone: MonotoneAxis,
    max_steps: usize,
) -> Result<Neighbourhood> {
    let MonotoneAxis {
        axis,
        sign: axis_sign,
    } = monotone;
    if axis > 1
        || !matches!(direction, -1 | 1)
        || !matches!(axis_sign, -1 | 1)
        || !(1..=MAX_STEPS).contains(&max_steps)
        || domain[0] >= domain[1]
        || net.iter().any(|h| h[2] <= Q::zero())
    {
        return Err(refused(
            "Invalid original monotone branch neighbourhood context",
        ));
    }
    let powers = [power(net, 0), power(net, 1), power(net, 2)];
    let critical = sub(
        &product(&derivative(&powers[axis]), &powers[2]),
        &product(&powers[axis], &derivative(&powers[2])),
    );
    if critical.is_empty() {
        return Err(refused("Selected branch coordinate is constant"));
    }
    let p = local(&contact.value, domain);
    let mut root = contact.interval.clone();
    let contact_critical = exact::zero_at(
        &exact::compose(&critical, &p, &contact.primitive)?,
        &contact.primitive,
        &root,
    )?;
    let mut search = CutSearch {
        domain,
        powers,
        critical,
        direction,
        expected: if axis_sign * direction > 0 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        },
        contact_critical,
        cut: if direction > 0 { Q::one() } else { Q::zero() },
    };
    for step in 1..=max_steps {
        let b = box_at(&p, &contact.primitive, &root)?;
        if b[0] > b[1] {
            return Err(refused(
                "Original contact parameter enclosure leaves its actual source cell",
            ));
        }
        if search.approach(&b) {
            let natural = search.natural();
            if search.separated(&natural, neighbour.as_ref()) && search.monotone(&b)? {
                return Ok(Neighbourhood {
                    root,
                    cut_point: search.cut_point()?,
                    parameter_box: b,
                    cut: natural,
                    contact_critical,
                    steps: step,
                });
            }
        }
        if step < max_steps {
            refine(&contact.primitive, &mut root)?;
            if let Some(n) = &mut neighbour {
                refine(&n.primitive, &mut n.interval)?;
            }
        }
    }
    Err(refused(
        "Original contact-free monotone neighbourhood remains unproven within allowance",
    ))
}
