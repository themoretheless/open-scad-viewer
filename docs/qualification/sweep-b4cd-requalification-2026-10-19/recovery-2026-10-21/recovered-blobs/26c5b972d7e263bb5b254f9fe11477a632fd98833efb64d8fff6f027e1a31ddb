use std::fmt::{self, Write};

use crate::{
    JobProfile, MAX_OUTPUT_BYTES, MachineProfile, PlannedLayer, Result, emit_body, invalid,
};

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
    write_bounded(target, |out| emit_body(layers, machine, out))
}

/// Emit a machine job directly to a text writer under the same bounds as `emit_to`.
/// On error discard partial output. This performs no printer I/O or readiness checks.
pub fn emit_job_to(
    layers: &[PlannedLayer],
    job: &JobProfile,
    target: &mut impl Write,
) -> Result<()> {
    write_bounded(target, |out| crate::job::emit_job_body(layers, job, out))
}

fn write_bounded<W: Write>(
    target: &mut W,
    emit: impl FnOnce(&mut LimitedWriter<'_, W>) -> Result<()>,
) -> Result<()> {
    let mut out = LimitedWriter {
        target,
        written: 0,
        failed: false,
    };
    let result = emit(&mut out);
    if out.failed {
        return Err(invalid(
            "GCODE_WRITE",
            "G-code output writer failed; discard partial output",
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_budget_accepts_then_refuses_without_appending() {
        let mut bytes = String::new();
        let mut out = LimitedWriter {
            target: &mut bytes,
            written: 0,
            failed: false,
        };
        out.write_str(&"x".repeat(MAX_OUTPUT_BYTES)).unwrap();
        assert!(out.write_str("x").is_err());
        assert!(!out.failed);
        assert_eq!(bytes.len(), MAX_OUTPUT_BYTES);
    }

    #[test]
    fn job_propagates_writer_failure_after_partial_progress() {
        struct Partial {
            calls: usize,
        }
        impl Write for Partial {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                self.calls += 1;
                if self.calls > 2 {
                    Err(fmt::Error)
                } else {
                    Ok(())
                }
            }
        }
        let mut target = Partial { calls: 0 };
        let error = emit_job_to(&[], &JobProfile::default(), &mut target).unwrap_err();
        assert_eq!(error.code, "GCODE_WRITE");
        assert_eq!(target.calls, 3);
    }
}
