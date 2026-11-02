# Distributed C2 tile construction with a curvature gate

`propose_curvature_limited_tile` changes every interior pole, retaining three
control layers at each side. It requires four fresh whole-seam G2 receipts,
continuous closure, and a complete trimmed-face principal-curvature cover
before returning a candidate. `Exceeds`, `Unresolved`, invalid or exhausted
budgets return an error without changing the input model. This does not grant
material containment, a radius law, global embedding, or whole-solid admission.
The original single-pole constructor remains available with its prior scope.
The proposal records all modified poles, not only the middle pole.

`certify_tile_rectangle_inside` proves ownership by a finite complete union of
closed Inside cells when a parent rectangle cannot be classified. Every
Unresolved cell must be split. Any Outside cell, precision limit, 4096 visited
regions or exhausted shared domain budget refuses. It cannot ignore holes or
replace coverage by midpoint checks.

## Native evidence

13 focused tests passed: seven curvature controls and six tile controls. The
new cases cover a distributed degree-nine closed tile with four G2 seams and
a complete curvature receipt, strict-curvature rejection, zero-work rejection,
hole rejection under ownership subdivision, and immutable inputs. The control
is not the production three-radius body. See `native-tests.log` and the source
hashes in `status.json`. The wider dirty workspace was not fully qualified.

## Actual three-radius fixture: still rejected

The offline Rust `fair_tile_search` example tried 27 fixed form proposals,
including square, anisotropic and offset windows, normal-chart lifts and
Y-only lifts. Limits were not loosened: the declared principal magnitude cap
remained 10/mm and original cube allowance remained 1e-7 mm. Search reports
are separate from topology/G2 admission and explicitly say `G2Qualified=false`.
No sampled point admits a surface.

At the original bad point, distributing and widening the correction reduced
the certified magnitude from the previous roughly 214.966/mm to roughly
10.862/mm in one Y-only trial. This is a POINT bound, not a maximum or a passing
result. Automatic checks found nearby excesses in other trials. The original
unmodified central surface itself has a certified Inside witness of
[11.158094087107578, 11.158094088889547]/mm at UV
[0.9408259112081446, 0.4836167378919158]. Thus the defect predates the tiny tile.

The asymmetric rectangle U [0.8730759112081445, 0.9530759112081446],
V [0.4596167378919158, 0.5396167378919158] did receive full Inside ownership.
However, lifting its original witness to 0.001 mm produced a certified
46.8293/mm excess, and the material check remained Outside. Some retained
corner poles are already outside the cube allowance. For clamped tensor
supports the endpoint corners equal these poles exactly; moving only interior
poles cannot repair those corners. This is not an impossibility proof for
other deformations or moving shared boundaries.

No production candidate has been accepted or substituted into a live document.
The remaining construction must jointly change the retained central region
and adjacent transition boundaries, then repeat whole-seam G2, continuous
curvature, material, thickness and self-intersection checks. General NURBS
fillets, end transitions, unequal-radius corners, STEP/history and WASM/UI
acceptance remain separate open work.

## Reproduction

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --test interior_tile --test face_curvature
cargo build --offline --manifest-path crates/Cargo.toml -p brep-core --example fair_tile_search
fair_tile_search SOURCE.graft.json OUTPUT.json
```

The example is an offline fixture experiment. Optional `OSV_TILE_LIFT_DIRECTION`,
`OSV_TILE_HALF_WIDTHS`, `OSV_TILE_V_RATIO`, `OSV_TILE_CENTER_SHIFT` and
`OSV_TILE_TARGET` make the recorded finite parameter variants reproducible.
Reports omit the large trial surfaces in this documentation folder; source
outputs and individual surface hashes are retained separately in qualification
output. No build cache or generated STEP is added here.
