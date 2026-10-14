//! Native SS classification; legacy intersection wire functions are preserved.
#[derive(Clone,Copy,Debug)] pub enum UnresolvedReason { ConditioningBoundary,ResourceBoundary }
#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub enum ContactClass { Transverse,OddTangency,EvenTangency,HigherOrderContact,UnresolvedConditioning,Coincident,PoleOrSingular,Boundary,NearCoincidence }

#[cfg(feature="codec")]
impl value_codec::Serialize for ContactClass {
    fn to_value(&self) -> value_codec::Value {
        let label = match self {
            Self::Transverse => "transverse",
            Self::OddTangency => "odd_tangency",
            Self::EvenTangency => "even_tangency",
            Self::HigherOrderContact => "higher_order_contact",
            Self::UnresolvedConditioning => "unresolved_conditioning",
            Self::Coincident => "coincident",
            Self::PoleOrSingular => "pole_or_singular",
            Self::Boundary => "boundary",
            Self::NearCoincidence => "near_coincidence",
        };
        value_codec::Serialize::to_value(label)
    }
}
