use super::*;
impl Proof {
    pub(in crate::intersection::ss_intersection::fiber_coverage::mixed) fn verify(
        &self,
        c: &QualifiedMixedFiberCoverage,
        work: &mut Work,
    ) -> Result<()> {
        let support = natural(c.fibers[0].source());
        let entire = c.domain == support;
        check(
            c.nodes.is_empty()
                && !self.nodes.is_empty()
                && self.nodes.len() <= c.limits.max_cells,
            "Reduced original product census changed",
        )?;
        let mut product = GraphProduct::prepare(c.fibers[0].source(), c.domain, work)?;
        let patches = std::mem::take(&mut product.patches);
        check(
            patches.len() == self.patch_roots,
            "Original graph knot census changed",
        )?;
        for f in &c.fibers {
            replay_fiber(f, work)?;
        }
        let charts = c
            .fibers
            .iter()
            .map(|fiber| product.chart(fiber.domain()))
            .collect::<Result<Vec<_>>>()?;
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
                        index < charts.len() && contained(&cell, &charts[index]),
                        "Reduced roots leave their original fiber",
                    )?;
                    covered += 1;
                }
                Node::Image { index, axis } => {
                    let roots = root_image(&cell, &net, axis).ok_or_else(|| {
                        crate::input("Reduced graph derivative/zero enclosure changed")
                    })?;
                    check(
                        index < charts.len() && contained(&roots, &charts[index]),
                        "Reduced graph zero image leaves original fiber",
                    )?;
                    covered += 1;
                }
                Node::Excluded => {
                    check(excluded(&net), "Reduced exact exclusion changed")?;
                    excludes += 1;
                }
                Node::ImageExcluded { axis } => {
                    check(
                        root_image(&cell, &net, axis).as_ref().is_some_and(empty_image),
                        "Reduced graph zero image exclusion changed",
                    )?;
                    excludes += 1;
                }
                Node::Split { axis, ref at, children } => {
                    check(
                        children[0] != children[1]
                            && children.iter().all(|&i| i < self.nodes.len()),
                        "Invalid reduced closed children",
                    )?;
                    let (cells, nets) = split_cell(&cell, &net, axis, at)?;
                    for k in (0..2).rev() {
                        pending.push((children[k], cells[k].clone(), nets[k].clone(), depth + 1));
                    }
                }
            }
        }
        check(
            seen.iter().all(|v| *v)
                && covered > 0
                && c.report == report(self.nodes.len(), excludes, work.used, entire),
            "Reduced original graph report changed",
        )
    }
}
