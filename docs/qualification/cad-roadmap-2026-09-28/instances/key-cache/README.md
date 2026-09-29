# Bounded encoding of repeated CAD field names

A Node CPU profile of the 1000-instance benchmark showed substantial self time in `valueBinaryCodec` and UTF-8 encoding/decoding. The encoder now caches up to 256 field names, each at most 128 UTF-16 code units, for one encode call. Values are not cached; wire format and validation are unchanged. There is no retained geometry cache.

A paired benchmark alternated old/new order over 20 measured runs after two warmups. The payload contains 32 materialized CAD bodies. Median encoding time: 7.82 → 4.35 ms; output bytes compared equal. Raw samples: `codec.json`.

End-to-end samples (`before.json`, `after.json`) for 1000 instances: commit 2043 → 1927 ms; Undo 1276 → 1229 ms; Redo 1301 → 1231 ms. These single complete-operation samples establish remaining latency, not a statistically qualified end-to-end speedup. Performance is still too slow for interactive editing. Node results are not browser FPS or heap measurements.

Reproduce with an original codec snapshot supplied explicitly:

```sh
node --import tsx scripts/benchmark-solid-instances.mts /tmp/cad-instances 1000
node --import tsx scripts/benchmark-cad-codec.mts /tmp/cad-instances/instances-1000.json /path/to/baseline-codec.ts /tmp/codec-report.json
```
