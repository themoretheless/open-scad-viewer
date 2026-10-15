use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn value(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<f64> {
        self.eval
            .field(expr, scope, &format!("{current}/{field}"), dimension)
    }
    pub(super) fn vector(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<Vec<f64>> {
        if !expr.is_array() {
            let path = format!("{current}/{field}");
            let value = self.eval.resolve(expr, scope, &path, 0)?;
            return sequence(&value, &path)?
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let path = format!("{path}/{index}");
                    crate::units::field(
                        numeric_value(value, &path)?,
                        dimension,
                        &path,
                        self.eval.strict,
                    )
                })
                .collect();
        }
        array(expr)
            .iter()
            .enumerate()
            .map(|(i, v)| self.value(v, scope, current, &format!("{field}/{i}"), dimension))
            .collect()
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn geometry_vector(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
        size: usize,
    ) -> Result<Vec<f64>> {
        let values = self.vector(expr, scope, current, field, dimension)?;
        if values.len() != size {
            return Err(Error::new(
                "type_error",
                format!("{current}/{field}"),
                format!(
                    "Expected vector with {size} components, received {}.",
                    values.len()
                ),
            ));
        }
        Ok(values)
    }
    pub(super) fn frame(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
    ) -> Result<Value> {
        Ok(
            json!({"origin":self.vector(&expr["origin"],scope,current,&format!("{field}/origin"),LENGTH)?,"rotation":self.vector(&expr["rotation"],scope,current,&format!("{field}/rotation"),ANGLE)?}),
        )
    }
    pub(super) fn child(
        &mut self,
        item: &'a Value,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        current: &str,
        depth: usize,
        expected: GeometryType,
    ) -> Result<()> {
        self.emit(
            str_at(item, "input"),
            nodes,
            scope,
            current,
            depth + 1,
            expected,
            None,
            false,
        )
    }
}
