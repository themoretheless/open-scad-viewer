use crate::block::Block;

fn word(block: &Block, letter: char) -> Option<f64> {
    block
        .words
        .iter()
        .rev()
        .find(|word| word.letter == letter)
        .map(|word| word.value)
}

fn has_axis(block: &Block) -> bool {
    block
        .words
        .iter()
        .any(|word| matches!(word.letter, 'X' | 'Y' | 'Z' | 'E'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionKind {
    Rapid,
    Linear,
    ArcCw,
    ArcCcw,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveArgs {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub e: Option<f64>,
    pub f: Option<f64>,
    pub i: Option<f64>,
    pub j: Option<f64>,
    pub k: Option<f64>,
    pub r: Option<f64>,
    pub p: Option<f64>,
}

impl MoveArgs {
    fn from_block(block: &Block) -> Self {
        Self {
            x: word(block, 'X'),
            y: word(block, 'Y'),
            z: word(block, 'Z'),
            e: word(block, 'E'),
            f: word(block, 'F'),
            i: word(block, 'I'),
            j: word(block, 'J'),
            k: word(block, 'K'),
            r: word(block, 'R'),
            p: word(block, 'P'),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Motion {
        kind: MotionKind,
        args: MoveArgs,
    },
    Dwell {
        p_ms: Option<f64>,
        s_s: Option<f64>,
    },
    FirmwareRetract,
    FirmwareRecover,
    PlaneXy,
    UnitsInch,
    UnitsMm,
    Home {
        x: bool,
        y: bool,
        z: bool,
    },
    AbsoluteXyz,
    RelativeXyz,
    SetPosition {
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        e: Option<f64>,
    },
    Hotend {
        c: f64,
        wait: bool,
        tool: Option<u32>,
    },
    Bed {
        c: f64,
        wait: bool,
    },
    Chamber {
        c: f64,
        wait: bool,
    },
    Fan {
        pwm: f64,
    },
    ExtrusionAbsolute,
    ExtrusionRelative,
    MotorsOff,
    Volumetric {
        diameter_mm: Option<f64>,
    },
    SpeedFactor {
        percent: f64,
    },
    FlowFactor {
        percent: f64,
    },
    Tool {
        index: u32,
    },
    Comment,
    Other {
        block: Block,
    },
}

pub fn classify(block: &Block) -> Result<Command, ClassifyError> {
    if block.words.iter().all(|word| word.letter == 'N') || block.words.is_empty() {
        return Ok(Command::Comment);
    }
    let g = word(block, 'G').map(|v| v as i32);
    let m = word(block, 'M').map(|v| v as i32);
    let t = word(block, 'T').map(|v| v as i32);
    if let Some(code) = g {
        return classify_g(code, block);
    }
    if let Some(code) = m {
        return classify_m(code, block);
    }
    if let Some(index) = t {
        if index < 0 {
            return Err(ClassifyError::Unsupported);
        }
        return Ok(Command::Tool {
            index: index as u32,
        });
    }
    Ok(Command::Other {
        block: block.clone(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassifyError {
    Unsupported,
}

fn classify_g(code: i32, block: &Block) -> Result<Command, ClassifyError> {
    match code {
        0 => Ok(Command::Motion {
            kind: MotionKind::Rapid,
            args: MoveArgs::from_block(block),
        }),
        1 => Ok(Command::Motion {
            kind: MotionKind::Linear,
            args: MoveArgs::from_block(block),
        }),
        2 => Ok(Command::Motion {
            kind: MotionKind::ArcCw,
            args: MoveArgs::from_block(block),
        }),
        3 => Ok(Command::Motion {
            kind: MotionKind::ArcCcw,
            args: MoveArgs::from_block(block),
        }),
        4 => Ok(Command::Dwell {
            p_ms: word(block, 'P'),
            s_s: word(block, 'S'),
        }),
        10 if has_axis(block) => Err(ClassifyError::Unsupported),
        10 => Ok(Command::FirmwareRetract),
        11 if has_axis(block) => Err(ClassifyError::Unsupported),
        11 => Ok(Command::FirmwareRecover),
        17 => Ok(Command::PlaneXy),
        20 => Ok(Command::UnitsInch),
        21 => Ok(Command::UnitsMm),
        28 => Ok(Command::Home {
            x: word(block, 'X').is_some(),
            y: word(block, 'Y').is_some(),
            z: word(block, 'Z').is_some(),
        }),
        90 => Ok(Command::AbsoluteXyz),
        91 => Ok(Command::RelativeXyz),
        92 => Ok(Command::SetPosition {
            x: word(block, 'X'),
            y: word(block, 'Y'),
            z: word(block, 'Z'),
            e: word(block, 'E'),
        }),
        18 | 19 | 41 | 42 | 43 | 81 | 82 | 83 | 84 | 85 | 86 | 87 | 88 | 89 => {
            Err(ClassifyError::Unsupported)
        }
        _ => Ok(Command::Other {
            block: block.clone(),
        }),
    }
}

fn classify_m(code: i32, block: &Block) -> Result<Command, ClassifyError> {
    match code {
        18 | 84 => Ok(Command::MotorsOff),
        82 => Ok(Command::ExtrusionAbsolute),
        83 => Ok(Command::ExtrusionRelative),
        104 => Ok(Command::Hotend {
            c: word(block, 'S').unwrap_or(0.0),
            wait: false,
            tool: word(block, 'T').map(|v| v as u32),
        }),
        109 => Ok(Command::Hotend {
            c: word(block, 'S').unwrap_or(0.0),
            wait: true,
            tool: word(block, 'T').map(|v| v as u32),
        }),
        106 => Ok(Command::Fan {
            pwm: word(block, 'S').unwrap_or(255.0).clamp(0.0, 255.0),
        }),
        107 => Ok(Command::Fan { pwm: 0.0 }),
        140 => Ok(Command::Bed {
            c: word(block, 'S').unwrap_or(0.0),
            wait: false,
        }),
        190 => Ok(Command::Bed {
            c: word(block, 'S').unwrap_or(0.0),
            wait: true,
        }),
        141 => Ok(Command::Chamber {
            c: word(block, 'S').unwrap_or(0.0),
            wait: false,
        }),
        191 => Ok(Command::Chamber {
            c: word(block, 'S').unwrap_or(0.0),
            wait: true,
        }),
        200 => Ok(Command::Volumetric {
            diameter_mm: word(block, 'D'),
        }),
        220 => Ok(Command::SpeedFactor {
            percent: word(block, 'S').unwrap_or(100.0),
        }),
        221 => Ok(Command::FlowFactor {
            percent: word(block, 'S').unwrap_or(100.0),
        }),
        _ => Ok(Command::Other {
            block: block.clone(),
        }),
    }
}

pub fn layer_index(block: &Block) -> Option<usize> {
    let comment = block.comment.as_deref()?.trim();
    comment
        .strip_prefix("LAYER:")
        .or_else(|| comment.strip_prefix("layer "))
        .and_then(|rest| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|digits| digits.parse().ok())
        })
}
