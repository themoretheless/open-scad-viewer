use super::*;

pub(super) fn len<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("len", argc, &[1])?;
    match &arg(0)? {
        Value::Str(deref!(s)) => Ok(Value::Number(s.chars().count() as f64)),
        Value::Vector(deref!(items)) => Ok(Value::Number(items.len() as f64)),
        _ => fail("len", "argument 1 must be a string or vector"),
    }
}

pub(super) fn log<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("log", argc, &[1, 2])?;
    if argc == 1 {
        return Ok(Value::Number(number_value("log", &arg(0)?, 0)?.log10()));
    }
    let base = number_value("log", &arg(0)?, 0)?;
    let value = number_value("log", &arg(1)?, 1)?;
    Ok(Value::Number(value.ln() / base.ln()))
}

pub(super) fn str_<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut out = String::new();
    for index in 0..argc {
        out.push_str(&format_builtin_value(&arg(index)?, false));
    }
    ctx.register_string("str", out)
}

pub(super) fn character_string(value: &Value, ctx: &mut BuiltinContext) -> String {
    match value {
        Value::Number(v) => {
            let code_point = v.trunc();
            if !code_point.is_finite()
                || code_point <= 0.0
                || code_point > 0x10ffff as f64
                || (code_point >= 0xd800 as f64 && code_point <= 0xdfff as f64)
            {
                return String::new();
            }
            char::from_u32(code_point as u32).map_or_else(String::new, |c| c.to_string())
        }
        Value::Vector(items) => items
            .iter()
            .map(|item| character_string(item, ctx))
            .collect(),
        Value::Range { start, step, end } => {
            let count = range_item_count(*start, *step, *end);
            if count >= MAX_CHR_RANGE_ITEMS {
                ctx.warn(
                    "chr",
                    format!(
                        "range exceeds the {}-item character limit",
                        crate::value::locale(MAX_CHR_RANGE_ITEMS as usize)
                    ),
                );
                return String::new();
            }
            // RangeType::numValues() reports one item when all three fields
            // are equal, but its 2021.01 iterator is empty for zero steps.
            if *step == 0.0 {
                return String::new();
            }
            let mut result = String::new();
            for index in 0..count as usize {
                result.push_str(&character_string(
                    &Value::Number(start + step * index as f64),
                    ctx,
                ));
            }
            result
        }
        _ => String::new(),
    }
}

pub(super) fn chr<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut out = String::new();
    for index in 0..argc {
        out.push_str(&character_string(&arg(index)?, ctx));
    }
    ctx.register_string("chr", out)
}

pub(super) fn ord<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return Ok(Value::Undef);
    }
    if argc != 1 {
        return fail("ord", "expects 1 argument");
    }
    let value = arg(0)?;
    let Value::Str(deref!(s)) = &value else {
        return fail("ord", "argument 1 must be a string");
    };
    // Rust strings are well-formed UTF-8 by construction, so the TS
    // "argument 1 must be valid Unicode" check cannot fire here.
    match s.chars().next() {
        None => Ok(Value::Undef),
        Some(first) => Ok(Value::Number(first as u32 as f64)),
    }
}

pub(super) fn concat<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    let mut result: Vec<Value<'a>> = Vec::new();
    for index in 0..argc {
        let value = arg(index)?;
        let addition = match &value {
            Value::Vector(items) => items.len(),
            _ => 1,
        };
        if result.len() + addition > MAX_GENERATED_ITEMS {
            return fail(
                "concat",
                format!(
                    "result exceeds {} items",
                    crate::value::locale(MAX_GENERATED_ITEMS)
                ),
            );
        }
        match value {
            Value::Vector(items) => result.extend(items.iter().cloned()),
            other => result.push(other),
        }
    }
    ctx.register_array("concat", result)
}

