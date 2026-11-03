//! Exact normalized knot cells; only diagnostics are rounded to binary64.
use super::*;
use num_traits::ToPrimitive;
pub(super) fn full_cell() -> Cell {
    std::array::from_fn(|_| [q(0.).unwrap(), q(1.).unwrap()])
}
pub(super) fn cut(cell: &Cell, axis: usize) -> Result<[Cell; 2]> {
    check(axis < 2, "Invalid reduced coverage split axis")?;
    let mid = (&cell[axis][0] + &cell[axis][1]) / Q::from_integer(2.into());
    check(
        cell[axis][0] < mid && mid < cell[axis][1],
        "Invalid exact reduced parameter cell",
    )?;
    let mut out = [cell.clone(), cell.clone()];
    out[0][axis][1] = mid.clone();
    out[1][axis][0] = mid;
    Ok(out)
}
pub(super) fn diagnostic(cell: &Cell, domain: Box4) -> Result<Box4> {
    let mut out = [[0.; 2]; 4];
    for k in 0..4 {
        let lo = q(domain[k][0])?;
        let width = q(domain[k][1])? - &lo;
        for end in 0..2 {
            let exact = &lo + &width * &cell[k % 2][end];
            let v = exact
                .to_f64()
                .filter(|x| x.is_finite())
                .ok_or_else(|| crate::input("Nonfinite original coverage diagnostic"))?;
            let outward = if end == 0 && q(v)? > exact {
                v.next_down()
            } else if end == 1 && q(v)? < exact {
                v.next_up()
            } else {
                v
            };
            out[k][end] = outward.max(domain[k][0]).min(domain[k][1]);
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nondyadic_cuts_have_no_gaps_and_only_diagnostics_round() {
        let third = Q::new(1.into(), 3.into());
        let cell = [
            [third.clone(), Q::new(2.into(), 3.into())],
            [q(0.).unwrap(), q(1.).unwrap()],
        ];
        let children = cut(&cell, 0).unwrap();
        assert_eq!(children[0][0][1], children[1][0][0]);
        assert_eq!(children[0][0][0], third);
        assert_eq!(children[1][0][1], cell[0][1]);
        let d = diagnostic(&cell, [[0., 1.]; 4]).unwrap();
        for k in 0..4 {
            assert!(q(d[k][0]).unwrap() <= cell[k % 2][0] && q(d[k][1]).unwrap() >= cell[k % 2][1]);
        }
    }
}
