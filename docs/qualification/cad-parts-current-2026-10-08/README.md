# Current packaged CAD part acceptance — 2026-10-08

Geometry WASM SHA-256:
`d00b9f1dd638fcb6fe1477d849735a8b7230bbce066ddcbd8214f4db1d23e2b7`.

The current production build passed mouse histories for bracket, enclosure and
rational flange: 20 edits, seven cancelled previews, every Undo/Redo snapshot,
and exact exported-document equality after reload. STEP menu export, import
and re-export passed for all three final documents without changing the source.

Independent OpenCascade 8.0.1.0.0 passed all six fresh STEP files: solid validity,
solid count, bounding dimensions and volume against the existing numerical
references. Maximum bounding error was 1.000000082740371e-7 mm. See
`mouse-step/occt-report.json` and `mouse-step/occt-preview.png`. STEP files are
stored as deterministic gzip archives; decompress to their manifest filenames
before re-running the independent verifier. Manifest hashes refer to the
uncompressed STEP payloads.

These are controlled existing part chains. General curved fillets, general
B-rep self-intersection and wall qualification, the complete command matrix,
and P2/P3 remain unproven. Keyboard histories also passed all three chains, including 20 edits, seven
preview cancellations, every Undo/Redo state and exact reload. The collector
checked 422 documents across both input methods, 140 distinct payloads, unique
topology identities and every snapshot bound within 1e-6 mm. See
`history/history-audit.json` and its Brotli document archive. Keyboard STEP menu
export/import/re-export passed six files; independent OpenCascade measurements passed all six keyboard exports too,
with the same dimension, volume and solid-validity checks. See `keyboard-step/`.

## Draft recovery and concurrent writers

The current production UI passed the isolated persistent-browser draft suite:
three concurrent writers preserve local/shared conflict versions and local Undo;
closing a queued writer does not publish its abandoned state; aborted snapshots
retain the previous revision; renderer crashes immediately before/after
publication recover; missing/corrupt caches and heads preserve source state and
support Retry; real browser quota refusal and large-snapshot Retry pass. The
same isolated profile survives browser restart. Quota was deliberately set to
one byte via Chromium's storage override; ordinary disk exhaustion is not
claimed. See `recovery/report.json` and the hashed JSON snapshot archive.
This qualifies these persistence scenarios, not the complete P0 command matrix.
