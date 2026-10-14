use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Preview,
    Full,
}

/// Host knobs for one evaluation. `should_abort` is the synchronous
/// cancellation flag (TS `shouldAbort` polled at statement/loop checkpoints).
pub struct EvaluatorOptions<'h> {
    pub record_geometry: bool,
    pub should_abort: Option<&'h dyn Fn() -> bool>,
    /// Unseeded `rands` stream. TS uses `Math.random`; the deterministic
    /// default here is a splitmix64 stream (seeded `rands` is exact either way).
    pub random: Option<&'h RefCell<dyn FnMut() -> f64>>,
    /// Host animation position exposed as `$t` (0..1).
    pub animation_time: f64,
    pub quality: Quality,
}

impl Default for EvaluatorOptions<'_> {
    fn default() -> Self {
        Self {
            record_geometry: false,
            should_abort: None,
            random: None,
            animation_time: 0.0,
            quality: Quality::Full,
        }
    }
}

/// Result of a successful evaluation.
pub struct Evaluation {
    pub geometry: Option<geometry_ops::solid_program::Program>,
    pub shapes: Vec<ShapeDescriptor>,
    pub warnings: Vec<String>,
    pub reduced: bool,
}

#[derive(Clone)]
pub(super) enum CallChildren<'a> {
    List(&'a [Statement]),
    Passed(Rc<PassedCallChildren<'a>>),
}

pub(super) struct PassedCallChildren<'a> {
    pub(super) statements: &'a [Statement],
    pub(super) env: Env<'a>,
    pub(super) scope: Option<Rc<StableScope<'a>>>,
    pub(super) continuation: Option<CallChildren<'a>>,
}

/// Immutable evaluation context, cloned along evaluation paths exactly like
/// the TS `{...ctx, ...}` spreads; shared mutable state lives in `Evaluator`.
#[derive(Clone)]
pub(super) struct Ctx<'a> {
    pub(super) env: Env<'a>,
    pub(super) stable_scope: Option<Rc<StableScope<'a>>>,
    pub(super) scope_visible_before: usize,
    pub(super) call_children: Option<CallChildren<'a>>,
    pub(super) function_stack: Rc<Vec<String>>,
    pub(super) module_stack: Rc<Vec<String>>,
    pub(super) depth: usize,
    pub(super) viewport_root_locked: bool,
    pub(super) viewport_root_owner: Option<*const CallNode>,
}

impl<'a> Ctx<'a> {
    pub(super) fn with_env(&self, env: HashMap<String, Value<'a>>) -> Self {
        Self {
            env: env_of(env),
            ..self.clone()
        }
    }
    pub(super) fn push_function(&self, name: &str) -> Rc<Vec<String>> {
        let mut stack = (*self.function_stack).clone();
        stack.push(name.to_string());
        Rc::new(stack)
    }
    pub(super) fn push_module(&self, name: &str) -> Rc<Vec<String>> {
        let mut stack = (*self.module_stack).clone();
        stack.push(name.to_string());
        Rc::new(stack)
    }
}

/// Shared evaluator state (one per `evaluate` call).
pub struct Evaluator<'a> {
    pub(super) record_geometry: bool,
    pub(super) geometry: RefCell<Vec<geometry_ops::solid_program::Node>>,
    pub(super) profiles: RefCell<Vec<geometry_ops::profile_program::Node>>,
    pub(super) units: &'a [u16],
    pub(super) profile: LanguageProfile,
    pub(super) quality: Quality,
    pub(super) ops: Cell<u64>,
    pub(super) value_budget: Cell<u64>,
    pub(super) value_meta: RefCell<HashMap<*const Vec<Value<'a>>, (u64, u32)>>,
    pub(super) warnings: RefCell<Vec<String>>,
    pub(super) functions: HashMap<String, &'a FunctionNode>,
    pub(super) modules: HashMap<String, &'a ModuleNode>,
    pub(super) reduced: Cell<bool>,
    pub(super) should_abort: Option<&'a dyn Fn() -> bool>,
    pub(super) random_host: Option<&'a RefCell<dyn FnMut() -> f64>>,
    pub(super) animation_time: f64,
    pub(super) random_state: Cell<u64>,
}

pub(super) fn has_modifier(node: &CallNode, kind: &str) -> bool {
    node.viewport_modifiers.iter().any(|m| m.kind == kind)
}

/// Deterministic splitmix64 stream standing in for the host's unseeded
/// `Math.random` (TS only exposes it through unseeded `rands`, which is
/// nondeterministic there as well).
pub(super) fn splitmix64(state: u64) -> (u64, f64) {
    let next = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = next;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (next, (z >> 11) as f64 / 0x20_0000_0000_0000u64 as f64)
}
