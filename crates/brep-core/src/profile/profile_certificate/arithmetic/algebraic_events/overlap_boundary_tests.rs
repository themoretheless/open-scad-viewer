use super::*;
#[test]
fn independent_square_and_cube_parameters_share_complete_joint_extension() {
    let q = Q::new(1.into(), 2.into());
    // s^2=q and t^3=q. Neither parameter is a rational polynomial
    // of the other; a separating joint primitive is required.
    let equations = [
        vec![vec![-q.clone(), Q::zero(), Q::one()]],
        vec![vec![-q], Vec::new(), Vec::new(), vec![Q::one()]],
    ];
    let width = Q::new(1.into(), 1024.into());
    let first = vec![
        [Q::zero(), Q::zero(), Q::one()],
        [Q::zero(), Q::zero(), Q::one()],
        [Q::one(), Q::zero(), Q::one()],
    ];
    let second = vec![
        [Q::zero(), Q::zero(), Q::one()],
        [Q::zero(), Q::zero(), Q::one()],
        [Q::zero(), Q::zero(), Q::one()],
        [Q::one(), Q::zero(), Q::one()],
    ];
    let (k, factors) =
        coordinate_boundary(&first, &second, 0, &Q::new(1.into(), 2.into()), &width).unwrap();
    assert!(coordinate_boundary(&first, &second, 1, &Q::zero(), &width).is_err());
    assert!(coordinate_boundary(&first, &second, 2, &Q::zero(), &width).is_err());
    assert_ne!(k, 0);
    let events = verify_equations(&equations, false, &width, k, &factors).unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].first[0] > Q::new(7.into(), 10.into()));
    assert!(events[0].second[0] > Q::new(79.into(), 100.into()));
    let mut tampered = factors.clone();
    let mapped = tampered.iter_mut().find(|f| f.map.is_some()).unwrap();
    mapped.map = Some(vec![Q::zero()]);
    assert!(verify_equations(&equations, false, &width, k, &tampered).is_err());
    let mut omitted = factors.clone();
    omitted.pop();
    assert!(verify_equations(&equations, false, &width, k, &omitted).is_err());
}
