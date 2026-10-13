use super::*;

pub fn print_fit(v: Value) -> Result<Value> {
    let limit_opening = match v.get("limitOpening") {
        None => false,
        Some(value) => value
            .as_bool()
            .ok_or_else(|| input("limitOpening must be boolean."))?,
    };
    let options = field::<Value>(&v, "options")?;
    let settings = field::<Value>(&v, "settings")?;
    if !options.is_object() || !settings.is_object() {
        return Err(input(PRINT_SETTINGS_MESSAGE));
    }
    let nozzle = print_setting(&settings, "nozzle")?;
    let layer = print_setting(&settings, "layer")?;
    let lines = print_setting(&settings, "lines")?;
    let skin_layers = print_setting(&settings, "skinLayers")?;
    let max_bridge = print_setting(&settings, "maxBridge")?;
    let open_top = settings["openTop"]
        .as_bool()
        .ok_or_else(|| input(PRINT_SETTINGS_MESSAGE))?;
    if ![nozzle, layer, lines, skin_layers, max_bridge]
        .iter()
        .all(|v| v.is_finite())
        || nozzle <= 0.
        || layer <= 0.
        || layer > nozzle
        || lines.fract() != 0.
        || lines < 1.
        || lines > 8.
        || skin_layers.fract() != 0.
        || skin_layers < 0.
        || max_bridge <= 0.
    {
        return Err(input(PRINT_SETTINGS_MESSAGE));
    }

    let width = (nozzle * 1.125 * 1e6).round() / 1e6;
    if !width.is_finite() || width <= 0. {
        return Err(input("Extrusion width exceeds printable numeric range."));
    }
    let rib = round_to_step(
        required_lattice_option(&options, "rib")?.max(width * lines),
        width,
    );
    let minimum_cell = rib * 2. + width;
    let mut cell = required_lattice_option(&options, "cell")?.max(minimum_cell);
    if !rib.is_finite() || !cell.is_finite() || rib <= 0. {
        return Err(input("Lattice fit exceeds finite numeric range."));
    }
    if limit_opening && cell - rib > max_bridge {
        // Increasing the rib would also increase the minimum cell and can
        // worsen the opening. Preserve the fitted rib and bound the cell.
        cell = max_bridge + rib;
        if cell - rib > max_bridge {
            cell = cell.next_down();
        }
        if !cell.is_finite() || cell < minimum_cell || cell - rib > max_bridge {
            return Err(input(
                "Opening limit is incompatible with the fitted rib and extrusion width.",
            ));
        }
    }
    let spatial = matches!(
        options["pattern"].as_str(),
        Some("bone" | "spatial" | "bcc" | "octet")
    );

    let mut result = options;
    result["lineWidth"] = Value::from(width);
    result["perimeters"] = settings["lines"].clone();
    result["rib"] = Value::from(rib);
    result["cell"] = Value::from(cell);
    if spatial {
        let skin = match lattice_option(&result, "skin")? {
            // JS truthiness: absent/zero skin stays open, anything else is fitted.
            Some(s) if s != 0. => round_to_step(s.max(width * lines), width),
            _ => 0.,
        };
        result["skin"] = Value::from(skin);
        result["openTop"] = Value::from(open_top);
        let mut step = lattice_option(&result, "step")?.unwrap_or(rib / 3.);
        step = step.min(rib / 3.);
        if skin != 0. {
            step = step.min(skin / 2.);
        }
        if let Some(wall) = lattice_option(&result, "wallDepth")?
            && wall != 0. {
                step = step.min(wall / 2.);
            }
        result["step"] = Value::from(step);
    } else {
        result["axis"] = Value::from("z");
        let floor = skin_layers * layer;
        let bottom = required_lattice_option(&result, "bottom")?;
        let top = required_lattice_option(&result, "top")?;
        let rim = required_lattice_option(&result, "rim")?;
        result["bottom"] = Value::from(round_to_step(bottom.max(floor), layer));
        result["top"] = Value::from(if open_top {
            0.
        } else {
            round_to_step(top.max(floor), layer)
        });
        result["rim"] = Value::from(round_to_step(rim.max(width * lines), width));
    }
    Ok(result)
}

pub fn bridge_warning(v: Value) -> Result<Value> {
    // Legacy comparison semantics: absent or non-finite inputs never warn.
    let warn = match (
        v["cell"].as_f64(),
        v["rib"].as_f64(),
        v["maxBridge"].as_f64(),
    ) {
        (Some(cell), Some(rib), Some(max_bridge))
            if cell.is_finite() && rib.is_finite() && max_bridge.is_finite() =>
        {
            cell - rib > max_bridge
        }
        _ => false,
    };
    Ok(Value::from(warn))
}
