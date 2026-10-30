//! iges interchange v2 algorithm: render.
use super::*;
pub fn render(mut w: Writer) -> Result<String> {
    let mut p_lines = Vec::new();
    let mut p_start = Vec::new();
    for (index, e) in w.entities.iter().enumerate() {
        let de = 2 * index + 1;
        let text = format!("{};", e.values.join(","));
        p_start.push(p_lines.len() + 1);
        for chunk in text.as_bytes().chunks(64) {
            let payload =
                std::str::from_utf8(chunk).map_err(|_| refuse("Non-ASCII IGES parameter"))?;
            p_lines.push(
                pad_record(payload, 'P', p_lines.len() + 1, 64)?[..64].to_string()
                    + &format!("{de:>8}P{:>7}", p_lines.len() + 1),
            );
        }
    }
    let mut lines = vec![pad_record(
        "OpenSCAD Viewer direct finite B-rep",
        'S',
        1,
        72,
    )?];
    let global = format!(
        "1H,,1H;,{} ,{} ,{} ,{} ,32,38,6,308,15,{},1.0,2,2HMM,1,0.001,{},0.001,{},{},{},11,0,{};",
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("IGES2"),
        hollerith("20260917.000000"),
        number(1e-7),
        hollerith(""),
        hollerith(""),
        hollerith("20260917.000000")
    );
    for (i, chunk) in global.as_bytes().chunks(72).enumerate() {
        lines.push(pad_record(
            std::str::from_utf8(chunk).unwrap(),
            'G',
            i + 1,
            72,
        )?);
    }
    for (index, e) in w.entities.iter_mut().enumerate() {
        let de = 2 * index + 1;
        let a = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}",
            e.ty, p_start[index], 0, 0, 0, 0, 0, 0, "00000000"
        );
        let b = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:<8}{:>8}",
            e.ty, 0, e.color, 0, e.form, 0, 0, e.label, 0
        );
        lines.push(pad_record(&a, 'D', de, 72)?);
        lines.push(pad_record(&b, 'D', de + 1, 72)?);
    }
    for p in &p_lines {
        lines.push(p.clone());
    }
    let terminate = format!(
        "S{:>7}G{:>7}D{:>7}P{:>7}",
        1,
        lines.iter().filter(|l| l.as_bytes()[72] == b'G').count(),
        2 * w.entities.len(),
        p_lines.len()
    );
    lines.push(pad_record(&terminate, 'T', 1, 72)?);
    let text = lines.join("\n") + "\n";
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES output exceeds 16 MiB"));
    }
    Ok(text)
}
