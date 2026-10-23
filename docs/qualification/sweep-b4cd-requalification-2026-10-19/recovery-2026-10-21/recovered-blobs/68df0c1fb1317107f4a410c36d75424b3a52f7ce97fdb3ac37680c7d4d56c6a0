//! Native spatial lattice utilities: graph generation, component counting,
//! topology-preserving decimation, and lightening cell construction.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;

use polygon_core::lattice_tools::component_count;
pub fn components(v: Value) -> Result<Value> {
    encode(component_count(
        &field::<Mesh>(&v, "mesh")?,
        field(&v, "positiveOnly")?,
    )?)
}
pub fn decimate(v: Value) -> Result<Value> {
    encode(polygon_core::lattice_tools::decimate(
        field(&v, "mesh")?,
        field(&v, "tolerance")?,
        field(&v, "target")?,
    )?)
}

pub fn lightening(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let mesh: Mesh = field(&body, "mesh")?;
    let spatial = matches!(
        v["pattern"].as_str(),
        Some("bone" | "spatial" | "bcc" | "octet")
    );
    let options = polygon_core::lattice_tools::LighteningOptions {
        pattern: field(&v, "pattern")?,
        axis: field(&v, "axis")?,
        cell: field(&v, "cell")?,
        rib: field(&v, "rib")?,
        rim: field(&v, "rim")?,
        bottom: field(&v, "bottom")?,
        top: field(&v, "top")?,
        seed: field(&v, "seed")?,
        jitter: field(&v, "jitter")?,
        line_width: field(&v, "lineWidth")?,
        perimeters: field(&v, "perimeters")?,
        skin: if spatial && v.get("skin").is_some_and(|v| !v.is_null()) {
            field(&v, "skin")?
        } else {
            0.
        },
        step: if spatial && v.get("step").is_some_and(|v| !v.is_null()) {
            field(&v, "step")?
        } else {
            0.
        },
        wall_depth: if spatial && v.get("wallDepth").is_some_and(|v| !v.is_null()) {
            field(&v, "wallDepth")?
        } else {
            0.
        },
        open_top: if spatial && v.get("openTop").is_some_and(|v| !v.is_null()) {
            field(&v, "openTop")?
        } else {
            false
        },
        keep_core: if spatial && v.get("keepCore").is_some_and(|v| !v.is_null()) {
            field(&v, "keepCore")?
        } else {
            false
        },
        diagonals: if spatial && v.get("diagonals").is_some_and(|v| !v.is_null()) {
            field(&v, "diagonals")?
        } else {
            false
        },
    };
    let mesh = polygon_core::lattice_tools::lighten(mesh, options)?;
    let mut output = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    output.insert("mesh".into(), encode(mesh)?);
    Ok(Value::Object(output))
}

/// User-facing admission message, kept identical to the pre-migration TS text.
const PRINT_SETTINGS_MESSAGE: &str = "Проверьте сопло, высоту слоя, число линий и длину моста.";

fn print_setting(settings: &Value, key: &str) -> Result<f64> {
    settings[key]
        .as_f64()
        .ok_or_else(|| input(PRINT_SETTINGS_MESSAGE))
}

/// Optional lightening option: absent/null stays absent, present values must be
/// finite numbers (the pre-migration TS computed NaN silently).
fn lattice_option(options: &Value, key: &str) -> Result<Option<f64>> {
    let v = &options[key];
    if v.is_null() {
        return Ok(None);
    }
    match v.as_f64() {
        Some(x) if x.is_finite() => Ok(Some(x)),
        _ => Err(input("Lattice geometry options must be finite numbers.")),
    }
}

fn required_lattice_option(options: &Value, key: &str) -> Result<f64> {
    lattice_option(options, key)?
        .ok_or_else(|| input("Lattice geometry options must be finite numbers."))
}

