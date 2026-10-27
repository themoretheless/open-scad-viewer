# Exact historical archive restoration

PR 61 CI failed the immutable archive check on the inherited v75-v92 script. A full inventory found 47 mismatches against the existing frozen hash registry. Every file is restored from an available byte-identical archive or a verified historical Git blob; the expected hashes are unchanged.

`previous-variants.tar.gz` preserves every replaced variant at its original path. `restoration.json` records previous and expected SHA-256 values and the exact recovery source. This restores historical identities and does not qualify the current source or WASM, or claim semantic G1 completion.

Verification: all existing archive hash entries now match; four architecture-auditor unit tests pass. The complete architecture audit still fails on inherited size/layer baseline violations; the raw result is retained in `architecture-audit.log.gz`. This is a partial CI repair.
