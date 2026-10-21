//! One locally verified immutable original-boundary view for path construction.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use arithmetic::branch_neighbourhood::ParameterValue;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::sync::Arc;
struct Cell {
    net: Vec<[Q; 3]>,
    domain: [Q; 2],
    bounds: [ParameterValue; 2],
}
pub(in super::super) struct ClearanceView<'a> {
    owner: &'a Arc<OwnedPeriodicSupportLifts>,
    limits: ParameterOrderLimits,
    chart: [[Q; 2]; 2],
    periods: [Q; 2],
    wrapped: [bool; 2],
    arcs: Vec<Cell>,
}
impl<'a> ClearanceView<'a> {
    pub fn new(
        owner: &'a Arc<OwnedPeriodicSupportLifts>,
        limits: ParameterOrderLimits,
    ) -> Result<Self> {
        if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
            return Err(refused("Invalid original clearance view allowance"));
        }
        let _replay = super::periodic_replay::scope();
        super::periodic_replay::verify_arc(owner, |owner| owner.reverify_source())?;
        let selection = owner.owner().owner();
        let graph = selection.rule().material().base().owner().owner().owner();
        let cover = graph.owner().owner().pieces();
        let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
        let periods = chart.clone().map(|d| d[1].clone() - &d[0]);
        let original = cover
            .sources()
            .iter()
            .map(cells)
            .collect::<Result<Vec<_>>>()?;
        let mut seen = vec![false; graph.edges().len()];
        let mut arcs = Vec::new();
        for walk in owner.walks() {
            for part in &walk.parts {
                if part.edge >= seen.len() || seen[part.edge] {
                    return Err(refused("Material path repeats a selected physical edge"));
                }
                seen[part.edge] = true;
                let cell = &original[part.source][part.cell];
                let piece = &cover.pieces()[part.piece];
                let offset = [
                    decode_ratio(&piece.translation[0])?,
                    decode_ratio(&piece.translation[1])?,
                ];
                let net = cell
                    .net
                    .iter()
                    .map(|p| {
                        [
                            &p[0] + &offset[0] * &p[2],
                            &p[1] + &offset[1] * &p[2],
                            p[2].clone(),
                        ]
                    })
                    .collect();
                let mut bounds = part
                    .parameters
                    .iter()
                    .map(|p| {
                        Ok(ParameterValue {
                            primitive: decode_polynomial(&p.primitive)?,
                            value: decode_polynomial(&p.parameter)?,
                            interval: [
                                decode_ratio(&p.interval[0])?,
                                decode_ratio(&p.interval[1])?,
                            ],
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if part.direction < 0 {
                    bounds.reverse();
                }
                arcs.push(Cell {
                    net,
                    domain: cell.domain.clone(),
                    bounds: bounds
                        .try_into()
                        .map_err(|_| refused("Material path lost original cuts"))?,
                });
            }
        }
        if arcs.len() != owner.parts_count() || arcs.len() != selection.selected().len() {
            return Err(refused("Material path omitted selected original geometry"));
        }
        Ok(Self {
            owner,
            limits,
            chart,
            periods,
            wrapped: cover.wrapped(),
            arcs,
        })
    }
    pub fn chart(&self) -> &[[Q; 2]; 2] {
        &self.chart
    }
    pub fn periods(&self) -> &[Q; 2] {
        &self.periods
    }
    pub fn wrapped(&self) -> [bool; 2] {
        self.wrapped
    }
    pub fn limits(&self) -> ParameterOrderLimits {
        self.limits
    }
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportLifts> {
        self.owner
    }
    pub fn inside(&self, p: &[Q; 2]) -> bool {
        (0..2).all(|axis| {
            self.wrapped[axis] || (p[axis] > self.chart[axis][0] && p[axis] < self.chart[axis][1])
        })
    }
    pub fn sample(&self, at: [usize; 2]) -> Result<[Q; 2]> {
        let row = self
            .owner
            .owner()
            .owner()
            .sides()
            .get(at[0])
            .ok_or_else(|| refused("Material path endpoint edge is unknown"))?;
        if at[1] > 1 || !row.selected()[at[1]] || row.samples()[at[1]].is_none() {
            return Err(refused(
                "Material path endpoint is not an actual selected inside sample",
            ));
        }
        let p = &row.probes().queries()[at[1]];
        Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?])
    }
    pub fn bind(&self, endpoints: [[usize; 2]; 2], path: &[[Q; 2]]) -> Result<[[String; 2]; 2]> {
        if path.iter().any(|p| !self.inside(p)) {
            return Err(refused(
                "Material path escaped its actual unwrapped support domain",
            ));
        }
        let mut shifts = [
            [String::new(), String::new()],
            [String::new(), String::new()],
        ];
        for end in 0..2 {
            let sample = self.sample(endpoints[end])?;
            let p = &path[if end == 0 { 0 } else { path.len() - 1 }];
            for axis in 0..2 {
                let delta = &p[axis] - &sample[axis];
                let q = if self.wrapped[axis] {
                    delta / &self.periods[axis]
                } else {
                    delta
                };
                if !q.is_integer() || (!self.wrapped[axis] && !q.is_zero()) {
                    return Err(refused(
                        "Material path does not bind its actual source-owned endpoint modulo support periods",
                    ));
                }
                shifts[end][axis] = q.to_integer().to_string();
            }
        }
        Ok(shifts)
    }
    fn ranges(&self, a: &[Q; 2], b: &[Q; 2]) -> [[BigInt; 2]; 2] {
        std::array::from_fn(|axis| {
            if self.wrapped[axis] {
                let lo = a[axis].clone().min(b[axis].clone());
                let hi = a[axis].clone().max(b[axis].clone());
                [
                    ((lo - &self.chart[axis][1]) / &self.periods[axis])
                        .ceil()
                        .to_integer(),
                    ((hi - &self.chart[axis][0]) / &self.periods[axis])
                        .floor()
                        .to_integer(),
                ]
            } else {
                [BigInt::from(0), BigInt::from(0)]
            }
        })
    }
    pub fn cost(&self, a: &[Q; 2], b: &[Q; 2]) -> Result<usize> {
        let mut n = 1usize;
        for r in &self.ranges(a, b) {
            let c = (&r[1] - &r[0] + BigInt::from(1))
                .to_usize()
                .filter(|n| *n > 0)
                .ok_or_else(|| {
                    refused("Complete material path lattice census exceeds allowance")
                })?;
            n = n
                .checked_mul(c)
                .ok_or_else(|| refused("Complete material path lattice count overflow"))?;
        }
        n.checked_mul(self.arcs.len())
            .ok_or_else(|| refused("Complete material path work overflow"))
    }
    /// False means PROVED closed boundary contact, never budget or arithmetic failure.
    pub fn clear(&self, a: &[Q; 2], b: &[Q; 2]) -> Result<bool> {
        if !self.inside(a) || !self.inside(b) {
            return Err(refused(
                "Material path escaped its actual unwrapped support domain",
            ));
        }
        let range = self.ranges(a, b);
        let required = self.cost(a, b)?;
        let mut tested = 0usize;
        for cell in &self.arcs {
            let mut u = range[0][0].clone();
            while u <= range[0][1] {
                let mut v = range[1][0].clone();
                while v <= range[1][1] {
                    let shift = [
                        Q::from_integer(u.clone()) * &self.periods[0],
                        Q::from_integer(v.clone()) * &self.periods[1],
                    ];
                    let net = cell
                        .net
                        .iter()
                        .map(|p| {
                            [
                                &p[0] + &shift[0] * &p[2],
                                &p[1] + &shift[1] * &p[2],
                                p[2].clone(),
                            ]
                        })
                        .collect::<Vec<_>>();
                    tested += 1;
                    if !arithmetic::selected_clearance::clear(
                        &net,
                        &cell.domain,
                        &cell.bounds,
                        &[a.clone(), b.clone()],
                        self.limits.max_refinements,
                        self.limits.max_resultant_size,
                    )? {
                        return Ok(false);
                    }
                    v += 1;
                }
                u += 1;
            }
        }
        if tested != required {
            return Err(refused(
                "Material path omitted selected original boundary copies",
            ));
        }
        Ok(true)
    }
}
