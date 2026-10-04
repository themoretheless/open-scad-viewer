//! Bounded combinatorial ownership premise for retained sweep geometry.
//! This proves face-set coverage only, not shell winding or embedding.
use value_codec::Deserialize;
pub(super) struct Model { pub faces: Vec<value_codec::Value>, shells: Vec<Shell>, bodies: Vec<Body> }
struct Shell { closed: bool, faces: Vec<Use> }
struct Use { face: usize, reversed: bool }
struct Body { outer_shell: usize, inner_shells: Vec<usize> }
macro_rules! decode {
    ($ty:ty, $($field:ident => $key:literal),+) => {
        impl<'a> Deserialize<'a> for $ty {
            fn from_value(v:value_codec::Value)->value_codec::Result<Self> {
                Ok(Self {$($field:value_codec::from_value(v[$key].clone())?),+})
            }
        }
    }
}
decode!(Model, faces=>"faces", shells=>"shells", bodies=>"bodies");
decode!(Shell, closed=>"closed", faces=>"faces");
decode!(Use, face=>"face", reversed=>"reversed");
decode!(Body, outer_shell=>"outerShell", inner_shells=>"innerShells");
pub(super) fn covers(model: &Model, max_faces: usize) -> bool {
    let count=model.faces.len();
    if max_faces==0 || max_faces>1026 || count==0 || count>max_faces || model.bodies.len()!=1
        || model.shells.is_empty() || model.shells.len()>count { return false; }
    let body=&model.bodies[0];
    if body.inner_shells.len().checked_add(1)!=Some(model.shells.len()) { return false; }
    let mut owners=vec![false;model.shells.len()];
    let mut faces=vec![false;count];
    for owner in std::iter::once(&body.outer_shell).chain(body.inner_shells.iter()) {
        let Some(seen)=owners.get_mut(*owner) else { return false; };
        if *seen { return false; } *seen=true;
        let shell=&model.shells[*owner];
        if !shell.closed || shell.faces.is_empty() || shell.faces.len()>count { return false; }
        for usage in &shell.faces {
            let Some(seen)=faces.get_mut(usage.face) else { return false; };
            if *seen { return false; } *seen=true;
            // Deserialization requires an explicit boolean orientation.
            let _=usage.reversed;
        }
    }
    faces.into_iter().all(|seen|seen)
}
#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    #[test]
    fn retained_body_coverage_checks_inner_shell_ownership() {
        let base=json!({"faces":[{},{}],"shells":[
            {"closed":true,"faces":[{"face":0,"reversed":false}]},
            {"closed":true,"faces":[{"face":1,"reversed":true}]}],
            "bodies":[{"outerShell":0,"innerShells":[1]}]});
        let check=|v|covers(&value_codec::from_value::<Model>(v).unwrap(),2);
        assert!(check(base.clone()));
        let mut v=base.clone();v["bodies"][0]["innerShells"]=json!([0]);assert!(!check(v));
        let mut v=base.clone();v["shells"][1]["faces"][0]["face"]=json!(0);assert!(!check(v));
        let mut v=base;v["bodies"][0]["innerShells"]=json!([]);assert!(!check(v));
    }
    #[test]
    fn retained_body_coverage_refuses_missing_duplicate_and_wrong_owners() {
        let base=json!({"faces":[{},{}],"shells":[{"closed":true,"faces":[{"face":0,"reversed":false},{"face":1,"reversed":true}]}],"bodies":[{"outerShell":0,"innerShells":[]}]});
        let check=|v|covers(&value_codec::from_value::<Model>(v).unwrap(),2);
        assert!(check(base.clone()));
        let mut v=base.clone();v["shells"][0]["faces"]=json!([{"face":0,"reversed":false}]);assert!(!check(v));
        let mut v=base.clone();v["shells"][0]["faces"][1]["face"]=json!(0);assert!(!check(v));
        let mut v=base.clone();v["bodies"][0]["outerShell"]=json!(1);assert!(!check(v));
        let mut v=base.clone();v["shells"][0]["closed"]=json!(false);assert!(!check(v));
        assert!(!covers(&value_codec::from_value::<Model>(base.clone()).unwrap(),1));
        let mut v=base;v["shells"][0]["faces"][0]["reversed"]=json!(1);assert!(value_codec::from_value::<Model>(v).is_err());
    }
}
