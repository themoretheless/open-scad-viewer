use super::*;

impl<'a> Evaluator<'a> {
    pub(in crate::eval) fn emit_geometry(
        &self,
        name: &str,
        node: geometry_ops::solid_program::Node,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let mut shape = descriptor(name, 3);
        if self.record_geometry {
            shape.geometry = Some(self.store_geometry(node, p)?)
        }
        Ok(vec![shape])
    }
}
