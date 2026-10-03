use value_codec::Value;
fn main(){
 let input=std::env::args().nth(1).unwrap();let output=std::env::args().nth(2).unwrap();
 let mut fixture:Value=value_codec::from_str(&std::fs::read_to_string(input).unwrap()).unwrap();
 for c in fixture["cases"].as_array_mut().unwrap(){
  c["response"]=value_codec::json!({"ok":true,"value":geometry_bridge::dispatch(c["request"].clone()).unwrap()});
 }
 std::fs::write(output,value_codec::to_string(&fixture).unwrap()).unwrap();
}
