//! Model-wide shell nesting evidence. Role consistency is conditional on each
//! shell being an embedded geometric boundary; this is not a solid certificate.
use crate::{Error, Model, Result, shell_relation, solid_audit};

pub struct Pair {
    pub shells: [usize; 2],
    pub result: shell_relation::Report,
}
pub struct Report {
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    /// None when any pair is unvisited/unresolved or inclusion is inconsistent.
    pub parents: Option<Vec<Option<usize>>>,
    /// Cavities must be immediate children of their owning outer shell.
    /// Separate material bodies can be roots or islands inside cavity shells.
    /// Does not assert that shells have the correct geometric orientation.
    pub roles_consistent: Option<bool>,
}
pub fn inspect(model: &Model, tolerance_mm: f64, tolerance_uv: f64,
    max_pairs: usize, max_cells: usize, max_domain_cells: usize) -> Result<Report> {
    inspect_with_boundary(model,tolerance_mm,tolerance_uv,max_pairs,max_cells,max_domain_cells,None)
}
pub(crate) fn inspect_with_boundary(model:&Model,tolerance_mm:f64,tolerance_uv:f64,
    max_pairs:usize,max_cells:usize,max_domain_cells:usize,boundary:Option<&crate::boundary_embedding::Report>)->Result<Report> {
    model.validate()?;
    let n = model.shells.len();
    if n == 0 || n > 448 || model.shells.iter().any(|s| !s.closed || s.faces.is_empty())
        || !(1..=100000).contains(&max_pairs)
        || !(2..=1000000).contains(&max_cells)
        || !(2..=8000000).contains(&max_domain_cells)
        || !tolerance_mm.is_finite() || tolerance_mm <= 0.
        || !tolerance_uv.is_finite() || tolerance_uv <= 0.
    {
        return Err(Error::new("BREP_INVALID_INPUT", "Nesting requires nonempty closed shells, positive tolerances and bounded work"));
    }
    let total_pairs = n*(n-1)/2;
    let mut out = Report { pairs: Vec::new(), total_pairs, cells: 0, domain_cells: 0, parents: None, roles_consistent: None };
    let shells = (0..n).map(|i| solid_audit::isolated_outward_shell(model,i,false)).collect::<Result<Vec<_>>>()?;
    // Index only this invocation's fresh complete proof. Keep the exact
    // separation predicates; avoid rescanning every record for every face pair.
    let separation_index = boundary.filter(|proof| proof.proven
        && proof.intersections.pairs.all_pairs_classified
        && proof.intersections.pairs.next_pair.is_none()).map(|proof| {
        let pairs = &proof.intersections.pairs;
        let individual = pairs.pairs.iter()
            .filter(|p| p.reason == "pair-disjoint" && p.boundary.is_none())
            .map(|p| (p.faces[0].min(p.faces[1]), p.faces[0].max(p.faces[1])))
            .collect::<std::collections::HashSet<_>>();
        let mut groups = std::collections::HashMap::<usize, Vec<_>>::new();
        for group in &pairs.disjoint_groups {
            if group.axis < 3 && group.range[0] > group.face
                && if group.first_before_range {
                    group.first_bounds[group.axis][1] < group.range_bounds[group.axis][0]
                } else { group.range_bounds[group.axis][1] < group.first_bounds[group.axis][0] }
            {
                groups.entry(group.face).or_default().push(group.range);
            }
        }
        (individual, groups)
    });
    let mut inside = vec![vec![false;n];n];
    for a in 0..n { for b in a+1..n {
        if out.pairs.len() == max_pairs || max_cells-out.cells < 2 || max_domain_cells-out.domain_cells < 2 {
            return Ok(out);
        }
        let remaining = total_pairs-out.pairs.len();
        let pair_cells=((max_cells-out.cells)/remaining).max(2);
        let pair_domains=((max_domain_cells-out.domain_cells)/remaining).max(2);
        let separated = separation_index.as_ref().is_some_and(|(individual, groups)| {
            model.shells[a].faces.iter().all(|fa| model.shells[b].faces.iter().all(|fb| {
                let first = fa.face.min(fb.face);
                let second = fa.face.max(fb.face);
                individual.contains(&(first, second))
                    || groups.get(&first).is_some_and(|ranges| ranges.iter()
                        .any(|range| range[0] <= second && second < range[1]))
            }))
        });
        let r=if separated {
            shell_relation::inspect_certified_boundaries(&shells[a],&shells[b],tolerance_uv,pair_cells,pair_domains)?
        } else {shell_relation::inspect(&shells[a],&shells[b],tolerance_mm,tolerance_uv,pair_cells,pair_domains)?};
        out.cells += r.boundary.cells + r.witness_parity.iter().flatten().map(|p|p.cells).sum::<usize>();
        out.domain_cells += r.boundary.domain_cells + r.witness_parity.iter().flatten().map(|p|p.domain_cells).sum::<usize>();
        out.pairs.push(Pair { shells: [a,b], result: r });
    }}
    for pair in &out.pairs {
        let [a,b] = pair.shells;
        let Some(ab) = pair.result.witness_parity[0].as_ref().and_then(|r|r.parity) else {return Ok(out)};
        let Some(ba) = pair.result.witness_parity[1].as_ref().and_then(|r|r.parity) else {return Ok(out)};
        if ab && ba { return Ok(out); }
        inside[a][b] = ab; inside[b][a] = ba;
    }
    let mut parents = vec![None;n];
    for i in 0..n {
        let containers: Vec<_> = (0..n).filter(|&j|inside[i][j]).collect();
        // Containers must form a chain, and every inclusion must be transitive.
        for &a in &containers { for b in 0..n {
            if inside[a][b] && !inside[i][b] { return Ok(out); }
        }}
        for (k,&a) in containers.iter().enumerate() { for &b in &containers[k+1..] {
            if !inside[a][b] && !inside[b][a] { return Ok(out); }
        }}
        parents[i] = containers.iter().copied().find(|&a|containers.iter().all(|&b|a==b||inside[a][b]));
        if !containers.is_empty() && parents[i].is_none() {return Ok(out)}
    }
    let mut owner = vec![None;n];
    let mut cavity = vec![false;n];
    let mut consistent = true;
    for (body_id,body) in model.bodies.iter().enumerate() {
        if owner[body.outer_shell].replace(body_id).is_some() {consistent=false;}
        for &inner in &body.inner_shells {
            if owner[inner].replace(body_id).is_some() {consistent=false;}
            cavity[inner]=true;
            consistent &= parents[inner] == Some(body.outer_shell);
        }
    }
    consistent &= owner.iter().all(Option::is_some);
    for body in &model.bodies {
        if let Some(parent) = parents[body.outer_shell] {consistent &= cavity[parent];}
    }
    out.parents = Some(parents);
    out.roles_consistent = Some(consistent);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cavity() -> Model {
        crate::operations::boolean(&crate::cuboid([0.;3],[10.;3]).unwrap(),&crate::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap()
    }
    #[test]
    fn cavity_parent_matches_ownership_and_two_nested_material_bodies_do_not() {
        let mut m = cavity();
        let outer=m.bodies[0].outer_shell;let inner=m.bodies[0].inner_shells[0];
        let r=inspect(&m,1e-5,1e-7,10,20000,200000).unwrap();
        assert_eq!(r.roles_consistent,Some(true));
        assert_eq!(r.parents.unwrap()[inner],Some(outer));
        assert!(r.cells<=20000 && r.domain_cells<=200000);
        m.bodies=vec![crate::Body{outer_shell:outer,inner_shells:vec![]},crate::Body{outer_shell:inner,inner_shells:vec![]}];
        m.rebuild_topology_ids();
        let r=inspect(&m,1e-5,1e-7,10,20000,200000).unwrap();
        assert_eq!(r.roles_consistent,Some(false));
    }
    #[test]
    fn material_island_inside_a_cavity_and_unvisited_pair_suffix() {
        let m=crate::operations::boolean(&cavity(),&crate::cuboid([3.;3],[4.;3]).unwrap(),"union").unwrap();
        assert_eq!(m.shells.len(),3);
        assert_eq!(m.bodies.len(),2);
        let r=inspect(&m,1e-5,1e-7,10,100000,1000000).unwrap();
        assert_eq!(r.roles_consistent,Some(true));
        let parents=r.parents.unwrap();
        let cavity_shell=m.bodies.iter().find_map(|b|b.inner_shells.first()).copied().unwrap();
        let island=m.bodies.iter().find(|b|b.inner_shells.is_empty()).unwrap().outer_shell;
        assert_eq!(parents[island],Some(cavity_shell));
        let partial=inspect(&m,1e-5,1e-7,1,100000,1000000).unwrap();
        assert_eq!(partial.total_pairs,3);
        assert_eq!(partial.pairs.len(),1);
        assert!(partial.parents.is_none() && partial.roles_consistent.is_none());
    }
    #[test]
    fn exhausted_work_does_not_assign_parents() {
        let m=cavity();let before=format!("{m:?}");
        let r=inspect(&m,1e-5,1e-7,10,2,2).unwrap();
        assert!(r.parents.is_none() && r.roles_consistent.is_none());
        assert!(r.cells<=2 && r.domain_cells<=2);
        assert_eq!(format!("{m:?}"),before);
    }
}
