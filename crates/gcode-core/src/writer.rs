use std::fmt::{self, Write};

use crate::{MAX_OUTPUT_BYTES, MachineProfile, PlannedLayer, Result, emit_body, invalid};

struct LimitedWriter<'a, W> {
    target: &'a mut W,
    written: usize,
    failed: bool,
}

impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > MAX_OUTPUT_BYTES.saturating_sub(self.written) {
            return Err(fmt::Error);
        }
        if self.target.write_str(text).is_err() {
            self.failed = true;
            return Err(fmt::Error);
        }
        self.written += text.len();
        Ok(())
    }
}

/// Emit the preview dialect directly to a text writer, at most 4 MiB per call.
/// Errors may leave a prefix in the writer; discard it rather than using a partial document.
/// This does not flush, roll back, or communicate with a printer.
pub fn emit_to(
    layers: &[PlannedLayer],
    machine: &MachineProfile,
    target: &mut impl Write,
) -> Result<()> {
    let mut out = LimitedWriter {
        target,
        written: 0,
        failed: false,
    };
    let result = emit_body(layers, machine, &mut out);
    if out.failed {
        return Err(invalid(
            "GCODE_WRITE",
            "G-code output writer failed; discard partial output",
        ));
    }
    result
}