/// Legacy rounding order: ceil toward the step with a 1e-8 epsilon, then
/// compact to 1e6 grid. Kept bit-identical to the pre-migration TS helper.
fn round_to_step(v: f64, s: f64) -> f64 {
    (((v - 1e-8) / s).ceil() * s * 1e6).round() / 1e6
}

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
            && wall != 0.
        {
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

#[cfg(test)]
mod print_tests {
    use super::*;
    use value_codec::json;

    fn request(limit: f64) -> Value {
        json!({"options":{"pattern":"grid","axis":"x","cell":20.,"rib":1.35,
            "rim":2.,"bottom":0.1,"top":0.7},
            "settings":{"nozzle":0.6,"layer":0.25,"lines":4,"skinLayers":4,
            "maxBridge":limit,"openTop":true},"limitOpening":true})
    }

    #[test]
    fn opening_fit_is_bounded_and_idempotent() {
        let v = request(5.);
        let fitted = print_fit(v.clone()).unwrap();
        let rib = fitted["rib"].as_f64().unwrap();
        let cell = fitted["cell"].as_f64().unwrap();
        assert_eq!(rib, 2.7);
        assert!(cell - rib <= 5.);
        assert!(cell >= 2. * rib + fitted["lineWidth"].as_f64().unwrap());
        let mut again = v;
        again["options"] = fitted.clone();
        assert_eq!(print_fit(again).unwrap(), fitted);
    }

    #[test]
    fn opening_fit_refuses_impossible_and_malformed_constraints() {
        assert!(
            print_fit(request(1.))
                .unwrap_err()
                .message
                .contains("incompatible")
        );
        let mut v = request(5.);
        v["limitOpening"] = json!("true");
        assert!(print_fit(v).unwrap_err().message.contains("boolean"));
        for nozzle in [1e-20, f64::MAX] {
            let mut v = request(5.);
            v["settings"]["nozzle"] = json!(nozzle);
            v["settings"]["layer"] = json!(nozzle);
            assert!(print_fit(v).unwrap_err().message.contains("numeric range"));
        }
    }

    #[test]
    fn opening_fit_respects_representable_bounds() {
        for nozzle in [0.25, 0.4, 0.6, 0.8] {
            for lines in 1..=8 {
                for limit in [2., 3., 5., 10.] {
                    let mut v = request(limit);
                    v["settings"]["nozzle"] = json!(nozzle);
                    v["settings"]["layer"] = json!(0.2);
                    v["settings"]["lines"] = json!(lines);
                    let mut unlimited = v.clone();
                    unlimited["limitOpening"] = json!(false);
                    let base = print_fit(unlimited).unwrap();
                    let rib = base["rib"].as_f64().unwrap();
                    let minimum = 2. * rib + base["lineWidth"].as_f64().unwrap();
                    match print_fit(v) {
                        Ok(fit) => {
                            let cell = fit["cell"].as_f64().unwrap();
                            assert!(cell >= minimum && cell - rib <= limit);
                            assert_eq!(fit["rib"], base["rib"]);
                        }
                        Err(_) => assert!(minimum - rib > limit),
                    }
                }
            }
        }
    }
}

pub fn lightening_cells(v: Value) -> Result<Value> {
    encode(polygon_core::lattice_tools::generate_lightening_cells(
        field(&v, "min")?,
        field(&v, "max")?,
        &field::<String>(&v, "pattern")?,
        field(&v, "cell")?,
        field(&v, "rib")?,
        field(&v, "seed")?,
        field(&v, "jitter")?,
    )?)
}
pub fn graph(v: Value) -> Result<Value> {
    let options = &v["options"];
    let g = polygon_core::lattice_tools::graph(
        field(&v, "mesh")?,
        field(options, "cell")?,
        field(options, "jitter")?,
        field(options, "seed")?,
        &field::<String>(options, "pattern")?,
        options
            .get("diagonals")
            .map(|_| field::<bool>(options, "diagonals"))
            .transpose()?
            .unwrap_or(false),
    )?;
    encode(value_codec::json!({"nodes":g.nodes,"edges":g.edges}))
}
