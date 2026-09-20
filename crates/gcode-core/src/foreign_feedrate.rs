use crate::{MAX_COORDINATE_MM, Result, invalid, number};

/// Programmed feedrate and its override remain independent modal state.
pub(crate) struct Feedrate {
    programmed: Option<f64>,
    percent: f64,
    saved_percent: f64,
}

impl Default for Feedrate {
    fn default() -> Self {
        Self {
            programmed: None,
            percent: 100.0,
            saved_percent: 100.0,
        }
    }
}

impl Feedrate {
    pub(crate) fn set(&mut self, mm_per_minute: f64) -> Result<()> {
        if !(mm_per_minute > 0.0 && mm_per_minute <= MAX_COORDINATE_MM * 60.0) {
            return Err(invalid(
                "GCODE_INVALID_FEEDRATE",
                "Feedrate F must be positive and within 60000000 mm/min",
            ));
        }
        self.programmed = Some(mm_per_minute / 60.0);
        Ok(())
    }

    pub(crate) fn configure<'a>(&mut self, words: impl Iterator<Item = &'a str>) -> Result<()> {
        let (mut backup, mut restore, mut percent) = (false, false, None);
        for token in words {
            match token.as_bytes().first().map(u8::to_ascii_uppercase) {
                Some(b'B') if token.len() == 1 => backup = true,
                Some(b'R') if token.len() == 1 => restore = true,
                Some(b'S') if token.len() > 1 => {
                    let value = number(&token[1..])?;
                    if value.fract() != 0.0 || !(1.0..=32767.0).contains(&value) {
                        return Err(invalid(
                            "GCODE_INVALID_FEEDRATE",
                            "M220 S must be an integer within 1..32767 percent",
                        ));
                    }
                    percent = Some(value);
                }
                _ => {
                    return Err(invalid(
                        "GCODE_UNSUPPORTED_COMMAND",
                        "M220 supports S percentage and bare B/R flags",
                    ));
                }
            }
        }
        // Simultaneous B/R saves the incoming factor, not the restored one.
        let previous = self.percent;
        if restore {
            self.percent = self.saved_percent;
        }
        if backup {
            self.saved_percent = previous;
        }
        if let Some(value) = percent {
            self.percent = value;
        }
        Ok(())
    }

    pub(crate) fn effective(&self) -> Result<Option<f64>> {
        self.programmed
            .map(|speed| {
                let speed = speed * (self.percent / 100.0);
                if speed.is_finite() && speed > 0.0 && speed <= MAX_COORDINATE_MM {
                    Ok(speed)
                } else {
                    Err(invalid(
                        "GCODE_INVALID_FEEDRATE",
                        "Effective feedrate exceeds finite positive preview range",
                    ))
                }
            })
            .transpose()
    }
}
