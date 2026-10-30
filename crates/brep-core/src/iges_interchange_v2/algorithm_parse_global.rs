//! iges interchange v2 algorithm: parse_global.
use super::*;
pub fn parse_global(text: &str) -> Result<(char, char, IgesLengthUnit, f64)> {
    let mut parameter = ',';
    let mut record = ';';
    let provisional = tokenize(text, ',', ';')?;
    if let Some(first) = provisional.first().filter(|v| v.chars().count() == 1) {
        parameter = first.chars().next().unwrap();
    }
    if let Some(second) = provisional.get(1).filter(|v| v.chars().count() == 1) {
        record = second.chars().next().unwrap();
    }
    if parameter == record || parameter.is_ascii_alphanumeric() || record.is_ascii_alphanumeric() {
        return Err(refuse("Invalid IGES global delimiters"));
    }
    let values = tokenize(text, parameter, record)?;
    for index in [6, 7, 8, 9, 10, 12, 15, 16, 18, 19, 22, 23] {
        if let Some(value) = values.get(index).filter(|value| !value.is_empty()) {
            real(value, &format!("global numeric value {index}"))?;
        }
    }
    let unit_flag = values
        .get(13)
        .ok_or_else(|| refuse("Missing IGES unit flag"))?;
    if unit_flag.is_empty() {
        return Err(refuse("Missing IGES unit flag"));
    }
    let unit_flag = unit_flag
        .parse::<usize>()
        .map_err(|_| refuse(format!("Invalid IGES unit flag {unit_flag}")))?;
    let unit = IgesLengthUnit::from_flag(unit_flag)
        .ok_or_else(|| refuse(format!("Unsupported IGES unit flag {unit_flag}")))?;
    let resolution = real(
        values
            .get(18)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| refuse("Missing IGES global minimum resolution (parameter 19)"))?,
        "global minimum resolution (parameter 19)",
    )?;
    if resolution <= 0. {
        return Err(refuse("IGES global minimum resolution must be positive"));
    }
    Ok((parameter, record, unit, resolution))
}
