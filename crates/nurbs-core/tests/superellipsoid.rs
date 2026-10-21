use nurbs_core::superellipsoid::{self, Request, Stop};
fn request(exponents:[f64;2])->Request {Request {center:[2.,-3.,7.],radii:[2.,3.,4.],exponents,tolerance:0.5,max_cells:8192}}
#[test]
fn all_octants_match_independent_implicit_surface_at_original_grid_points() {
    for e in [[1.,1.],[2.,2.],[0.5,0.5],[0.7,1.8]] {
        let r=request(e);let report=superellipsoid::approximate(r).unwrap();
        for patch in &report.patches {
            for row in &patch.surface.control_points {for p in row {
                let x=(p[0]-r.center[0]).abs()/r.radii[0];
                let y=(p[1]-r.center[1]).abs()/r.radii[1];
                let z=(p[2]-r.center[2]).abs()/r.radii[2];
                let implicit=(x.powf(2./e[1])+y.powf(2./e[1])).powf(e[1]/e[0])+z.powf(2./e[0]);
                assert!((implicit-1.).abs()<1e-12,"{e:?}: {implicit}");
            }}
            for i in 0..=7 {for j in 0..=7 {
                let u=patch.domain[0][0]+(patch.domain[0][1]-patch.domain[0][0])*i as f64/7.;
                let v=patch.domain[1][0]+(patch.domain[1][1]-patch.domain[1][0])*j as f64/7.;
                let lat=std::f64::consts::FRAC_PI_2*u;let lon=std::f64::consts::FRAC_PI_2*v;
                let ideal=[lat.cos().powf(e[0])*lon.cos().powf(e[1]),lat.cos().powf(e[0])*lon.sin().powf(e[1]),lat.sin().powf(e[0])];
                let p=patch.surface.evaluate(u,v).unwrap().point;
                let error=(0..3).map(|k|(p[k]-r.center[k]-patch.signs[k] as f64*r.radii[k]*ideal[k]).powi(2)).sum::<f64>().sqrt();
                assert!(error<=report.analytic_interpolation_estimate+1e-6,"{error}");
            }}
        }
    }
}
#[test]
fn budget_preserves_complete_octants_and_exact_seam_and_pole_controls() {
    let mut r=request([1.,1.]);r.max_cells=32768;r.tolerance=1e-9;
    let report=superellipsoid::approximate(r).unwrap();
    assert_eq!(report.stop,Stop::CellBudget);assert_eq!(report.subdivisions,64);assert_eq!(report.cells,32768);
    assert_eq!(report.patches.len(),72);
    for a in &report.patches {for b in &report.patches {
        if a.signs==b.signs && a.domain[0][1]==b.domain[0][0] && a.domain[1]==b.domain[1] {
            assert_eq!(a.surface.control_points.last(),b.surface.control_points.first());
        }
        if a.signs==b.signs && a.domain[1][1]==b.domain[1][0] && a.domain[0]==b.domain[0] {
            for (x,y) in a.surface.control_points.iter().zip(&b.surface.control_points) {assert_eq!(x.last(),y.first());}
        }
    }}
    for patch in &report.patches {
        if patch.domain[0][1]==1. {for p in patch.surface.control_points.last().unwrap() {
            assert_eq!(p,&vec![2.,-3.,7.+patch.signs[2] as f64*4.]);
        }}
    }
    r.max_cells=8;let coarse=superellipsoid::approximate(r).unwrap();assert_eq!(coarse.patches.len(),8);assert_eq!(coarse.cells,8);
}
#[test]
fn analytic_estimate_refines_and_invalid_inputs_are_refused() {
    let mut r=request([1.,1.]);r.tolerance=10.;let coarse=superellipsoid::approximate(r).unwrap();
    assert_eq!(coarse.stop,Stop::AnalyticEstimateReached);
    r.tolerance=1.;let fine=superellipsoid::approximate(r).unwrap();
    assert!(fine.cells>coarse.cells && fine.analytic_interpolation_estimate<coarse.analytic_interpolation_estimate);
    for e in [0.,-1.,f64::NAN,f64::INFINITY] {let mut bad=r;bad.exponents[0]=e;assert!(superellipsoid::approximate(bad).is_err());}
    for b in [0,7,131073] {let mut bad=r;bad.max_cells=b;assert!(superellipsoid::approximate(bad).is_err());}
    for radius in [0.,-1.,f64::INFINITY] {let mut bad=r;bad.radii[1]=radius;assert!(superellipsoid::approximate(bad).is_err());}
    for tol in [0.,f64::NAN] {let mut bad=r;bad.tolerance=tol;assert!(superellipsoid::approximate(bad).is_err());}
    let mut bad=r;bad.center[0]=1e100;assert!(superellipsoid::approximate(bad).is_err());
}
