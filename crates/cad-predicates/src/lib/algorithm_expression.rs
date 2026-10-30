//! lib algorithm: expression.
use super::*;
pub fn expression<T: Algebra>(
    kind: Predicate,
    values: &[T],
    ctx: &mut PredicateContext<'_>,
) -> Result<T, Reason> {
    match kind {
        Predicate::Orient2 => {
            let x = values[2].sub(&values[0], ctx)?;
            let y = values[3].sub(&values[1], ctx)?;
            let u = values[4].sub(&values[0], ctx)?;
            let v = values[5].sub(&values[1], ctx)?;
            x.mul(&v, ctx)?.sub(&y.mul(&u, ctx)?, ctx)
        }
        Predicate::InCircle2 => {
            let adx = values[0].sub(&values[6], ctx)?;
            let ady = values[1].sub(&values[7], ctx)?;
            let bdx = values[2].sub(&values[6], ctx)?;
            let bdy = values[3].sub(&values[7], ctx)?;
            let cdx = values[4].sub(&values[6], ctx)?;
            let cdy = values[5].sub(&values[7], ctx)?;
            let alift = adx.mul(&adx, ctx)?.add(&ady.mul(&ady, ctx)?, ctx)?;
            let blift = bdx.mul(&bdx, ctx)?.add(&bdy.mul(&bdy, ctx)?, ctx)?;
            let clift = cdx.mul(&cdx, ctx)?.add(&cdy.mul(&cdy, ctx)?, ctx)?;
            let bc = bdx.mul(&cdy, ctx)?.sub(&bdy.mul(&cdx, ctx)?, ctx)?;
            let ca = cdx.mul(&ady, ctx)?.sub(&cdy.mul(&adx, ctx)?, ctx)?;
            let ab = adx.mul(&bdy, ctx)?.sub(&ady.mul(&bdx, ctx)?, ctx)?;
            alift
                .mul(&bc, ctx)?
                .add(&blift.mul(&ca, ctx)?, ctx)?
                .add(&clift.mul(&ab, ctx)?, ctx)
        }
        Predicate::Orient3 => {
            let mut vectors = Vec::with_capacity(9);
            for i in 3..12 {
                vectors.push(values[i].sub(&values[i % 3], ctx)?);
            }
            let x = vectors[4]
                .mul(&vectors[8], ctx)?
                .sub(&vectors[5].mul(&vectors[7], ctx)?, ctx)?;
            let y = vectors[5]
                .mul(&vectors[6], ctx)?
                .sub(&vectors[3].mul(&vectors[8], ctx)?, ctx)?;
            let z = vectors[3]
                .mul(&vectors[7], ctx)?
                .sub(&vectors[4].mul(&vectors[6], ctx)?, ctx)?;
            vectors[0]
                .mul(&x, ctx)?
                .add(&vectors[1].mul(&y, ctx)?, ctx)?
                .add(&vectors[2].mul(&z, ctx)?, ctx)
        }
        Predicate::InSphere3 => {
            let aex = values[0].sub(&values[12], ctx)?;
            let aey = values[1].sub(&values[13], ctx)?;
            let aez = values[2].sub(&values[14], ctx)?;
            let bex = values[3].sub(&values[12], ctx)?;
            let bey = values[4].sub(&values[13], ctx)?;
            let bez = values[5].sub(&values[14], ctx)?;
            let cex = values[6].sub(&values[12], ctx)?;
            let cey = values[7].sub(&values[13], ctx)?;
            let cez = values[8].sub(&values[14], ctx)?;
            let dex = values[9].sub(&values[12], ctx)?;
            let dey = values[10].sub(&values[13], ctx)?;
            let dez = values[11].sub(&values[14], ctx)?;

            let ab = aex.mul(&bey, ctx)?.sub(&bex.mul(&aey, ctx)?, ctx)?;
            let bc = bex.mul(&cey, ctx)?.sub(&cex.mul(&bey, ctx)?, ctx)?;
            let cd = cex.mul(&dey, ctx)?.sub(&dex.mul(&cey, ctx)?, ctx)?;
            let da = dex.mul(&aey, ctx)?.sub(&aex.mul(&dey, ctx)?, ctx)?;
            let ac = aex.mul(&cey, ctx)?.sub(&cex.mul(&aey, ctx)?, ctx)?;
            let bd = bex.mul(&dey, ctx)?.sub(&dex.mul(&bey, ctx)?, ctx)?;

            let abc = aez
                .mul(&bc, ctx)?
                .sub(&bez.mul(&ac, ctx)?, ctx)?
                .add(&cez.mul(&ab, ctx)?, ctx)?;
            let bcd = bez
                .mul(&cd, ctx)?
                .sub(&cez.mul(&bd, ctx)?, ctx)?
                .add(&dez.mul(&bc, ctx)?, ctx)?;
            let cda = cez
                .mul(&da, ctx)?
                .add(&dez.mul(&ac, ctx)?, ctx)?
                .add(&aez.mul(&cd, ctx)?, ctx)?;
            let dab = dez
                .mul(&ab, ctx)?
                .add(&aez.mul(&bd, ctx)?, ctx)?
                .add(&bez.mul(&da, ctx)?, ctx)?;

            let alift = aex
                .mul(&aex, ctx)?
                .add(&aey.mul(&aey, ctx)?, ctx)?
                .add(&aez.mul(&aez, ctx)?, ctx)?;
            let blift = bex
                .mul(&bex, ctx)?
                .add(&bey.mul(&bey, ctx)?, ctx)?
                .add(&bez.mul(&bez, ctx)?, ctx)?;
            let clift = cex
                .mul(&cex, ctx)?
                .add(&cey.mul(&cey, ctx)?, ctx)?
                .add(&cez.mul(&cez, ctx)?, ctx)?;
            let dlift = dex
                .mul(&dex, ctx)?
                .add(&dey.mul(&dey, ctx)?, ctx)?
                .add(&dez.mul(&dez, ctx)?, ctx)?;

            alift
                .mul(&bcd, ctx)?
                .sub(&blift.mul(&cda, ctx)?, ctx)?
                .add(&clift.mul(&dab, ctx)?, ctx)?
                .sub(&dlift.mul(&abc, ctx)?, ctx)
        }
        Predicate::DirectionDot3 => {
            let product = |k: usize, ctx: &mut PredicateContext<'_>| {
                values[3 + k].sub(&values[k], ctx)?.mul(
                    &values[9 + k].sub(&values[6 + k], ctx)?, ctx,
                )
            };
            product(0, ctx)?.add(&product(1, ctx)?, ctx)?.add(&product(2, ctx)?, ctx)
        }
        Predicate::Distance(dimension) => {
            let first = values[0].sub(&values[dimension], ctx)?;
            let mut result = first.mul(&first, ctx)?;
            for i in 1..dimension {
                let difference = values[i].sub(&values[dimension + i], ctx)?;
                result = result.add(&difference.mul(&difference, ctx)?, ctx)?;
            }
            result.sub(
                &values[2 * dimension].mul(&values[2 * dimension], ctx)?,
                ctx,
            )
        }
    }
}
