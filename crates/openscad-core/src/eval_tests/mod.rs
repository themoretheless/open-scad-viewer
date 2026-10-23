//! Evaluator unit tests (migration stage 2): values, scopes, statements,
//! argument binding, echo/assert, list comprehensions, built-ins and budgets,
//! in both language profiles. Expectations are transcribed from the TypeScript
//! evaluator semantics (`openscadParser.ts`, `openScadValueSemantics.ts`,
//! `openScadBuiltinFunctions.ts`).
use crate::LanguageProfile;
use crate::eval::{Evaluation, EvaluatorOptions, MAX_EVAL_OPS, evaluate_source};
use crate::value::{
    Value, compare_values, deep_equal, format_number, format_value, js_number_to_string,
    js_to_precision, truthy,
};

const SUBSET: LanguageProfile = LanguageProfile::ViewerSubset;
const STABLE: LanguageProfile = LanguageProfile::Stable2021;

mod big_stack;
mod stable_ranges_and_undef_semantics;
use big_stack::*;

mod geometry_recording;
