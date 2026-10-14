//! Stable OpenSCAD degree arithmetic shared by native evaluation and host adapters.
// Degree-exact trigonometry: normalize once, solve a first-quadrant pair and
// restore signs by quadrant so common angles stay exact after full rotations.
const DEG_TO_RAD: f64 = 0.017_453_292_519_943_295;
const RAD_TO_DEG: f64 = 57.295_779_513_082_32;
const SQRT_THREE_QUARTERS: f64 = 0.866_025_403_784_438_6;
const SQRT_ONE_THIRD: f64 = 0.577_350_269_189_625_7;
const TRIG_HUGE_VALUE: f64 = 360.0 * 4_503_599_627_370_496.0; // 360 * 2^52

fn reduce_degrees(value: f64, period: f64) -> Option<(f64, f64)> {
    if !(value < TRIG_HUGE_VALUE && value > -TRIG_HUGE_VALUE) {
        return None;
    }
    if value >= 0.0 && value < period {
        return Some((value, 0.0));
    }
    let cycles = (value / period).floor();
    Some((value - cycles * period, cycles))
}

fn first_quadrant_components(angle: f64) -> (f64, f64) {
    if angle == 30.0 {
        return (0.5, SQRT_THREE_QUARTERS);
    }
    if angle == 45.0 {
        return (
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        );
    }
    if angle == 60.0 {
        return (SQRT_THREE_QUARTERS, 0.5);
    }
    if angle < 45.0 {
        let radians = angle * DEG_TO_RAD;
        return (radians.sin(), radians.cos());
    }
    let complement = (90.0 - angle) * DEG_TO_RAD;
    (complement.cos(), complement.sin())
}

fn unit_circle_components(value: f64) -> Option<(f64, f64)> {
    let (angle, _) = reduce_degrees(value, 360.0)?;
    if angle == 0.0 {
        return Some((angle, 1.0));
    }
    if angle == 90.0 {
        return Some((1.0, 0.0));
    }
    if angle == 180.0 {
        return Some((-0.0, -1.0));
    }
    if angle == 270.0 {
        return Some((-1.0, -0.0));
    }
    let quadrant = (angle / 90.0).floor();
    let (sin, cos) = first_quadrant_components(angle - quadrant * 90.0);
    Some(match quadrant as i32 {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    })
}

pub fn sin_degrees(value: f64) -> f64 {
    unit_circle_components(value).map_or(f64::NAN, |(s, _)| s)
}
pub fn cos_degrees(value: f64) -> f64 {
    unit_circle_components(value).map_or(f64::NAN, |(_, c)| c)
}

pub fn tan_degrees(value: f64) -> f64 {
    let Some((angle, cycles)) = reduce_degrees(value, 180.0) else {
        return f64::NAN;
    };
    if angle == 0.0 {
        return if (cycles as i64) % 2 == 0 { 0.0 } else { -0.0 };
    }
    if angle == 90.0 {
        return if (cycles as i64) % 2 == 0 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
    }
    let oppose = angle > 90.0;
    let acute = if oppose { 180.0 - angle } else { angle };
    let magnitude = if acute == 30.0 {
        SQRT_ONE_THIRD
    } else if acute == 45.0 {
        1.0
    } else if acute == 60.0 {
        3.0_f64.sqrt()
    } else {
        (acute * DEG_TO_RAD).tan()
    };
    if oppose { -magnitude } else { magnitude }
}

/// C++ std::round semantics: halfway cases round away from zero.
pub fn round_away_from_zero(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    value.signum() * (value.abs() + 0.5).floor()
}

pub fn asin_degrees(value: f64) -> f64 {
    let degrees = value.asin() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if sin_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
pub fn acos_degrees(value: f64) -> f64 {
    let degrees = value.acos() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if cos_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
pub fn atan_degrees(value: f64) -> f64 {
    let degrees = value.atan() * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if tan_degrees(whole) == value {
        whole
    } else {
        degrees
    }
}
pub fn atan2_degrees(y: f64, x: f64) -> f64 {
    let degrees = y.atan2(x) * RAD_TO_DEG;
    let whole = round_away_from_zero(degrees);
    if (degrees - whole).abs() < 3e-14 {
        whole
    } else {
        degrees
    }
}
