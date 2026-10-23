# Archive bulk selection validation

2026-10-23, local Windows development build. Internal private-preview record.

Selection now requests up to 1,000 lightweight ID/revision/title rows per page (previously 100). Normal library page limits stay at 100. The client falls back to 100 for older servers, reports loaded counts, checks the library revision between pages, and ignores cancelled replies. Existing explicit ID selection and revision-checked destructive operations remain unchanged.

Read-only HTTP measurements on the existing archive, 27,326 games, unchanged revision 25914:

| Case | Requests | Elapsed |
| --- | ---: | ---: |
| Previous server, batches of 100 | 274 | 759 ms |
| Updated server, batches of 1,000 | 28 | 210 ms |
| Updated server, batches of 100 | 274 | 688 ms |

All three returned the same ordered ID SHA-256: 4ad9e30fd130e652816b0bb571a6c496e9a90feda5e48c0da52690d93d09bb04. Single-run local measurements; excludes browser rendering and is not a guarantee for other machines or networks. No game documents, notes or analyses were changed or exported. Credential stayed in memory.

267 JavaScript tests and 40 native library tests passed. Required WASM build and site refresh passed. Default development Companion package rebuilt at output/companion/current, preserving its data directory and private update channel; restarted and healthcheck passed. No push, deployment or public release changes.