pub(super) fn lookup<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("lookup", argc, &[2])?;
    let position_value = arg(0)?;
    let Some(position) = position_value.as_number().filter(|v| v.is_finite()) else {
        // The reference diagnostic formats a second evaluation of the first
        // expression and never reads the table on this path.
        let _ = arg(0)?;
        return if position_value.as_number().is_some() {
            fail("lookup", "argument 1 must be finite")
        } else {
            fail("lookup", "argument 1 must be a number")
        };
    };
    let table = arg(1)?;
    let Value::Vector(rows) = &table else {
        return Ok(Value::Undef);
    };
    if rows.is_empty() {
        return Ok(Value::Undef);
    }
    let pair = |row: &Value| -> Option<(f64, f64)> {
        let row = row.as_vector()?;
        if row.len() != 2 {
            return None;
        }
        Some((row[0].as_number()?, row[1].as_number()?))
    };
    let Some(first) = pair(&rows[0]) else {
        return Ok(Value::Undef);
    };
    let (mut low_position, mut low_value) = first;
    let (mut high_position, mut high_value) = first;
    for row in rows.iter().skip(1) {
        let Some((candidate_position, candidate_value)) = pair(row) else {
            continue;
        };
        if candidate_position <= position
            && (candidate_position > low_position || low_position > position)
        {
            low_position = candidate_position;
            low_value = candidate_value;
        }
        if candidate_position >= position
            && (candidate_position < high_position || high_position < position)
        {
            high_position = candidate_position;
            high_value = candidate_value;
        }
    }
    if position <= low_position {
        return Ok(Value::Number(high_value));
    }
    if position >= high_position {
        return Ok(Value::Number(low_value));
    }
    let fraction = (position - low_position) / (high_position - low_position);
    Ok(Value::Number(
        high_value * fraction + low_value * (1.0 - fraction),
    ))
}

pub(super) fn unsigned_search_parameter(value: &Value) -> usize {
    match value.as_number() {
        Some(v) if v.is_finite() => {
            let truncated = v.trunc();
            if truncated < 0.0 {
                // JS `>>> 0` wraps modulo 2^32.
                (truncated as i64).rem_euclid(0x1_0000_0000) as usize
            } else {
                (truncated as u64 % 0x1_0000_0000) as usize
            }
        }
        _ => 0,
    }
}

pub(super) fn return_matches<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    matches: Vec<Value<'a>>,
    maximum: usize,
) -> BResult<Value<'a>> {
    let selected = if maximum == 0 {
        matches
    } else {
        matches.into_iter().take(maximum).collect()
    };
    ctx.register_array("search", selected)
}

pub(super) fn search_needles<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    needles: &[Value<'a>],
    table: &[Value<'a>],
    maximum: usize,
    column: usize,
    include_empty_first_match: bool,
) -> BResult<Value<'a>> {
    let mut output: Vec<Value<'a>> = Vec::new();
    for needle in needles {
        let mut matches: Vec<Value<'a>> = Vec::new();
        for (index, row) in table.iter().enumerate() {
            let hit = (column == 0 && deep_equal(needle, row))
                || match row {
                    Value::Vector(items) => {
                        column < items.len() && deep_equal(needle, &items[column])
                    }
                    _ => false,
                };
            if hit {
                matches.push(Value::Number(index as f64));
            }
            if maximum != 0 && matches.len() >= maximum {
                break;
            }
        }
        if maximum == 1 {
            if let Some(first) = matches.into_iter().next() {
                output.push(first);
            } else if include_empty_first_match {
                output.push(ctx.register_array("search", Vec::new())?);
            }
        } else {
            output.push(return_matches(ctx, matches, maximum)?);
        }
    }
    ctx.register_array("search", output)
}

pub(super) fn search_string_rows<'a>(
    ctx: &mut BuiltinContext<'_, 'a>,
    characters: Vec<char>,
    rows: &[Value<'a>],
    maximum: usize,
    column: usize,
) -> BResult<Value<'a>> {
    let mut output: Vec<Value<'a>> = Vec::new();
    for character in characters {
        let mut matches: Vec<Value<'a>> = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let Some(items) = row.as_vector() else {
                ctx.warn(
                    "search",
                    format!("table row {index} does not contain column {column}"),
                );
                return ctx.register_array("search", Vec::new());
            };
            if items.len() <= column {
                ctx.warn(
                    "search",
                    format!("table row {index} does not contain column {column}"),
                );
                return ctx.register_array("search", Vec::new());
            }
            let candidate = format_builtin_value(&items[column], false).chars().next();
            if candidate == Some(character) {
                matches.push(Value::Number(index as f64));
            }
            if maximum != 0 && matches.len() >= maximum {
                break;
            }
        }
        if matches.is_empty() {
            ctx.warn("search", format!("term {character} was not found"));
        }
        if maximum == 1 {
            if let Some(first) = matches.into_iter().next() {
                output.push(first);
            }
        } else {
            output.push(return_matches(ctx, matches, maximum)?);
        }
    }
    ctx.register_array("search", output)
}

