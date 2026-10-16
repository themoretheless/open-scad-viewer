use super::*;
impl<'a> Evaluator<'a> {
    pub(super) fn eval_node_cases_4(&self, node: &'a CallNode, ctx: Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
        match node.name.as_str() {
            "child" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "children()");
                let index_value = match node.call_arguments.first() {
                    None => Value::Number(0.0),
                    Some(argument) => self.eval_expression(&argument.value, &ctx, 0)?,
                };
                let index = match index_value.as_number() {
                    Some(v) if v.is_finite() => v.trunc(),
                    _ => 0.0,
                };
                let index = if index == 0.0 { 0.0 } else { index };
                if index < 0.0 {
                    self.warn(format!(
                        "Negative child index ({}) is not allowed",
                        js_number_to_string(index)
                    ));
                    return Ok(Vec::new());
                }
                let (all, child_context) = self.passed_call_children(&ctx);
                let Some(statement) = all.get(index as usize) else {
                    if ctx.call_children.is_some() {
                        self.warn(format!(
                            "Child index ({}) out of bounds ({} children)",
                            js_number_to_string(index),
                            all.len()
                        ));
                    }
                    return Ok(Vec::new());
                };
                self.eval_nodes(std::slice::from_ref(statement), &child_context, true)
            }
            _ => unreachable!("selected native node family"),
        }
    }
}
