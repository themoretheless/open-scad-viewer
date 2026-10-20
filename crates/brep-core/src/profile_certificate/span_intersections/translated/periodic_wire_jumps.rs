//! Sparse exact original-wire jumps, local conservation and global homology.
use super::*;
use num_bigint::BigInt;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWireContribution {
    pub carrier: usize,
    pub source: usize,
    pub wire: usize,
    pub sign: i8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWireJump {
    pub wire: usize,
    pub delta: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicEdgeJump {
    pub edge: usize,
    pub deck: [String; 2],
    pub contributions: Vec<OriginalPeriodicWireContribution>,
    pub jumps: Vec<OriginalPeriodicWireJump>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicBalanceTerm {
    pub edge: usize,
    pub end: usize,
    pub wire: usize,
    pub delta: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicVertexBalance {
    pub vertex: usize,
    pub terms: Vec<OriginalPeriodicBalanceTerm>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWireFlux {
    pub wire: usize,
    pub native_homology: [Option<String>; 2],
    pub reconstructed: [String; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicWireJumps {
    owner: Arc<OwnedPeriodicVertexCharts>,
    max_work: usize,
    edges: Vec<OriginalPeriodicEdgeJump>,
    balances: Vec<OriginalPeriodicVertexBalance>,
    fluxes: Vec<OriginalPeriodicWireFlux>,
    work: usize,
}
fn integer(s: &str) -> Result<BigInt> {
    s.parse()
        .map_err(|_| refused("Invalid exact wire jump or source period count"))
}
fn bump(work: &mut usize, max: usize) -> Result<()> {
    if *work >= max {
        return Err(refused(
            "Complete original wire jump proof exhausted its work allowance",
        ));
    }
    *work += 1;
    Ok(())
}
type Data = (
    Vec<OriginalPeriodicEdgeJump>,
    Vec<OriginalPeriodicVertexBalance>,
    Vec<OriginalPeriodicWireFlux>,
    usize,
);
fn derive(o: &OwnedPeriodicWireJumps) -> Result<Data> {
    let graph = o.owner.owner();
    let cover = graph.owner().owner().pieces();
    if o.max_work == 0 || o.max_work > 10000000 {
        return Err(refused("Invalid complete original wire jump allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |x| x.reverify_source())?;
    let wires = graph.seam_contacts().embedding().wires();
    let mut source_wire = vec![None; cover.sources().len()];
    for (wire, uses) in wires.source_uses().iter().enumerate() {
        for u in uses {
            let slot = source_wire
                .get_mut(u.source)
                .ok_or_else(|| refused("Wire jump names an unrelated original source"))?;
            if slot.replace((wire, u.reversed)).is_some() {
                return Err(refused("Wire jump source ownership is not unique"));
            }
        }
    }
    if source_wire.iter().any(Option::is_none) {
        return Err(refused("Original wire jump census omits a source"));
    }
    let mut work = 0;
    let mut shifts = BTreeMap::new();
    for chart in o.owner.charts() {
        for u in &chart.uses {
            bump(&mut work, o.max_work)?;
            let value = [integer(&u.period_shift[0])?, integer(&u.period_shift[1])?];
            if shifts.insert(u.source, value).is_some() {
                return Err(refused(
                    "Original wire flux has duplicate source-cut transport",
                ));
            }
        }
    }
    let mut flux = vec![[BigInt::from(0), BigInt::from(0)]; wires.wires().len()];
    let mut edges = Vec::new();
    for (edge, e) in graph.edges().iter().enumerate() {
        let first = e
            .carriers
            .first()
            .ok_or_else(|| refused("Wire jump physical edge lacks an original carrier"))?;
        let transport = |end: usize| -> Result<&[BigInt; 2]> {
            let source_end = end ^ usize::from(first.reversed);
            shifts
                .get(&OriginalPeriodicChartCutRef {
                    piece: first.piece,
                    cut: first.interval + source_end,
                })
                .ok_or_else(|| refused("Original wire flux edge endpoint lacks a proved chart"))
        };
        let a = transport(0)?;
        let b = transport(1)?;
        let deck = [&a[0] - &b[0], &a[1] - &b[1]];
        let mut sums: BTreeMap<usize, BigInt> = BTreeMap::new();
        let mut contributions = Vec::new();
        for (carrier, c) in e.carriers.iter().enumerate() {
            bump(&mut work, o.max_work)?;
            let (wire, reversed) = source_wire
                .get(c.source)
                .and_then(|v| *v)
                .ok_or_else(|| refused("Original edge contribution has no authored wire"))?;
            let sign = if reversed ^ c.reversed { -1 } else { 1 };
            *sums.entry(wire).or_default() += BigInt::from(sign);
            contributions.push(OriginalPeriodicWireContribution {
                carrier,
                source: c.source,
                wire,
                sign,
            });
        }
        let mut jumps = Vec::new();
        for (wire, delta) in sums {
            if delta == BigInt::from(0) {
                continue;
            }
            for axis in 0..2 {
                bump(&mut work, o.max_work)?;
                flux[wire][axis] += &delta * &deck[axis];
            }
            jumps.push(OriginalPeriodicWireJump {
                wire,
                delta: delta.to_string(),
            });
        }
        edges.push(OriginalPeriodicEdgeJump {
            edge,
            deck: deck.map(|q| q.to_string()),
            contributions,
            jumps,
        });
    }
    let mut balances = Vec::new();
    for (vertex, v) in graph.vertices().iter().enumerate() {
        let mut sums: BTreeMap<usize, BigInt> = BTreeMap::new();
        let mut terms = Vec::new();
        for incidence in &v.incidences {
            if incidence.end > 1 {
                return Err(refused(
                    "Original wire conservation has invalid endpoint incidence",
                ));
            }
            let edge = edges
                .get(incidence.edge)
                .ok_or_else(|| refused("Original wire conservation lost its physical edge"))?;
            for j in &edge.jumps {
                bump(&mut work, o.max_work)?;
                let value = integer(&j.delta)?;
                let delta = if incidence.end == 0 { value } else { -value };
                *sums.entry(j.wire).or_default() += &delta;
                terms.push(OriginalPeriodicBalanceTerm {
                    edge: incidence.edge,
                    end: incidence.end,
                    wire: j.wire,
                    delta: delta.to_string(),
                });
            }
        }
        if sums.values().any(|q| q != &BigInt::from(0)) {
            return Err(refused(
                "Authored original wire jumps do not conserve at the actual physical vertex",
            ));
        }
        balances.push(OriginalPeriodicVertexBalance { vertex, terms });
    }
    let mut fluxes = Vec::new();
    for (wire, w) in wires.wires().iter().enumerate() {
        for axis in 0..2 {
            let expected = if let Some(v) = &w.homology[axis] {
                integer(v)?
            } else {
                BigInt::from(0)
            };
            if flux[wire][axis] != expected {
                return Err(refused(
                    "Physical edge contributions do not reconstruct ORIGINAL authored wire homology",
                ));
            }
        }
        fluxes.push(OriginalPeriodicWireFlux {
            wire,
            native_homology: w.homology.clone(),
            reconstructed: flux[wire].clone().map(|q| q.to_string()),
        });
    }
    Ok((edges, balances, fluxes, work))
}
impl OwnedPeriodicWireJumps {
    pub fn owner(&self) -> &Arc<OwnedPeriodicVertexCharts> {
        &self.owner
    }
    pub fn edges(&self) -> &[OriginalPeriodicEdgeJump] {
        &self.edges
    }
    pub fn vertex_balances(&self) -> &[OriginalPeriodicVertexBalance] {
        &self.balances
    }
    pub fn wire_fluxes(&self) -> &[OriginalPeriodicWireFlux] {
        &self.fluxes
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (e, b, f, w) = derive(self)?;
        if e != self.edges || b != self.balances || f != self.fluxes || w != self.work {
            return Err(refused(
                "Original material wire jumps, balance, homology or work changed",
            ));
        }
        Ok(())
    }
}
/// Per-authored-wire jumps stay SEPARATE: opposite independent wires are not
/// prematurely collapsed to one scalar fill. This proves local conservation
/// and original global homology, not material sides or Boolean selection.
pub fn own_periodic_wire_jumps(
    owner: Arc<OwnedPeriodicVertexCharts>,
    max_work: usize,
) -> Result<OwnedPeriodicWireJumps> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicWireJumps {
        owner,
        max_work,
        edges: Vec::new(),
        balances: Vec::new(),
        fluxes: Vec::new(),
        work: 0,
    };
    let (e, b, f, w) = derive(&o)?;
    o.edges = e;
    o.balances = b;
    o.fluxes = f;
    o.work = w;
    Ok(o)
}
#[cfg(test)]
pub(in crate::profile_certificate::span_intersections::translated) mod tests {
    use super::*;
    pub(in crate::profile_certificate::span_intersections::translated) fn own(
        curves: &[Curve],
        uses: &[Vec<OriginalPeriodicSourceUse>],
    ) -> OwnedPeriodicWireJumps {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        own_surface(
            curves,
            uses,
            &surface,
            [true, false],
            OriginalAtlasPairLimits::default(),
        )
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn own_surface(
        curves: &[Curve],
        uses: &[Vec<OriginalPeriodicSourceUse>],
        surface: &nurbs_core::surface::Surface,
        wrapped: [bool; 2],
        pair_limits: OriginalAtlasPairLimits,
    ) -> OwnedPeriodicWireJumps {
        let chart = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let _replay = super::super::periodic_replay::scope();
        let copies = 3usize.pow(wrapped.iter().filter(|w| **w).count() as u32);
        let cover = Arc::new(
            own_original_periodic_pieces(
                curves,
                chart,
                wrapped,
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let n = cover.pieces().len();
        let count = n * (n + 1) / 2;
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                cover.clone(),
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let wires = Arc::new(own_original_periodic_wires(support, uses).unwrap());
        let embedding = Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                pair_limits,
            )
            .unwrap(),
        );
        let base = Arc::new(
            own_periodic_arrangement_cuts(cover, 1e-12, ParameterOrderLimits::default(), count)
                .unwrap(),
        );
        let seams = Arc::new(
            own_periodic_seam_contacts(
                embedding,
                1e-12,
                ParameterOrderLimits::default(),
                count * (copies - 1),
            )
            .unwrap(),
        );
        let combined = Arc::new(
            own_periodic_quotient_arrangement_cuts(
                base,
                seams,
                ParameterOrderLimits::default(),
                count * copies,
            )
            .unwrap(),
        );
        let aligned = Arc::new(
            own_periodic_aligned_intervals(combined, ParameterOrderLimits::default(), 100000)
                .unwrap(),
        );
        let graph = Arc::new(
            own_periodic_quotient_graph(aligned, ParameterOrderLimits::default()).unwrap(),
        );
        let charts = Arc::new(own_periodic_vertex_charts(graph, 192, 100000).unwrap());
        own_periodic_wire_jumps(charts, 100000).unwrap()
    }
    fn line(a: f64, b: f64) -> Curve {
        Curve::from_polyline(vec![vec![a, 0.5], vec![b, 0.5]]).unwrap()
    }
    #[test]
    fn twice_authored_loop_keeps_jump_two_and_both_conserving_loop_incidence_terms() {
        let o = own(
            &[line(0., 8.)],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        o.reverify_source().unwrap();
        assert_eq!(o.edges().len(), 1);
        assert_eq!(o.edges()[0].contributions.len(), 2);
        assert_eq!(o.edges()[0].deck, ["1", "0"]);
        assert_eq!(
            o.edges()[0].jumps,
            vec![OriginalPeriodicWireJump {
                wire: 0,
                delta: "2".to_string()
            }]
        );
        assert_eq!(o.vertex_balances()[0].terms.len(), 2);
        assert_eq!(o.vertex_balances()[0].terms[0].delta, "2");
        assert_eq!(o.vertex_balances()[0].terms[1].delta, "-2");
        assert_eq!(o.wire_fluxes()[0].reconstructed, ["2", "0"]);
        assert_eq!(
            o.wire_fluxes()[0].native_homology,
            [Some("2".to_string()), None]
        );
        assert!(own_periodic_wire_jumps(o.owner().clone(), 1).is_err());
        let mut bad = o.clone();
        bad.edges[0].contributions.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.balances[0].terms.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.fluxes[0].reconstructed[0] = "1".to_string();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.work += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn same_wire_retracing_cancels_but_authored_reverse_flag_changes_the_signed_flux() {
        let o = own(
            &[line(0., 4.), line(4., 0.)],
            &[vec![
                OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                },
                OriginalPeriodicSourceUse {
                    source: 1,
                    reversed: false,
                },
            ]],
        );
        o.reverify_source().unwrap();
        assert_eq!(o.edges().len(), 1);
        assert_eq!(
            o.edges()[0]
                .contributions
                .iter()
                .map(|c| c.sign)
                .collect::<Vec<_>>(),
            vec![1, -1]
        );
        assert!(o.edges()[0].jumps.is_empty());
        assert!(o.vertex_balances()[0].terms.is_empty());
        assert_eq!(o.wire_fluxes()[0].reconstructed, ["0", "0"]);
        let reversed = own(
            &[line(0., 4.)],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: true,
            }]],
        );
        reversed.reverify_source().unwrap();
        assert_eq!(
            reversed.edges()[0].jumps,
            vec![OriginalPeriodicWireJump {
                wire: 0,
                delta: "-1".to_string()
            }]
        );
        assert_eq!(reversed.wire_fluxes()[0].reconstructed, ["-1", "0"]);
    }
    #[test]
    fn independent_wire_columns_never_collapse_to_a_false_scalar_cancellation() {
        let o = own(
            &[line(0., 4.), line(4., 0.)],
            &[
                vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }],
                vec![OriginalPeriodicSourceUse {
                    source: 1,
                    reversed: false,
                }],
            ],
        );
        o.reverify_source().unwrap();
        assert_eq!(
            o.edges()[0].jumps,
            vec![
                OriginalPeriodicWireJump {
                    wire: 0,
                    delta: "1".to_string()
                },
                OriginalPeriodicWireJump {
                    wire: 1,
                    delta: "-1".to_string()
                }
            ]
        );
        assert_eq!(o.vertex_balances()[0].terms.len(), 4);
        assert_eq!(o.wire_fluxes()[0].reconstructed, ["1", "0"]);
        assert_eq!(o.wire_fluxes()[1].reconstructed, ["-1", "0"]);
        let flipped = own(
            &[line(0., 4.), line(4., 0.)],
            &[
                vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }],
                vec![OriginalPeriodicSourceUse {
                    source: 1,
                    reversed: true,
                }],
            ],
        );
        flipped.reverify_source().unwrap();
        assert_eq!(
            flipped.edges()[0].jumps,
            vec![
                OriginalPeriodicWireJump {
                    wire: 0,
                    delta: "1".to_string()
                },
                OriginalPeriodicWireJump {
                    wire: 1,
                    delta: "1".to_string()
                }
            ]
        );
        assert_eq!(flipped.edges()[0].contributions[1].sign, 1);
        assert_eq!(flipped.wire_fluxes()[1].reconstructed, ["1", "0"]);
    }
}
