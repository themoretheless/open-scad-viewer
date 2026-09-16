# G0 v5 / G1 v22 MCP manifest-selection re-freeze

After G0 v4 and G1 v21 were frozen, `src/mcp/createServer.ts` was corrected to
select `ACTIVE_GEOMETRY_MANIFEST_VERSIONS` for runtime publication instead of
the baseline-pending `CURRENT_GEOMETRY_MANIFEST_VERSIONS`. That source is bound
by the G1 candidate bundle, so v21 must remain immutable and cannot represent
the corrected bytes.

G0 v1-v4, G1 v1-v21, and all prior status records remain byte-immutable
historical evidence. G0 v5 records the current toolchain-bound bytes without
closing G0. G1 v22 preserves the complete finite v21 contract and binding
membership, recomputes current digests, and starts with zero completed clean
runs and zero completed work units.

Prior results remain discovery-only and cannot be imported.
`qualificationClaim` remains `none`, qualification approval remains
`not-approved`, u07 remains unresolved, and production cutover remains
prohibited. This freeze does not execute qualification work or create evidence.
