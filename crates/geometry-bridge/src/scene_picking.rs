//! Runtime-owned scene broadphase indexes; no tree serialization per query.
use super::{Result, Value, field, input};
use polygon_core::solid::scene_bvh::{Index, Item};
use std::collections::BTreeMap;
use value_codec::json;
const MAX_ITEMS: usize = 100_000;
#[derive(Default)]
struct Registry {
    trees: BTreeMap<u64, Index>,
    next: u64,
    items: usize,
}
thread_local! {static REGISTRY:std::cell::RefCell<Registry>=std::cell::RefCell::new(Registry::default());}
fn id(v: &Value) -> Result<u64> {
    let text: String = field(v, "handle")?;
    let id = text
        .parse::<u64>()
        .map_err(|_| input("Invalid scene picking handle"))?;
    if id == 0 || id.to_string() != text {
        return Err(input("Noncanonical scene picking handle"));
    }
    Ok(id)
}
pub fn dispatch(v: Value) -> Result<Value> {
    REGISTRY.with(|registry|{
        let mut r=registry.borrow_mut();
        match v["action"].as_str(){
            Some("create")=>{
                let source=v["items"].as_array().ok_or_else(||input("Scene bounds must be an array"))?;
                if r.trees.len()>=64||source.len()>MAX_ITEMS.saturating_sub(r.items){return Err(input("Scene picking index budget exceeded"))}
                let leaf:usize=field(&v,"leafSize")?;
                if !(1..=32).contains(&leaf){return Err(input("Invalid scene picking leaf size"))}
                let items:Result<Vec<_>>=source.iter().map(|item|Ok(Item{id:field(item,"id")?,min:field(&item["bounds"],"min")?,max:field(&item["bounds"],"max")?})).collect();
                let tree=Index::build(items?,leaf);
                let next=r.next.checked_add(1).ok_or_else(||input("Scene picking handle space exhausted"))?;
                let result=json!({"handle":next.to_string(),"itemCount":tree.item_count,"nodeCount":tree.node_count});
                r.items+=tree.item_count;r.trees.insert(next,tree);r.next=next;Ok(result)
            }
            Some("query")=>{
                let tree=r.trees.get(&id(&v)?).ok_or_else(||input("Unknown or disposed scene picking index"))?;
                let maximum=if v.get("maximum").is_none_or(Value::is_null){f64::INFINITY}else{field(&v,"maximum")?};
                Ok(Value::Array(tree.query(field(&v,"origin")?,field(&v,"direction")?,maximum).into_iter().map(|hit|json!({"id":hit.id,"distance":hit.distance})).collect()))
            }
            Some("dispose")=>{
                let tree=r.trees.remove(&id(&v)?).ok_or_else(||input("Unknown or disposed scene picking index"))?;
                r.items-=tree.item_count;Ok(Value::Null)
            }
            _=>Err(input("Unknown scene picking action")),
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_and_failed_admission() {
        let before = REGISTRY.with(|r| r.borrow().trees.len());
        assert!(dispatch(json!({"action":"create","items":[{"id":1}],"leafSize":4})).is_err());
        assert_eq!(REGISTRY.with(|r| r.borrow().trees.len()), before);
        let built=dispatch(json!({"action":"create","items":[{"id":1,"bounds":{"min":[-1,-1,-1],"max":[1,1,1]}}],"leafSize":4})).unwrap();
        let handle = built["handle"].clone();
        let q = json!({"action":"query","handle":handle,"origin":[0,0,2],"direction":[0,0,-1]});
        assert_eq!(
            dispatch(q.clone()).unwrap()[0]["distance"].as_f64(),
            Some(1.)
        );
        dispatch(json!({"action":"dispose","handle":handle})).unwrap();
        assert!(dispatch(q).is_err());
        assert_eq!(REGISTRY.with(|r| r.borrow().trees.len()), before);
    }
}
