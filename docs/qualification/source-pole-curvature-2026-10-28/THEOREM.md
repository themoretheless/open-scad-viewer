# Transfer from an exact polar jet to facewise curvature limits

Let each admitted original rational chart be S(u,v), with its pole at u=0 and v in a compact natural interval. Positive weights make S analytic in a neighborhood of the parameter rectangle. Coordinates are cyclic world coordinates x,y,z relative to the exact original pole, with z dropped for projection.

Write its homogeneous numerator as N=u N1+u² N2+O(u³) and denominator as W=W0+u W1+O(u²), with W0 positive. The predicate verifies, identically for every v,

    Z1 = gx X1 + gy Y1
    2 (Z2 - gx X2 - gy Y2) W0
      = Hxx X1² + 2 Hxy X1 Y1 + Hyy Y1².

All operations and denominator-cleared coefficient comparisons use exact expansions over original binary64 inputs. Thus the analytic remainder

    z - gx x - gy y - (Hxx x² + 2 Hxy xy + Hyy y²)/2

is u³ R(u,v), uniformly with its parameter derivatives. Denominator terms involving W1 cancel by the first identity.

The independently admitted graph atlas supplies a single-sheeted projected disk, exact common pole, finite incident fan and quantitative transverse collars. The additional first-order normal-factor requirement gives det D(x,y)=u B(u,v), with B uniformly separated from zero. Therefore the inverse derivatives satisfy u_x,u_y=O(1), v_x,v_y=O(1/u), second derivatives of u are O(1/u), and those of v are O(1/u²), uniformly on each face collar. These follow directly by differentiating the inverse matrix, using x,y=u A(u,v) and the lower bound on B.

Applying the chain rule to u³ R gives projected first derivatives O(u²) and second derivatives O(u). The original quadratic polynomial therefore supplies the facewise graph Hessian limit H. The finite collection and exact comparison of all private gradients/Hessians give common limits across all incident faces. The common graph gradient fixes the tangent plane; graph Hessians and that gradient determine the same limiting second fundamental form with a consistent graph orientation.

This establishes a common quadratic expansion and facewise curvature limit at the pole. It does not remove possible C0 ordinary seams, prove a C2 neighborhood, or promote an independently unqualified seam to G2. Higher-order normal factors, unsupported charts, incomplete ownership, unequal jets and insufficient work all retain explicit refusal.