pub(super) fn search<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc < 2 {
        return fail("search", "expects at least 2 arguments");
    }
    let needle = arg(0)?;
    let table = arg(1)?;
    let maximum = if argc > 2 {
        unsigned_search_parameter(&arg(2)?)
    } else {
        1
    };
    let column = if argc > 3 {
        unsigned_search_parameter(&arg(3)?)
    } else {
        0
    };

    if let (Value::Str(deref!(needle)), Value::Str(deref!(table))) = (&needle, &table) {
        let table_characters: Vec<char> = table.chars().collect();
        let mut output: Vec<Value<'a>> = Vec::new();
        for character in needle.chars() {
            let mut matches: Vec<Value<'a>> = Vec::new();
            for (index, candidate) in table_characters.iter().enumerate() {
                if *candidate == character {
                    matches.push(Value::Number(index as f64));
                }
                if maximum != 0 && matches.len() >= maximum {
                    break;
                }
            }
            if maximum == 1 {
                if let Some(first) = matches.into_iter().next() {
                    output.push(first);
                }
            } else {
                output.push(return_matches(ctx, matches, maximum)?);
            }
        }
        return ctx.register_array("search", output);
    }

    let rows: Vec<Value<'a>> = match &table {
        Value::Vector(items) => items.as_ref().clone(),
        _ => Vec::new(),
    };
    match &needle {
        Value::Number(_) => {
            let mut matches: Vec<Value<'a>> = Vec::new();
            for (index, row) in rows.iter().enumerate() {
                let hit = (column == 0 && deep_equal(&needle, row))
                    || match row {
                        Value::Vector(items) => {
                            column < items.len() && deep_equal(&needle, &items[column])
                        }
                        _ => false,
                    };
                if hit {
                    matches.push(Value::Number(index as f64));
                }
                if maximum != 0 && matches.len() >= maximum {
                    break;
                }
            }
            ctx.register_array("search", matches)
        }
        Value::Str(s) => search_string_rows(ctx, s.chars().collect(), &rows, maximum, column),
        Value::Vector(items) => search_needles(ctx, &items.clone(), &rows, maximum, column, true),
        _ => Ok(Value::Undef),
    }
}

pub(super) fn version<'a>(
    _argc: usize,
    _arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    // The pinned 2021.01 implementation ignores surplus arguments; the
    // official release reports an explicit zero day component.
    ctx.register_array(
        "version",
        vec![
            Value::Number(2021.0),
            Value::Number(1.0),
            Value::Number(0.0),
        ],
    )
}

pub(super) fn version_num<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return Ok(Value::Number(20210100.0));
    }
    let value = arg(0)?;
    let Some(parts) = value.as_vector() else {
        return Ok(Value::Undef);
    };
    if parts.len() != 2 && parts.len() != 3 {
        return Ok(Value::Undef);
    }
    let (Some(year), Some(month)) = (parts[0].as_number(), parts[1].as_number()) else {
        return Ok(Value::Undef);
    };
    let day = if parts.len() == 3 {
        let Some(day) = parts[2].as_number() else {
            return Ok(Value::Undef);
        };
        day
    } else {
        0.0
    };
    Ok(Value::Number(year * 10_000.0 + month * 100.0 + day))
}

pub(super) fn norm<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("norm", argc, &[1])?;
    let value = arg(0)?;
    let Some(vector) = value.as_vector() else {
        return Ok(Value::Undef);
    };
    let vector = vector.clone();
    let mut result = 0.0;
    for (index, item) in vector.iter().enumerate() {
        let value = number_value("norm", item, index)?;
        result += value * value;
    }
    Ok(Value::Number(result.sqrt()))
}

