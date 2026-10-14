// Sequence query operations share immutable list elements. Sorting evaluates
// each key once; no user callback runs inside the comparator.
fn query_compare(a: &Value<'_>, b: &Value<'_>, path: &str) -> Result<std::cmp::Ordering> {
    match (a, b) {
        (Value::Numeric(a), Value::Numeric(b)) => {
            units::equal(*a, *b, path)?;
            Ok(a.value
                .partial_cmp(&b.value)
                .unwrap_or(std::cmp::Ordering::Equal))
        }
        (Value::Text(a), Value::Text(b)) => Ok(a.cmp(b)),
        (Value::Record(a), Value::Record(b)) => {
            let mut ak: Vec<_> = a.keys().collect();
            ak.sort();
            let mut bk: Vec<_> = b.keys().collect();
            bk.sort();
            let order = ak.cmp(&bk);
            if !order.is_eq() {
                return Ok(order);
            }
            for key in ak {
                let order = query_compare(&a[key], &b[key], path)?;
                if !order.is_eq() {
                    return Ok(order);
                }
            }
            Ok(std::cmp::Ordering::Equal)
        }
        (Value::List(a), Value::List(b)) => {
            for (a, b) in a.iter().zip(b.iter()) {
                let order = query_compare(a, b, path)?;
                if !order.is_eq() {
                    return Ok(order);
                }
            }
            Ok(a.len().cmp(&b.len()))
        }
        _ => Err(Error::new(
            "type_error",
            path,
            "Query keys must be numbers or lists of comparable values.",
        )),
    }
}
impl<'a> Evaluator<'a> {
    fn invoke_indexed(
        &mut self,
        function: &Value<'a>,
        item: Value<'a>,
        index: usize,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        let indexed = matches!(function,Value::Closure{parameters,..} if parameters.len()==2);
        self.invoke(
            function,
            if indexed {
                vec![item, (index as f64).into()]
            } else {
                vec![item]
            },
            path,
            depth,
        )
    }

