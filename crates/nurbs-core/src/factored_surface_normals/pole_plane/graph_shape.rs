//! Original polar graph Hessian and curvature bounds after proved cancellation.
use super::*;
mod polynomial;
use polynomial::{Polynomial, Work, derivative as diff, divide_known_radial_zero as divide_r};
pub struct PolarGraphShape<'a> {
    surface: &'a Surface,
    boundary: Boundary,
    axis: usize,
    weight: Polynomial,
    radial: [Polynomial; 3],
    angular: [Polynomial; 3],
    normal: [Polynomial; 3],
    gradient_r: [Polynomial; 2],
    gradient_a_over_r: [Polynomial; 2],
    determinant: Polynomial,
}
impl<'a> PolarGraphShape<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
}
pub struct PolarGraphShapeReport<'a> {
    pub certificate: Option<PolarGraphShape<'a>>,
    pub products: u64,
    pub reason: &'static str,
}
pub struct PolarGraphBounds {
    /// Cyclic world coordinates x=(axis+1)%3, y=(axis+2)%3, z=axis.
    pub gradient: [[f64; 2]; 2],
    /// Hxx,Hxy,Hyy. At the collapsed side these enclose the angular family of
    /// one-sided limits, not a unique Hessian or a C2 quotient neighborhood.
    pub hessian: [[f64; 2]; 3],
    /// Original S_u cross S_v orientation, also used by the signed tensor.
    pub normal: [[f64; 2]; 3],
    pub tensor: [[[f64; 2]; 3]; 3],
}
pub struct PolarGraphBoundsReport {
    pub bounds: Option<PolarGraphBounds>,
    pub coefficients: usize,
    pub reason: &'static str,
}
impl<'a> PlaneLimit<'a> {
    /// Build full original polynomial fields, never sampled derivative fits.
    /// Only first-order normal factors and a nonzero leading projection apply.
    /// The common exact plane proves the angular gradient numerator's leading
    /// row zero, allowing division by r BEFORE any rectangle evaluation.
    pub fn prepare_graph_shape(
        &self,
        axis: usize,
        max_products: u64,
    ) -> Result<PolarGraphShapeReport<'a>> {
        check(
            axis < 3 && (1..=100000000).contains(&max_products),
            "Choose a world polar projection and bounded polynomial products",
        )?;
        let mut out = PolarGraphShapeReport {
            certificate: None,
            products: 0,
            reason: "polar-graph-shape-first-order-required",
        };
        if self.order != 1 {
            return Ok(out);
        }
        if self.normal[axis][0] <= 0. && self.normal[axis][1] >= 0. {
            out.reason = "polar-graph-shape-leading-projection-unproven";
            return Ok(out);
        }
        let mut work = Work {
            used: 0,
            limit: max_products,
        };
        let build = (|| -> Result<PolarGraphShape<'a>> {
            let view = mirrored(self.surface, self.boundary);
            let weight = view
                .weights
                .iter()
                .map(|row| row.iter().map(|&x| I::point(x)).collect())
                .collect::<Polynomial>();
            let mut shifted = Vec::new();
            for k in 0..3 {
                shifted.push(
                    view.control_points
                        .iter()
                        .enumerate()
                        .map(|(i, row)| {
                            row.iter()
                                .enumerate()
                                .map(|(j, p)| {
                                    if i == 0 {
                                        Ok(I::point(0.))
                                    } else {
                                        I::point(p[k])
                                            .sub(I::point(self.point[k]))?
                                            .mul(weight[i][j])
                                    }
                                })
                                .collect()
                        })
                        .collect::<Result<Polynomial>>()?,
                );
            }
            let wu = diff(&weight, 0)?;
            let wa = diff(&weight, 1)?;
            let mut radial = Vec::new();
            let mut angular = Vec::new();
            for n in &shifted {
                radial.push(work.difference_products(&diff(n, 0)?, &weight, n, &wu)?);
                // Original constant boundary proves the shifted numerator
                // row exactly zero, regardless of its positive authored weights.
                let m = divide_r(n)?;
                angular.push(work.difference_products(&diff(&m, 1)?, &weight, &m, &wa)?);
            }
            let radial: [Polynomial; 3] = radial.try_into().unwrap();
            let angular: [Polynomial; 3] = angular.try_into().unwrap();
            let normal: [Polynomial; 3] = std::array::from_fn(|k| {
                self.net
                    .iter()
                    .map(|row| row.iter().map(|p| p[k]).collect())
                    .collect()
            });
            let z = &normal[axis];
            let zu = diff(z, 0)?;
            let za = diff(z, 1)?;
            let mut gradient_r = Vec::new();
            let mut gradient_a_over_r = Vec::new();
            for k in [(axis + 1) % 3, (axis + 2) % 3] {
                gradient_r.push(work.difference_products(
                    &normal[k],
                    &zu,
                    &diff(&normal[k], 0)?,
                    z,
                )?);
                let angular_numerator =
                    work.difference_products(&normal[k], &za, &diff(&normal[k], 1)?, z)?;
                // EXACT parallelism of every original leading normal
                // coefficient proves this full first radial row zero.
                gradient_a_over_r.push(divide_r(&angular_numerator)?);
            }
            let x = (axis + 1) % 3;
            let y = (axis + 2) % 3;
            let determinant =
                work.difference_products(&radial[x], &angular[y], &radial[y], &angular[x])?;
            Ok(PolarGraphShape {
                surface: self.surface,
                boundary: self.boundary,
                axis,
                weight,
                radial,
                angular,
                normal,
                gradient_r: gradient_r.try_into().unwrap(),
                gradient_a_over_r: gradient_a_over_r.try_into().unwrap(),
                determinant,
            })
        })();
        out.products = work.used;
        match build {
            Ok(c) => {
                out.certificate = Some(c);
                out.reason = "original-polar-graph-shape-polynomials-qualified";
            }
            Err(e) if e.code == crate::RESOURCE_LIMIT => {
                out.reason = "polar-graph-shape-product-limit"
            }
            Err(e) if e.code == crate::NUMERIC_ERROR => {
                out.reason = "polar-graph-shape-polynomial-interval-unresolved"
            }
            Err(e) => return Err(e),
        }
        Ok(out)
    }
}
impl PolarGraphShape<'_> {
    fn fields(&self) -> Vec<&Polynomial> {
        let mut fields = vec![&self.weight, &self.determinant];
        fields.extend(self.radial.iter());
        fields.extend(self.angular.iter());
        fields.extend(self.normal.iter());
        fields.extend(self.gradient_r.iter());
        fields.extend(self.gradient_a_over_r.iter());
        fields
    }
    /// Bound the unchanged world graph derivatives and signed graph curvature
    /// over an original CLOSED rectangle. At r=0 the canceled expressions
    /// enclose limits from r>0. No independent C2/G2 admission is supplied.
    pub fn rectangle_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_coefficients: usize,
    ) -> Result<PolarGraphBoundsReport> {
        check(
            (1..=100000000).contains(&max_coefficients),
            "Choose bounded original polar shape coefficients",
        )?;
        let s = self.surface;
        let original = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        check(
            (0..2).all(|a| {
                domain[a][0].is_finite()
                    && domain[a][1].is_finite()
                    && original[a][0] <= domain[a][0]
                    && domain[a][0] <= domain[a][1]
                    && domain[a][1] <= original[a][1]
            }),
            "Use a closed original polar shape rectangle",
        )?;
        let cost = self.fields().iter().map(|p| p.len() * p[0].len()).sum();
        let mut out = PolarGraphBoundsReport {
            bounds: None,
            coefficients: 0,
            reason: "polar-graph-shape-coefficient-limit",
        };
        if cost > max_coefficients {
            return Ok(out);
        }
        out.coefficients = cost;
        let (axis, end) = axes(self.boundary);
        let query = (|| -> Result<Option<PolarGraphBounds>> {
            let mut range = [[I::point(0.); 2]; 2];
            for a in 0..2 {
                let length = I::point(original[a][1]).sub(I::point(original[a][0]))?;
                for e in 0..2 {
                    range[a][e] = I::point(domain[a][e])
                        .sub(I::point(original[a][0]))?
                        .div(length)?;
                }
            }
            if end {
                range[axis] = [
                    I::point(1.).sub(range[axis][1])?,
                    I::point(1.).sub(range[axis][0])?,
                ];
            }
            let range = [range[axis], range[1 - axis]];
            let point = [
                domain[axis][0] == domain[axis][1],
                domain[1 - axis][0] == domain[1 - axis][1],
            ];
            let evaluate = |p: &Polynomial| polynomial::bounds(p, range, point);
            let x = (self.axis + 1) % 3;
            let y = (self.axis + 2) % 3;
            let w = evaluate(&self.weight)?;
            let nz = evaluate(&self.normal[self.axis])?;
            let determinant = evaluate(&self.determinant)?;
            if w.lo <= 0.
                || (nz.lo <= 0. && nz.hi >= 0.)
                || (determinant.lo <= 0. && determinant.hi >= 0.)
            {
                return Ok(None);
            }
            let neg = |a: I| I::new(-a.hi, -a.lo);
            let gradient = [
                neg(evaluate(&self.normal[x])?)?.div_signed(nz)?,
                neg(evaluate(&self.normal[y])?)?.div_signed(nz)?,
            ];
            let arx = evaluate(&self.radial[x])?;
            let ary = evaluate(&self.radial[y])?;
            let bax = evaluate(&self.angular[x])?;
            let bay = evaluate(&self.angular[y])?;
            let ku = [
                evaluate(&self.gradient_r[0])?,
                evaluate(&self.gradient_r[1])?,
            ];
            let ka = [
                evaluate(&self.gradient_a_over_r[0])?,
                evaluate(&self.gradient_a_over_r[1])?,
            ];
            let denominator = nz.mul(nz)?.mul(determinant)?;
            let w2 = w.mul(w)?;
            let h = [
                ku[0]
                    .mul(bay)?
                    .sub(ka[0].mul(ary)?)?
                    .mul(w2)?
                    .div_signed(denominator)?,
                ka[0]
                    .mul(arx)?
                    .sub(ku[0].mul(bax)?)?
                    .mul(w2)?
                    .div_signed(denominator)?,
                ka[1]
                    .mul(arx)?
                    .sub(ku[1].mul(bax)?)?
                    .mul(w2)?
                    .div_signed(denominator)?,
            ];
            let mut jets = [[[I::point(0.); 3]; 4]; 4];
            jets[1][0][x] = I::point(1.);
            jets[0][1][y] = I::point(1.);
            jets[1][0][self.axis] = gradient[0];
            jets[0][1][self.axis] = gradient[1];
            jets[2][0][self.axis] = h[0];
            jets[1][1][self.axis] = h[1];
            jets[0][2][self.axis] = h[2];
            let Some(mut shape) = crate::surface_shape_operator::from_jets(&jets)? else {
                return Ok(None);
            };
            // The private factored net retains original source orientation.
            // Graph coordinates have positive dropped-axis normal by cyclic
            // construction; flip normal AND signed tensor together if needed.
            if nz.hi < 0. {
                for n in &mut shape.normal {
                    *n = neg(*n)?;
                }
                for row in &mut shape.tensor {
                    for t in row {
                        *t = neg(*t)?;
                    }
                }
            }
            Ok(Some(PolarGraphBounds {
                gradient: gradient.map(|v| [v.lo, v.hi]),
                hessian: h.map(|v| [v.lo, v.hi]),
                normal: shape.normal.map(|v| [v.lo, v.hi]),
                tensor: shape.tensor.map(|row| row.map(|v| [v.lo, v.hi])),
            }))
        })();
        match query {
            Ok(Some(b)) => {
                out.bounds = Some(b);
                out.reason = "original-polar-graph-curvature-bounds-qualified";
            }
            Ok(None) => out.reason = "polar-graph-shape-projection-unresolved",
            Err(e) if e.code == crate::NUMERIC_ERROR => {
                out.reason = "polar-graph-shape-interval-unresolved"
            }
            Err(e) => return Err(e),
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests;

mod pair;
pub use pair::{PolarShapePairReport, compare_polar_shape_rectangles};
