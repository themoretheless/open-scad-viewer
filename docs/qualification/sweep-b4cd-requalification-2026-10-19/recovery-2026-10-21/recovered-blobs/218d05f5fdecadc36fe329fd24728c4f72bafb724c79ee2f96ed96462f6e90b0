//! Host encoding of native domain and distance reports.
use super::*;
use value_codec::{Value, json};
impl Location {
    fn name(self) -> &'static str {
        match self {
            Self::Inside => "inside",
            Self::Outside => "outside",
            Self::Unresolved => "unresolved",
        }
    }
}
impl value_codec::Serialize for ClassificationReason {
    fn to_value(&self) -> Value {
        value_codec::Serialize::to_value(match self {
            Self::Separated => "separated",
            Self::WorkLimit => "work-limit",
            Self::JoinBand => "join-band",
            Self::BoundaryBand => "boundary-band",
            Self::PrecisionLimit => "precision-limit",
        })
    }
}

impl Classification {
    pub fn to_value(&self) -> Value {
        json!({"method":"interval-hull-homotopy","fillRule":"nonzero","location":self.location.name(),"winding":self.winding,"reason":self.reason,"cells":self.cells,"maxCells":self.max_cells,"toleranceUv":self.tolerance_uv,
          "uncertain":self.uncertain.map(|(loop_index,curve_index,domain)|json!({"loop":loop_index,"curve":curve_index,"parameterRange":domain})),"closure":"joins-bounded-by-tolerance"})
    }
}
impl value_codec::Serialize for Classification {
    fn to_value(&self) -> Value {
        Classification::to_value(self)
    }
}
