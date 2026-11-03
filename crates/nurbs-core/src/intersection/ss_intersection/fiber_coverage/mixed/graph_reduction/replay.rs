use super::*;
impl Proof {
    pub(in crate::intersection::ss_intersection::fiber_coverage::mixed) fn verify(
        &self,
        c: &QualifiedMixedFiberCoverage,
        work: &mut Work,
    ) -> Result<()> {
        check(
            c.nodes.is_empty()
                && c.domain == natural(c.fibers[0].source())
                && !self.nodes.is_empty()
                && self.nodes.len() <= c.limits.max_cells,
            "Reduced original product census changed",
        )?;
        let patches = derive_patches(c.fibers[0].source(), work)?;
        check(
            patches.len() == self.patch_roots,
            "Original graph knot census changed",
        )?;
        for f in &c.fibers {
            replay_fiber(f, work)?;
        }
        let mut pending: Vec<_> = patches
            .into_iter()
            .enumerate()
            .map(|(i, (cell, net))| (i, cell, net, 0))
            .collect();
        let mut seen = vec![false; self.nodes.len()];
        let mut excludes = 0;
        let mut covered = 0;
        while let Some((id, cell, net, depth)) = pending.pop() {
            tick(work)?;
            check(
                id < self.nodes.len() && !seen[id] && depth <= c.limits.max_depth,
                "Reduced coverage omitted, cycled or exceeded cells",
            )?;
            seen[id] = true;
            match self.nodes[id] {
                Node::Chart(index) => {
                    check(
                        index < c.fibers.len() && contained(&cell, c.domain, &c.fibers[index])?,
                        "Reduced roots leave their original fiber",
                    )?;
                    covered += 1;
                }
                Node::Image { index, axis } => {
                    let roots = root_image(&cell, &net, axis).ok_or_else(|| {
                        crate::input("Reduced graph derivative/zero enclosure changed")
                    })?;
                    check(
                        index < c.fibers.len()
                            && contained_image(&roots, c.domain, &c.fibers[index])?,
                        "Reduced graph zero image leaves original fiber",
                    )?;
                    covered += 1;
                }
                Node::Excluded => {
                    check(excluded(&net), "Reduced exact exclusion changed")?;
                    excludes += 1;
                }
                Node::Split { axis, children } => {
                    check(
                        children[0] != children[1]
                            && children.iter().all(|&i| i < self.nodes.len()),
                        "Invalid reduced closed children",
                    )?;
                    let cells = cut(&cell, axis)?;
                    let nets = split(&net, axis);
                    for k in (0..2).rev() {
                        pending.push((children[k], cells[k].clone(), nets[k].clone(), depth + 1));
                    }
                }
            }
        }
        check(
            seen.iter().all(|v| *v)
                && covered > 0
                && c.report == report(self.nodes.len(), excludes, work.used),
            "Reduced original graph report changed",
        )
    }
}