    fn query(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        let method = string(&expr["method"]);
        let value = self.resolve(&expr["input"], scope, path, depth + 1)?;
        let items = sequence(&value, path)?;
        let function = expr
            .get("function")
            .map(|f| self.resolve(f, scope, path, depth + 1))
            .transpose()?;
        let argument = expr
            .get("argument")
            .map(|a| self.resolve(a, scope, path, depth + 1))
            .transpose()?;
        let fail = |message: &str| Error::new("query_error", path, format!("{method}: {message}"));
        if method == "orderBy" {
            let keys = array(&expr["keys"]);
            let mut rows: Vec<(Value<'a>, Vec<Value<'a>>)> = Vec::with_capacity(items.len());
            self.allocate(items.len() * (keys.len() + 1), path)?;
            let functions = keys
                .iter()
                .map(|key| self.resolve(&key["function"], scope, path, depth + 1))
                .collect::<Result<Vec<_>>>()?;
            for (index, item) in items.iter().enumerate() {
                let mut values = Vec::with_capacity(keys.len());
                for f in &functions {
                    values.push(self.invoke_indexed(f, item.clone(), index, path, depth + 1)?);
                }
                if let Some(first) = rows.first() {
                    for (a, b) in first.1.iter().zip(&values) {
                        query_compare(a, b, path)?;
                    }
                }
                rows.push((item.clone(), values));
            }
            // Stable sorting preserves source order when all keys are equal.
            let mut comparison_error = None;
            rows.sort_by(|a, b| {
                for (i, (a, b)) in a.1.iter().zip(&b.1).enumerate() {
                    let mut order = match query_compare(a, b, path) {
                        Ok(order) => order,
                        Err(error) => {
                            comparison_error = Some(error);
                            std::cmp::Ordering::Equal
                        }
                    };
                    if keys[i]["descending"] == true {
                        order = order.reverse();
                    }
                    if !order.is_eq() {
                        return order;
                    }
                }
                std::cmp::Ordering::Equal
            });
            if let Some(error) = comparison_error {
                return Err(error);
            }
            return Ok(Value::List(rows.into_iter().map(|r| r.0).collect()));
        }
        let mut output = Vec::new();
        match method {
            "join" | "groupJoin" => {
                let other = sequence(
                    argument
                        .as_ref()
                        .ok_or_else(|| fail("requires an inner sequence"))?,
                    path,
                )?;
                let fs = array(&expr["functions"]);
                if fs.len() != 3 {
                    return Err(fail("requires outer key, inner key and result selector"));
                }
                let fs = fs
                    .iter()
                    .map(|f| self.resolve(f, scope, path, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                self.allocate(other.len(), path)?;
                let mut keys = Vec::with_capacity(other.len());
                for (i, item) in other.iter().enumerate() {
                    keys.push(self.invoke_indexed(&fs[1], item.clone(), i, path, depth + 1)?);
                }
                for (i, item) in items.iter().enumerate() {
                    let key = self.invoke_indexed(&fs[0], item.clone(), i, path, depth + 1)?;
                    let mut group = Vec::new();
                    for (key2, inner) in keys.iter().zip(other) {
                        self.tick(depth, path)?;
                        if !query_compare(&key, key2, path)?.is_eq() {
                            continue;
                        }
                        if method == "groupJoin" {
                            group.push(inner.clone());
                        } else {
                            output.push(self.invoke(
                                &fs[2],
                                vec![item.clone(), inner.clone()],
                                path,
                                depth + 1,
                            )?);
                        }
                        if output.len() > 256 {
                            return Err(fail("result exceeds 256 elements"));
                        }
                    }
                    if method == "groupJoin" {
                        self.allocate(group.len(), path)?;
                        output.push(self.invoke(
                            &fs[2],
                            vec![item.clone(), Value::List(group.into())],
                            path,
                            depth + 1,
                        )?);
                    }
                }
            }
            "scan" => {
                let f = function
                    .as_ref()
                    .ok_or_else(|| fail("requires an accumulator function"))?;
                let mut acc = argument.ok_or_else(|| fail("requires an initial value"))?;
                for item in items {
                    acc = self.invoke(f, vec![acc, item.clone()], path, depth + 1)?;
                    output.push(acc.clone());
                }
            }
            "defaultIfEmpty" => {
                if items.is_empty() {
                    output.push(argument.unwrap_or(0.0.into()));
                } else {
                    output.extend_from_slice(items);
                }
            }

            "length" | "count" if function.is_none() => return Ok((items.len() as f64).into()),
            "any" if function.is_none() => return Ok(((!items.is_empty()) as u8 as f64).into()),
            "take" | "skip" | "chunk" | "window" => {
                let n = numeric(
                    argument.as_ref().ok_or_else(|| fail("requires a count"))?,
                    path,
                )?;
                if n.fract() != 0.0
                    || !(0.0..=256.0).contains(&n)
                    || (n == 0.0 && ["chunk", "window"].contains(&method))
                {
                    return Err(fail(
                        "count must be an integer in 0..256 (positive for chunk/window)",
                    ));
                }
                let n = n as usize;
                match method {
                    "take" => output.extend_from_slice(&items[..n.min(items.len())]),
                    "skip" => output.extend_from_slice(&items[n.min(items.len())..]),
                    _ => {
                        let batches: Vec<&[Value<'a>]> = if method == "chunk" {
                            items.chunks(n).collect()
                        } else {
                            items.windows(n).collect()
                        };
                        self.allocate(batches.iter().map(|x| x.len()).sum(), path)?;
                        output.extend(batches.into_iter().map(|x| Value::List(x.to_vec().into())));
                    }
                }
            }
            "reverse" => output.extend(items.iter().rev().cloned()),
            "concat" | "append" | "prepend" => {
                let arg = argument.ok_or_else(|| fail("requires an argument"))?;
                if method == "prepend" {
                    output.push(arg.clone());
                }
                output.extend_from_slice(items);
                if method == "append" {
                    output.push(arg);
                } else if method == "concat" {
                    output.extend_from_slice(sequence(&arg, path)?);
                }
            }
            "flatten" => {
                for item in items {
                    output.extend_from_slice(sequence(item, path)?);
                    if output.len() > 256 {
                        return Err(fail("result exceeds 256 elements"));
                    }
                }
            }
            "contains" => {
                let arg = argument.ok_or_else(|| fail("requires an argument"))?;
                for item in items {
                    if query_compare(item, &arg, path)?.is_eq() {
                        return Ok(1.0.into());
                    }
                }
                return Ok(0.0.into());
            }
            "distinct" | "distinctBy" | "union" | "intersect" | "except" | "groupBy" => {
                let mut keys: Vec<Value<'a>> = Vec::new();
                let mut groups: Vec<Vec<Value<'a>>> = Vec::new();
                let other = argument
                    .as_ref()
                    .map(|a| sequence(a, path))
                    .transpose()?
                    .unwrap_or(&[]);
                let all = items.iter().chain(if method == "union" {
                    other.iter()
                } else {
                    [].iter()
                });
                for (index, item) in all.enumerate() {
                    self.tick(depth, path)?;
                    let key = if let Some(f) = &function {
                        self.invoke_indexed(f, item.clone(), index, path, depth + 1)?
                    } else {
                        item.clone()
                    };
                    if ["except", "intersect"].contains(&method) {
                        let mut found = false;
                        for other in other {
                            if query_compare(&key, other, path)?.is_eq() {
                                found = true;
                                break;
                            }
                        }
                        if found != (method == "intersect") {
                            continue;
                        }
                    }
                    let mut found = None;
                    for (i, k) in keys.iter().enumerate() {
                        if query_compare(k, &key, path)?.is_eq() {
                            found = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = found {
                        if method == "groupBy" {
                            groups[i].push(item.clone());
                        }
                    } else {
                        keys.push(key);
                        groups.push(vec![item.clone()]);
                        output.push(item.clone());
                    }
                }
                if method == "groupBy" {
                    self.allocate(items.len() + keys.len() * 2, path)?;
                    output = keys
                        .into_iter()
                        .zip(groups)
                        .map(|(key, values)| {
                            Value::Record(Rc::new(HashMap::from([
                                ("key".into(), key),
                                ("items".into(), Value::List(values.into())),
                            ])))
                        })
                        .collect();
                }
            }
            "takeWhile" | "skipWhile" => {
                let f = function
                    .as_ref()
                    .ok_or_else(|| fail("requires a predicate"))?;
                let mut split = 0;
                for (index, item) in items.iter().enumerate() {
                    if numeric(
                        &self.invoke_indexed(f, item.clone(), index, path, depth + 1)?,
                        path,
                    )? == 0.0
                    {
                        break;
                    }
                    split += 1;
                }
                output.extend_from_slice(if method == "takeWhile" {
                    &items[..split]
                } else {
                    &items[split..]
                });
            }
            "any" | "all" | "count" | "first" | "last" | "single" | "firstOrDefault"
            | "lastOrDefault" => {
                let mut found = None;
                let mut count = 0;
                for (index, item) in items.iter().enumerate() {
                    let passes = if let Some(f) = &function {
                        numeric(
                            &self.invoke_indexed(f, item.clone(), index, path, depth + 1)?,
                            path,
                        )? != 0.0
                    } else {
                        true
                    };
                    if method == "all" && !passes {
                        return Ok(0.0.into());
                    }
                    if !passes {
                        continue;
                    }
                    count += 1;
                    if method == "any" {
                        return Ok(1.0.into());
                    }
                    if ["first", "firstOrDefault"].contains(&method) {
                        return Ok(item.clone());
                    }
                    if method == "single" && count > 1 {
                        return Err(fail("more than one matching element"));
                    }
                    found = Some(item.clone());
                }
                return match method {
                    "any" => Ok(0.0.into()),
                    "all" => Ok(1.0.into()),
                    "count" => Ok((count as f64).into()),
                    "firstOrDefault" | "lastOrDefault" => {
                        Ok(found.or(argument).unwrap_or(0.0.into()))
                    }
                    _ => found.ok_or_else(|| fail("no matching element")),
                };
            }
            "sum" | "min" | "max" | "average" => {
                let mut total: Option<Numeric> = None;
                for (index, item) in items.iter().enumerate() {
                    let item = if let Some(f) = &function {
                        self.invoke_indexed(f, item.clone(), index, path, depth + 1)?
                    } else {
                        item.clone()
                    };
                    let n = numeric_value(&item, path)?;
                    total = Some(match total {
                        None => n,
                        Some(old) => units::binary(
                            match method {
                                "min" => "min",
                                "max" => "max",
                                _ => "add",
                            },
                            old,
                            n,
                            path,
                        )?,
                    });
                }
                let total = match total {
                    Some(n) => n,
                    None if method == "sum" => 0.0.into(),
                    None => return Err(fail("empty sequence")),
                };
                return Ok(if method == "average" {
                    units::binary("divide", total, (items.len() as f64).into(), path)?
                } else {
                    total
                }
                .into());
            }
            _ => return Err(fail("unknown operation")),
        }
        if output.len() > 256 {
            return Err(fail("result exceeds 256 elements"));
        }
        self.allocate(output.len(), path)?;
        Ok(Value::List(output.into()))
    }
}
