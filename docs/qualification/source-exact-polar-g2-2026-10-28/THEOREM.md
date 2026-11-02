# Exact geometric jets, parameter correspondence and dense extension

For a positive-weight, original single Bezier surface, first orient indices so the varying natural-meridian coordinate is the first parameter and the fixed meridian is the second parameter's minimum. Index permutation/reversal preserves original scalar bits. The original rational UV-line weights are C,D; native source-to-canonical forward weights are A,B. After canonical orientation, inverse composition has homogeneous endpoint factors CB,DA for a forward map and DA,CB for a reversed map. Multiplying original homogeneous row i by f0^(p-i) f1^i therefore represents the unchanged source under the exact common canonical parameter. These factors are formed by expansion products of original weights, not binary64 multiplication of a proposed combined weight.

A common authored unit leaf keeps homogeneous Cartesian/weight scaling consistent. After an arbitrary original origin translation, write h=(Nx,Ny,Nz,W). Original Bernstein coefficient differences give h_u,h_v,h_uu,h_uv,h_vv at the complete meridian. Power conversion is exact integer algebra. For each Cartesian coordinate k, define

    A_k = (N_k)_u W - N_k W_u,
    B_k = (N_k)_v W - N_k W_v,
    C_(k,ij) = (N_k)_ij W²
                 - [(N_k)_i W_j + (N_k)_j W_i + N_k W_ij] W
                 + 2 N_k W_i W_j.

Thus S_(k,u)=A_k/W², S_(k,v)=B_k/W² and S_(k,ij)=C_(k,ij)/W³. In cyclic world graph axes x,y,z, put

    D = A_x B_y - A_y B_x,
    Gx = A_z B_y - B_z A_y,
    Gy = A_x B_z - B_x A_z,
    R_ij = D C_(z,ij) - Gx C_(x,ij) - Gy C_(y,ij).

Where D is nonzero, Cartesian graph gradients are Gx/D and Gy/D, and Hessians are

    Hxx = W [B_y² R_uu - 2 B_y A_y R_uv + A_y² R_vv] / D³,
    Hxy = W [-B_x B_y R_uu + (B_y A_x + A_y B_x) R_uv - A_x A_y R_vv] / D³,
    Hyy = W [B_x² R_uu - 2 B_x A_x R_uv + A_x² R_vv] / D³.

All five rational-function identities are checked after exact cross multiplication, on every polynomial coefficient. Each source's D must first be proved NOT identically zero; 0/0 can never establish identity. Exact common monomial and homogeneous coefficient scaling reductions preserve ratios. A Different decision means the whole rational jet identity failed, not that every parameter differs. Budget, cancellation and precision failure remain Indeterminate, never Equal.

Nonzero univariate polynomials have only finitely many zeros. The five identities therefore prove matching geometric graph jets on the dense set where both projection denominators are nonzero. The sealed original carrier independently supplies positional correspondence; derivative identity does not supply C0 ownership. Matching graph jets at an identified point give equal unit normal lines and signed world curvature tensors up to one native orientation.

Native interval coverage separately proves nonzero regular normals and finite shape fields on every closed carrier cell, with explicitly factored one-sided limits at supported collapsed ends. Away from the pole a fresh original regular-jet query may cross a projection zero. No unresolved interval is omitted. These continuous native normal/tensor fields extend the dense equality through projection zeros and the proved finite endpoint limits.

The relative native orientation is continuous and takes values +1 or -1 wherever normals are nonzero, so it is constant on the covered connected parameter interval. A certified anchor with normal-distance upper bound below one selects which sign applies; it does not prove equality by sampling. Equality comes from the complete polynomial identity, and acceptance still requires complete native regularity/limit coverage. Normal AND signed tensor use this one sign. Only after all conditions hold may the seam's exact distance bounds be zero.

The complete body factory further requires immutable same-body ownership, the private single-sheeted pole graph atlas and exact common original quadratic jets across all incident charts. This prevents an edgewise identity from granting a rounded sphere an unproved unique pole Hessian. Authored C0 joins remain C0. Exact meridian compatibility alone is not a whole-neighborhood C2 or ideal-source global topology certificate.
