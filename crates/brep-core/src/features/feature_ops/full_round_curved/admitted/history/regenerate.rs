//! Edit an earlier authored selection and regenerate its actual descendants.
use super::*;
impl History {
    /// Replaces a root/reselection parameter, preserving every saved source
    /// model and subsequent explicit selection. Source-rebase steps inherit
    /// the preceding selection through authored IDs, never face indices.
    /// Hidden redo is regenerated too, and the original cursor is retained.
    /// This is not replay of source-construction expressions or a graph merge.
    pub fn regenerate_selection(
        &mut self,
        revision: usize,
        selected: [TopoId; 3],
    ) -> Result<&AdmittedFullRound> {
        let target = self
            .revisions
            .get(revision)
            .ok_or_else(|| error("Exact history parameter revision is missing"))?;
        if target.edit == Edit::RebaseSource {
            return Err(error("Source rebase has no authored selection parameter"));
        }
        let mut next = Self {
            revisions: self.revisions[..revision].to_vec(),
            cursor: self.cursor,
            budget: self.budget,
            work: Work::default(),
        };
        for row in &next.revisions {
            next.work = add(next.work, used(row.body()))?;
        }
        for (index, row) in self.revisions.iter().enumerate().skip(revision) {
            let choices = if index == revision {
                selected
            } else if row.edit == Edit::RebaseSource {
                ids(next
                    .revisions
                    .last()
                    .ok_or_else(|| error("Missing regeneration parent"))?
                    .body())
            } else {
                ids(row.body())
            };
            let source = row.body().source();
            let faces = resolve(source, choices)?;
            let body = build_admitted(source, faces, next.budget.remaining(next.work)?)?;
            next.work = add(next.work, used(&body))?;
            next.budget.remaining(next.work)?;
            next.revisions.push(Revision {
                edit: row.edit,
                body: Arc::new(body),
            });
        }
        next.check_storage()?;
        *self = next;
        Ok(self.current())
    }
}