pub(super) fn cross<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("cross", argc, &[2])?;
    let left_value = arg(0)?;
    let right_value = arg(1)?;
    let left = array_value("cross", &left_value, 0)?;
    let right = array_value("cross", &right_value, 1)?;
    if left.len() != right.len() || (left.len() != 2 && left.len() != 3) {
        return fail("cross", "arguments must be matching 2D or 3D vectors");
    }
    if left.len() == 2 {
        // Historical 2021.01 behaviour coerces non-numeric 2D elements to zero.
        let n = |v: &Value| v.as_number().unwrap_or(0.0);
        return Ok(Value::Number(
            n(&left[0]) * n(&right[1]) - n(&left[1]) * n(&right[0]),
        ));
    }
    let a = [
        finite_number_value("cross", &left[0], 0)?,
        finite_number_value("cross", &left[1], 1)?,
        finite_number_value("cross", &left[2], 2)?,
    ];
    let b = [
        finite_number_value("cross", &right[0], 0)?,
        finite_number_value("cross", &right[1], 1)?,
        finite_number_value("cross", &right[2], 2)?,
    ];
    ctx.register_array(
        "cross",
        vec![
            Value::Number(a[1] * b[2] - a[2] * b[1]),
            Value::Number(a[2] * b[0] - a[0] * b[2]),
            Value::Number(a[0] * b[1] - a[1] * b[0]),
        ],
    )
}

pub(super) fn parent_module<'a>(
    argc: usize,
    arg: ArgReader<'_, 'a>,
    ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    expect_count("parent_module", argc, &[0, 1])?;
    let value = if argc == 0 {
        Value::Number(1.0)
    } else {
        arg(0)?
    };
    let Some(raw_depth) = value.as_number() else {
        return Ok(Value::Undef);
    };
    let depth = if raw_depth.is_nan() {
        0.0
    } else if raw_depth == f64::INFINITY {
        0x7fff_ffff as f64
    } else if raw_depth == f64::NEG_INFINITY {
        -0x8000_0000 as f64
    } else {
        raw_depth.trunc()
    };
    if depth < 0.0 {
        ctx.warn(
            "parent_module",
            format!(
                "negative index {} is not allowed",
                js_number_to_string(depth)
            ),
        );
        return Ok(Value::Undef);
    }
    let result = (ctx.parent_module)(depth as usize);
    match result {
        Some(name) => Ok(Value::string(name)),
        None => {
            ctx.warn(
                "parent_module",
                format!(
                    "index {} is outside the active module stack",
                    js_number_to_string(depth)
                ),
            );
            Ok(Value::Undef)
        }
    }
}

pub(super) fn min_max<'a>(
    name: &'static str,
    argc: usize,
    arg: ArgReader<'_, 'a>,
    _ctx: &mut BuiltinContext<'_, 'a>,
) -> BResult<Value<'a>> {
    if argc == 0 {
        return fail(name, "expects at least 1 argument");
    }
    let first = arg(0)?;
    if argc == 1
        && let Value::Vector(values) = &first
    {
        if values.is_empty() {
            return fail(name, "expects at least 1 vector element");
        }
        let mut selected = values[0].clone();
        for value in values.iter().skip(1) {
            let compared = if name == "min" {
                compare_values(value, &selected)
            } else {
                compare_values(&selected, value)
            };
            if compared.is_some_and(|c| c < 0.0) {
                selected = value.clone();
            }
        }
        return Ok(selected);
    }
    let mut selected = number_value(name, &first, 0)?;
    for index in 1..argc {
        let value = number_value(name, &arg(index)?, index)?;
        if (name == "min" && value < selected) || (name == "max" && value > selected) {
            selected = value;
        }
    }
    Ok(Value::Number(selected))
}

pub(super) fn predicate<'a>(
    name: &'static str,
    test: fn(&Value<'a>) -> bool,
) -> impl Fn(usize, ArgReader<'_, 'a>, &mut BuiltinContext<'_, 'a>) -> BResult<Value<'a>> {
    move |argc, arg, _ctx| {
        expect_count(name, argc, &[1])?;
        Ok(Value::Bool(test(&arg(0)?)))
    }
}
