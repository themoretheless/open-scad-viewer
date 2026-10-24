
 use super::*;
 fn line(a:[f64;3],b:[f64;3])->Curve{Curve::from_polyline(vec![a.to_vec(),b.to_vec()]).unwrap()}
 #[test]fn default_budget_keeps_existing_report_identical(){
  let a=line([0.,0.,0.],[2.,2.,0.]);let b=line([0.,2.,0.],[2.,0.,0.]);
  assert_eq!(intersect_curve_curve(&a,&b,None).unwrap(),intersect_curve_curve_bounded(&a,&b,None,8192).unwrap());
 }
 #[test]fn unvisited_source_spans_remain_unresolved_at_box_limit(){
  let a=Curve::from_polyline(vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]]).unwrap();let b=line([1.,-1.,0.],[1.,2.,0.]);
  let report=intersect_curve_curve_bounded(&a,&b,None,1).unwrap();
  assert_eq!(report["coverage"]["resourceLimit"].as_u64(),Some(1));assert!(report["coverage"]["boxesVisited"].as_u64().unwrap()<=1);assert_eq!(report["coverage"]["complete"].as_bool(),Some(false));
  assert!(!report["unresolved"].as_array().unwrap().is_empty());
  assert!(intersect_curve_curve_bounded(&a,&b,None,0).is_err());
 }
 #[test]fn parallel_offset_lines_are_not_coincident(){
  let a=line([0.,0.,0.],[10.,10.,0.]);
  let b=line([0.,1.,0.],[10.,11.,0.]);
  for (first,second) in [(&a,&b),(&b,&a)] {
   let report=intersect_curve_curve(first,second,None).unwrap();
   assert_eq!(report["coverage"]["complete"].as_bool(),Some(true));
   assert!(report["components"].as_array().unwrap().is_empty());
  }
  let overlap=line([2.,2.,0.],[8.,8.,0.]);
  let report=intersect_curve_curve(&a,&overlap,None).unwrap();
  assert_eq!(report["components"][0]["kind"],"overlap");
 }
