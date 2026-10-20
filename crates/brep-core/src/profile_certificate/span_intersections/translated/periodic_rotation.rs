//! Intrinsic CCW order of physical quotient branches in proved local charts.
use super::super::event_nodes::port_order;
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use arithmetic::{branch_neighbourhood::ParameterValue, branch_slice::SliceBranch};
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicTangentWitness {
    pub ports: [usize; 2],
    pub axis: usize,
    pub sign: i8,
    /// Exact source-clock cuts; positions are in the proved COMMON vertex chart.
    pub neighbourhoods: [BranchNeighbourhoodRecord; 2],
    pub order: Ordering,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicRotation {
    owner: Arc<OwnedPeriodicPorts>,
    limits: ContactPortOrderLimits,
    rotations: Vec<Vec<usize>>,
    comparisons: usize,
    tangent_witnesses: Vec<OriginalPeriodicTangentWitness>,
}
fn slice(
    owner: &OwnedPeriodicPorts,
    p: &OriginalPeriodicPort,
    axis: usize,
    sign: i8,
    l: ContactPortOrderLimits,
) -> Result<(SliceBranch, BranchNeighbourhoodRecord)> {
    let charts = owner.owner();
    let graph = charts.owner();
    let aligned = graph.owner();
    let cover = aligned.owner().pieces();
    let piece = &cover.pieces()[p.piece];
    let values = &aligned.cuts()[p.piece];
    let other = if p.cut == p.interval {
        p.interval + 1
    } else if p.cut == p.interval + 1 {
        p.interval
    } else {
        return Err(refused(
            "Periodic tangent port is not at its actual interval endpoint",
        ));
    };
    let decode = |v: &OriginalSourceParameter| -> Result<ParameterValue> {
        Ok(ParameterValue {
            primitive: decode_polynomial(&v.primitive)?,
            value: decode_polynomial(&v.parameter)?,
            interval: [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
        })
    };
    let contact = decode(&values[p.cut].value)?;
    let neighbour = decode(&values[other].value)?;
    let transport = &charts.charts()[p.vertex].uses[p.chart_use];
    if transport.source
        != (OriginalPeriodicChartCutRef {
            piece: p.piece,
            cut: p.cut,
        })
    {
        return Err(refused(
            "Periodic tangent slice uses an unrelated local vertex chart",
        ));
    }
    let actual = cells(&cover.sources()[p.source])?;
    let cell = &actual[p.cell];
    let shift = [
        decode_ratio(&piece.translation[0])? + decode_ratio(&transport.translation[0])?,
        decode_ratio(&piece.translation[1])? + decode_ratio(&transport.translation[1])?,
    ];
    let net = arithmetic::shore_events::translated_net(&cell.net, &shift)?;
    // The complete combined pair/cut census proves this whole original open
    // interval has no other physical contact. The critical-root census below
    // independently supplies a monotone one-sided rational slice BEFORE it.
    let proof = arithmetic::branch_neighbourhood::prove(
        &net,
        &cell.domain,
        contact.clone(),
        Some(neighbour),
        p.direction,
        axis,
        sign,
        l.max_neighbourhood_steps,
    )?;
    let record = BranchNeighbourhoodRecord {
        refined_primitive_root: proof.root.clone().map(|q| ratio(&q)),
        local_contact_box: proof.parameter_box.map(|q| ratio(&q)),
        natural_cut: ratio(&proof.cut),
        cut_position: proof.cut_point.clone().map(|q| ratio(&q)),
        contact_coordinate_critical: proof.contact_critical,
        steps: proof.steps,
    };
    Ok((
        SliceBranch {
            net,
            domain: cell.domain.clone(),
            contact: ParameterValue {
                interval: proof.root,
                ..contact
            },
            cut: proof.cut,
            cut_point: proof.cut_point,
            direction: p.direction,
        },
        record,
    ))
}
fn compare(
    owner: &OwnedPeriodicPorts,
    a: usize,
    b: usize,
    l: ContactPortOrderLimits,
) -> Result<(Ordering, Option<OriginalPeriodicTangentWitness>)> {
    let first = &owner.ports()[a];
    let second = &owner.ports()[b];
    if first.vertex != second.vertex {
        return Err(refused(
            "Periodic cyclic order compares different actual vertices",
        ));
    }
    let leading = port_order::compare(&first.ray, &second.ray, l.algebraic)?;
    if leading != Ordering::Equal {
        return Ok((leading, None));
    }
    let axis = usize::from(first.ray.signs[0] == 0);
    let sign = first.ray.signs[axis];
    if sign == 0 || second.ray.signs[axis] != sign {
        return Err(refused(
            "Periodic tangent ports have incompatible leading directions",
        ));
    }
    let (a_branch, a_proof) = slice(owner, first, axis, sign, l)?;
    let (b_branch, b_proof) = slice(owner, second, axis, sign, l)?;
    let order = arithmetic::branch_slice::compare(
        a_branch,
        b_branch,
        axis,
        sign,
        l.algebraic.max_refinements,
        l.algebraic.max_resultant_size,
    )?;
    Ok((
        order,
        Some(OriginalPeriodicTangentWitness {
            ports: [a, b],
            axis,
            sign,
            neighbourhoods: [a_proof, b_proof],
            order,
        }),
    ))
}
type Data = (Vec<Vec<usize>>, usize, Vec<OriginalPeriodicTangentWitness>);
fn derive(o: &OwnedPeriodicRotation) -> Result<Data> {
    let l = o.limits;
    if l.max_ports == 0
        || l.max_ports > 4096
        || l.max_comparisons > 65536
        || l.max_neighbourhood_steps == 0
        || l.max_neighbourhood_steps > 512
        || l.algebraic.max_refinements > 512
        || !(1..=128).contains(&l.algebraic.max_resultant_size)
    {
        return Err(refused("Invalid original quotient cyclic order allowance"));
    }
    o.owner.reverify_source()?;
    let mut comparisons = 0;
    let mut rotations = Vec::new();
    let mut witnesses = Vec::new();
    for members in o.owner.vertices() {
        if members.len() > l.max_ports {
            return Err(refused(
                "Complete quotient vertex port count exceeds allowance",
            ));
        }
        let mut ordered = Vec::new();
        for &candidate in members {
            let (mut lo, mut hi) = (0, ordered.len());
            while lo < hi {
                if comparisons >= l.max_comparisons {
                    return Err(refused(
                        "Complete quotient cyclic order exhausted comparison allowance",
                    ));
                }
                comparisons += 1;
                let mid = lo + (hi - lo) / 2;
                let (order, witness) = compare(&o.owner, candidate, ordered[mid], l)?;
                if let Some(w) = witness {
                    witnesses.push(w);
                }
                match order {
                    Ordering::Less => hi = mid,
                    Ordering::Greater => lo = mid + 1,
                    Ordering::Equal => {
                        return Err(refused(
                            "Distinct physical quotient branches are not intrinsically separated",
                        ));
                    }
                }
            }
            ordered.insert(lo, candidate);
        }
        let below = ordered
            .iter()
            .take_while(|&&p| {
                o.owner.ports()[p].ray.signs == [1, 0] && o.owner.ports()[p].component_signs[1] < 0
            })
            .count();
        ordered.rotate_left(below);
        rotations.push(ordered);
    }
    Ok((rotations, comparisons, witnesses))
}
impl OwnedPeriodicRotation {
    pub fn owner(&self) -> &Arc<OwnedPeriodicPorts> {
        &self.owner
    }
    pub fn rotations(&self) -> &[Vec<usize>] {
        &self.rotations
    }
    pub fn comparisons_used(&self) -> usize {
        self.comparisons
    }
    pub fn tangent_witnesses(&self) -> &[OriginalPeriodicTangentWitness] {
        &self.tangent_witnesses
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (r, c, t) = derive(self)?;
        if r != self.rotations || c != self.comparisons || t != self.tangent_witnesses {
            return Err(refused(
                "Original physical quotient CCW order, tangent proof or work changed",
            ));
        }
        Ok(())
    }
}
/// CCW from +X. Tangent ties use exact common-chart slices of unchanged
/// source-clock branches; below-axis higher-order +X germs follow the wrap cut.
pub fn own_periodic_rotation(
    owner: Arc<OwnedPeriodicPorts>,
    limits: ContactPortOrderLimits,
) -> Result<OwnedPeriodicRotation> {
    let mut o = OwnedPeriodicRotation {
        owner,
        limits,
        rotations: Vec::new(),
        comparisons: 0,
        tangent_witnesses: Vec::new(),
    };
    let (r, c, t) = derive(&o)?;
    o.rotations = r;
    o.comparisons = c;
    o.tangent_witnesses = t;
    Ok(o)
}
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use periodic_graph::quotient_tests::{aligned, sources};
    pub(in super::super) fn ports(curves: &[Curve]) -> Arc<OwnedPeriodicPorts> {
        let (base, seams) = sources(curves);
        let graph = Arc::new(
            own_periodic_quotient_graph(aligned(base, seams), ParameterOrderLimits::default())
                .unwrap(),
        );
        let charts = Arc::new(own_periodic_vertex_charts(graph, 192, 100000).unwrap());
        Arc::new(own_periodic_ports(charts, 192).unwrap())
    }
    #[test]
    fn periodic_loop_orders_both_distinct_endpoint_ports_and_replays_work() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![4., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let owner = ports(&[a]);
        let o = own_periodic_rotation(owner.clone(), ContactPortOrderLimits::default()).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.rotations(), &[vec![0, 1]]);
        assert!(o.tangent_witnesses().is_empty());
        let mut limits = ContactPortOrderLimits::default();
        limits.max_comparisons = 0;
        assert!(own_periodic_rotation(owner, limits).is_err());
        let mut bad = o.clone();
        bad.rotations[0].reverse();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.comparisons += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn opposite_shore_tangent_ties_use_the_same_proved_intrinsic_center() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0.5],
                vec![0., 0.75],
                vec![1., 0.75],
                vec![0.5, 0.625],
                vec![0., 0.5],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let mut b = a.clone();
        for p in &mut b.control_points {
            p[0] = 4. - p[0];
        }
        let owner = ports(&[a, b]);
        let vertex = owner.vertices().iter().position(|v| v.len() == 4).unwrap();
        let o = own_periodic_rotation(owner, ContactPortOrderLimits::default()).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.rotations()[vertex], vec![3, 0, 4, 7]);
        let upper_corner = o.owner().ports()[5].vertex;
        assert_eq!(o.rotations()[upper_corner], vec![6, 5]);
        assert!(!o.tangent_witnesses().is_empty());
        let proof = o
            .tangent_witnesses()
            .iter()
            .find(|w| w.ports.contains(&0) && w.ports.contains(&4))
            .unwrap();
        assert_eq!(proof.axis, 1);
        assert_eq!(proof.sign, 1);
        assert!(proof.neighbourhoods.iter().all(|r| r.steps > 0));
        let mut bad = o;
        bad.tangent_witnesses[0].neighbourhoods[0].cut_position[0] =
            ["99".to_string(), "1".to_string()];
        assert!(bad.reverify_source().is_err());
    }
}
