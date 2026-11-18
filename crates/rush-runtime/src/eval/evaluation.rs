use super::*;

impl<'a> Evaluator<'a> {
    pub fn new(document: &'a Json) -> Result<Self> {
        let mut parameters = HashMap::new();
        for (i, parameter) in array(&document["parameters"]).iter().enumerate() {
            let id = string(&parameter["id"]);
            if parameters.contains_key(id) {
                return Err(Error::new(
                    "duplicate_id",
                    format!("/parameters/{i}/id"),
                    "Parameter IDs must be unique.",
                ));
            }
            let path = format!("/parameters/{i}");
            let value = number(&parameter["value"]);
            let min = parameter.get("min").and_then(Json::as_f64);
            let max = parameter.get("max").and_then(Json::as_f64);
            if min.zip(max).is_some_and(|(a, b)| a > b)
                || min.is_some_and(|min| value < min)
                || max.is_some_and(|max| value > max)
                || (parameter["integer"] == true && value.fract() != 0.0)
            {
                return Err(Error::new(
                    "parameter_constraint",
                    path,
                    format!("Parameter {id} violates its declared bounds or integer requirement."),
                ));
            }
            parameters.insert(
                id,
                if let Some(unit) = parameter.get("unit") {
                    units::quantity(value, string(unit), &path)?
                } else {
                    value.into()
                },
            );
        }
        let mut functions = HashMap::new();
        for function in array(&document["functions"]) {
            functions.insert(string(&function["id"]), function);
        }
        if functions.len() != array(&document["functions"]).len() {
            return Err(Error::new(
                "duplicate_id",
                "/functions",
                "Function IDs must be unique.",
            ));
        }
        // Preserve source-order error reporting even though lookup uses a hash table.
        for function in array(&document["functions"]) {
            if !unique(array(&function["parameters"])) {
                return Err(Error::new(
                    "duplicate_id",
                    format!("/functions/{}", string(&function["id"])),
                    "Function parameters must be unique.",
                ));
            }
        }
        Ok(Self {
            global_scope: Scope::default(),
            memo: HashMap::new(),
            active_memo: HashSet::new(),
            geometry_check_count: 0,
            geometry_effect_count: 0,
            strict: document["type_policy"] == "strict",
            functions,
            parameters,
            document,
            steps: 0,
            allocated: 0,
            pending_geometry_checks: vec![],
            constraint_report: vec![],
            suppress_checks: false,
            expanded_checks: 0,
        })
    }
    pub(super) fn allocate(&mut self, count: usize, path: &str) -> Result<()> {
        self.allocated += count;
        if self.allocated > 16384 {
            return Err(Error::new(
                "allocation_limit",
                path,
                "List allocation budget 16384 exceeded.",
            ));
        }
        Ok(())
    }
    pub(super) fn tick(&mut self, depth: usize, path: &str) -> Result<()> {
        // JS short-circuits before incrementing steps when depth is exceeded.
        if depth > 32 {
            return Err(Error::new(
                "evaluation_limit",
                path,
                "Evaluation depth 32 or step budget 100000 exceeded.",
            ));
        }
        self.steps += 1;
        if self.steps > 100_000 {
            return Err(Error::new(
                "evaluation_limit",
                path,
                "Evaluation depth 32 or step budget 100000 exceeded.",
            ));
        }
        Ok(())
    }
    pub fn bind(
        &mut self,
        name: &str,
        args: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<Bound<'a>> {
        let function = *self.functions.get(name).ok_or_else(|| {
            Error::new(
                "unknown_function",
                path,
                format!("Unknown function {name}."),
            )
        })?;
        let parameters = array(&function["parameters"]);
        let Some(args) = args.as_object() else {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Arguments must exactly match function parameters.",
            ));
        };
        if args.len() != parameters.len()
            || parameters.iter().any(|key| !args.contains_key(string(key)))
        {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Arguments must exactly match function parameters.",
            ));
        }
        let mut bound = HashMap::with_capacity(parameters.len());
        for key in parameters {
            let key = string(key);
            bound.insert(
                key.to_owned(),
                self.resolve(&args[key], scope, &format!("{path}/args/{key}"), depth + 1)?,
            );
        }
        Ok(Bound {
            function,
            scope: Rc::new(bound),
        })
    }
    pub fn invoke(
        &mut self,
        value: &Value<'a>,
        args: Vec<Value<'a>>,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        let Value::Closure {
            parameters,
            body,
            scope,
        } = value
        else {
            return Err(Error::new("type_error", path, "Expected a function value."));
        };
        if args.len() != parameters.len() {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Lambda arity mismatch.",
            ));
        }
        let mut bound = (**scope).clone();
        for (key, value) in parameters.iter().zip(args) {
            bound.insert(string(key).to_owned(), value);
        }
        self.resolve(body, &Rc::new(bound), &format!("{path}/apply"), depth + 1)
    }
    pub fn evaluate(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<f64> {
        numeric(&self.resolve(expr, scope, path, depth)?, path)
    }
    pub fn field(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        expected: Dimension,
    ) -> Result<f64> {
        units::field(
            numeric_value(&self.resolve(expr, scope, path, 0)?, path)?,
            expected,
            path,
            self.strict,
        )
    }
}
