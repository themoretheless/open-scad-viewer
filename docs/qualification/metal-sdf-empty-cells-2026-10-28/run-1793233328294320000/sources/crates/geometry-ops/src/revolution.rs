//! Profile classification for rotation about its vertical axis.
pub fn profile_side(minimum: f64, maximum: f64) -> &'static str {
    if minimum < 0. && maximum > 0. {
        "crossing"
    } else if maximum <= 0. && minimum < 0. {
        "negative"
    } else if minimum == 0. && maximum == 0. {
        "axis"
    } else {
        "positive"
    }
}
