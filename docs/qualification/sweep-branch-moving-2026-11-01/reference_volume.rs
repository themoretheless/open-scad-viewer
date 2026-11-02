// Independent analytic reference. D is the symmetric cubic profile with area
// 61/20 and radial centroid 3. C=(1-t,t)/(1-t+t²), rotated through 4 quarters.
// Integrating det DF gives 2*k*Area(C)*integral_D(k²*r + eps²*h*h') dr dz.
// h is even around r=3, hence h*h' is odd and its integral vanishes exactly.
fn main(){
    let k=60f64/64.;let area_d=61f64/20.;
    let area_c=4f64/3.+8.*std::f64::consts::PI/(9.*3f64.sqrt());
    let volume=6.*k.powi(3)*area_d*area_c;
    println!("{{\"volume\":{volume:.17},\"profileArea\":{area_d:.17},\"phaseArea\":{area_c:.17},\"scale\":{k:.17},\"radialCentroid\":3.0,\"independentFromNativeGeometryIntegration\":true}}");
}
