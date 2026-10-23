    use super::*;
    #[test]
    fn differentiated_tensor_endpoints_cover_an_independent_monomial() {
        let mut work = Work {
            used: 0,
            max: 10000,
        };
        let mut polynomial = vec![vec![I::point(0.); 4]; 3];
        polynomial[2][3] = I::point(1.);
        // d_u d_v(u² v³)=6u v². Differentiation must precede endpoint
        // restriction; the V=1 edge is 6u, while V=0 is identically zero.
        let du = difference(&polynomial, 0, &mut work).unwrap();
        let mixed = difference(&du, 1, &mut work).unwrap();
        let zero = enclose(&mixed, [[0.25, 0.75], [0., 0.]], &mut work).unwrap();
        assert!(zero.contains(0.) && zero.lo.abs().max(zero.hi.abs()) < 1e-12);
        let end = enclose(&mixed, [[0.25, 0.75], [1., 1.]], &mut work).unwrap();
        assert!(end.contains(1.5) && end.contains(4.5) && end.hi - end.lo < 3. + 1e-12);
    }
    #[test]
    fn coupled_products_and_quotients_cover_independent_rational_residuals() {
        let mut work = Work {
            used: 0,
            max: 10000,
        };
        let a = Coupled {
            base: I::point(1.25),
            delta: I::point(1. / 32.),
        };
        let b = Coupled {
            base: I::point(0.75),
            delta: I::point(-1. / 16.),
        };
        let product = a.mul(b, &mut work).unwrap();
        // Target product is 451/512, base is 480/512; the mixed
        // delta_a*delta_b term is required to contain -29/512.
        assert!(product.delta.contains(-29. / 512.));
        let ratio = a.div(b, &mut work).unwrap();
        // Target ratio is 41/22, base is 5/3, residual is 13/66.
        assert!(ratio.base.contains(5. / 3.));
        assert!(ratio.delta.contains(13. / 66.));
    }
    #[test]
    fn paired_shape_detects_independent_monge_curvature_with_unchanged_first_jets() {
        let mut work = Work {
            used: 0,
            max: 10000,
        };
        let mut jets = [[[Coupled::zero(); 3]; 3]; 3];
        jets[1][0][0].base = I::point(1.);
        jets[0][1][1].base = I::point(1.);
        // Candidate is a plane; original composition is z=u²/16.
        // At the origin both positions and normals agree, B_xx=1/8.
        jets[2][0][2].delta = I::point(0.125);
        let (normal, tensor) = shape(&jets, &mut work).unwrap().unwrap();
        for x in normal {
            assert!(x.delta.contains(0.));
        }
        for (i, x) in tensor.iter().enumerate() {
            assert!(x.delta.contains(if i == 0 { 0.125 } else { 0. }));
        }
        assert!(tensor[0].delta.lo > 0.124999 && tensor[0].delta.hi < 0.125001);
    }
