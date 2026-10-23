use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_assert(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            parent,
            allow_assembly,
            ..
        } = *cx;
        self.eval.scoped_checks(item, scope, &current)?;
        self.emit(
            str_at(item, "input"),
            nodes,
            scope,
            &current,
            depth + 1,
            expected,
            parent,
            allow_assembly,
        )?;
        Ok(())
    }
    pub(super) fn emit_evaluate(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let shape = self
            .eval
            .resolve(&item["value"], scope, &format!("{current}/value"), 0)?;
        if let RuntimeValue::Geometry {
            function,
            scope: bound_scope,
        } = shape
        {
            let fn_value = *self.eval.functions.get(function).unwrap();
            if str_at(fn_value, "kind") != "geometry" {
                return Err(fail("type_error", "Expected a geometry function."));
            }
            let body = self.bodies.get(function).unwrap().clone();
            self.emit(
                str_at(fn_value, "root"),
                &body,
                &bound_scope,
                &format!("{current}/value:{function}"),
                depth + 1,
                expected,
                None,
                false,
            )?;
        } else {
            return Err(fail("type_error", "Expected a geometry value."));
        }
        Ok(())
    }
    pub(super) fn emit_call(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let bound = self
            .eval
            .bind(str_at(item, "function"), &item["args"], scope, &current, 0)?;
        if str_at(bound.function, "kind") != "geometry" {
            return Err(fail("type_error", "Expected a geometry function."));
        }
        let id = str_at(bound.function, "id");
        let body = self.bodies.get(id).unwrap().clone();
        self.emit(
            str_at(bound.function, "root"),
            &body,
            &bound.scope,
            &format!("{current}/call:{id}"),
            depth + 1,
            expected,
            None,
            false,
        )?;
        Ok(())
    }
    pub(super) fn emit_match(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let (arm, bound) =
            self.eval
                .select_match_arm(&item["value"], array(&item["arms"]), scope, &current, 0)?;
        self.emit(
            str_at(&item["arms"][arm], "input"),
            nodes,
            &bound,
            &format!("{current}/arms/{arm}"),
            depth + 1,
            expected,
            None,
            false,
        )?;
        Ok(())
    }
    pub(super) fn emit_if(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let condition = self.value(&item["condition"], scope, &current, "condition", SCALAR)?;
        self.emit(
            str_at(item, if condition != 0. { "then" } else { "else" }),
            nodes,
            scope,
            &current,
            depth + 1,
            expected,
            None,
            false,
        )?;
        Ok(())
    }
    pub(super) fn emit_group(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        for child in children(item) {
            self.emit(
                child,
                nodes,
                scope,
                &current,
                depth + 1,
                expected,
                None,
                false,
            )?;
        }
        Ok(())
    }
    pub(super) fn emit_collect(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let list = self
            .eval
            .resolve(&item["values"], scope, &format!("{current}/values"), 0)?;
        let items = sequence(&list, &current)?;
        if items.len() > 256 {
            return Err(fail("invalid_count", "Collection exceeds 256 elements."));
        }
        for (i, v) in items.iter().enumerate() {
            let mut local = (**scope).clone();
            local.insert(str_at(item, "binding").into(), v.clone());
            self.emit(
                str_at(item, "input"),
                nodes,
                &Rc::new(local),
                &format!("{current}[{i}]"),
                depth + 1,
                expected,
                None,
                false,
            )?;
        }
        Ok(())
    }
    pub(super) fn emit_map(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        let count = self.value(&item["count"], scope, &current, "count", SCALAR)?;
        if count.fract() != 0. || !(1.0..=256.0).contains(&count) {
            return Err(fail(
                "invalid_count",
                "Map count must be an integer from 1 to 256.",
            ));
        }
        self.lines.push("union(){".into());
        for i in 0..count as usize {
            let mut local = (**scope).clone();
            local.insert(str_at(item, "index").into(), RuntimeValue::from(i as f64));
            self.emit(
                str_at(item, "input"),
                nodes,
                &Rc::new(local),
                &format!("{current}[{i}]"),
                depth + 1,
                expected,
                None,
                false,
            )?;
        }
        self.lines.push("}".into());
        Ok(())
    }
}
