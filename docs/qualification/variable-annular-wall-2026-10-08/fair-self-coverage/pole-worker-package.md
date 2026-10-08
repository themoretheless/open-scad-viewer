# Packaged original wall refinement

WASM SHA256: 1fb390e8662a59a8d6f8d3948f4618798ac68041d5a40774d6185c525fbb3175. Raw size 12,076,288 bytes.
The real worker test passes all nine request variants in 18.64 seconds, retaining earlier facts and rejecting forged or stale result packets. Its expanded case confirms 26/27 self faces and leaves 161 unproven pairs, matching native results. Whole-wall qualification remains false.
The separate cancellation/late-response/Retry test uses a controlled port and passes; this is not a browser interaction qualification.
The package audit confirms an exact compressed round trip and no external WASM imports.
The interface now requests the expanded ordinary and pole budgets from the Check whole wall action. TypeScript checks pass. Rendered UI verification is pending.

Production build and verifier pass: 152 artifacts, 8,374,409 asset bytes plus 15,372,680 raw WASM bytes. Named geometry and worker budgets were updated to measured sizes with the existing 608/123-byte margins; total retains its 761-byte margin.
