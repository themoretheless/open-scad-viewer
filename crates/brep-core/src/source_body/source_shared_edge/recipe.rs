//! Original mapping inputs, without cached proof or dynamic wire values.
use nurbs_core::curve::Curve;
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Recipe {
    Direct {
        world_reversed: [bool; 2],
        cutters: [Option<Curve>; 2],
        planes: [Option<[[f64; 3]; 3]>; 2],
    },
    Mapped {
        ranges: [[[f64; 2]; 2]; 2],
        planes: [Option<[[f64; 3]; 3]>; 2],
        candidates: [[Option<[f64; 2]>; 2]; 2],
    },
}
impl Recipe {
    #[cfg(feature = "codec")]
    pub(super) fn definition(&self) -> value_codec::Value {
        match self {
            Self::Direct {
                world_reversed,
                cutters,
                planes,
            } => {
                value_codec::json!({"kind":"direct","worldReversed":world_reversed,"cutters":cutters,"planes":planes})
            }
            Self::Mapped {
                ranges,
                planes,
                candidates,
            } => {
                value_codec::json!({"kind":"mapped","ranges":ranges,"planes":planes,"candidates":candidates})
            }
        }
    }
}
