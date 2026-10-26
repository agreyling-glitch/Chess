# Browser .7z import validation

Browse and drag-and-drop share `ironwoodOpenSevenZipFile`. The existing vendored
Apache-2.0 sevenz-rust2 decoder now also builds into browser WASM. Worker-only
exports use bounded, buffered Read/Seek callbacks against FileReaderSync and
stream extracted output to opaque OPFS entry files. No source files are changed.
PGNs and Scid 4 return to the existing review flow; Scid 5 opens the existing
range dialog, retaining temporary backing files until that dialog closes.

The extraction dialog reports progress and supports cancellation by terminating
the worker. Header validation and CRC-checked extraction must finish before
any input reaches game review. Paths, duplicate names, links, entry count,
dictionary memory, expanded byte counts, Scid grouping and browser quota are
bounded. File paths inside an archive never become filesystem output paths.
Temporary files are removed after handoff, cancellation/error, Scid window close,
and best-effort non-cached page exit. PGNs/Scid 4 retain the 64 MiB review limit;
Scid 5 may use up to 64 GiB of expanded staging data subject to browser quota.
Encrypted/unsupported codecs and ambiguous mixed/multiple databases require
manual extraction. The decoder operates locally without Companion.

Validation: 62 browser/WASM tests passed covering existing ZIP/Scid imports,
selection, pacing/storage, Browse handoff and the new archive driver. Real
rebuilt WASM extracted PGN, Scid 4 and Scid 5 .7z fixtures byte-for-byte and rejected
corruption. A virtual archive with its next header beyond 5 GiB verified 64-bit
seeks with source reads bounded to 1 MiB. The actual worker module ran with the
real decoder and simulated OPFS handles, validating staging and all three
handoffs, including retained Scid 5 files and close cleanup. Quota failure and
cancellation clean only the generated staging directory. Final rebuilt decoder
tests passed again; port 8788 returned the new import module, worker and Apache
license with HTTP 200. Browser WASM rebuilt and Wrangler assets refreshed.

## Cached decoder startup fix

Reproduced the reported disappearing window in a real browser: the unversioned
immutable WASM URL initialized the worker with an older app binary, which failed
with `missing #chess_canvas`. The worker now obtains the package version and fetches
the matching WASM URL explicitly. Decoder loading and archive inspection report
their stages; any failure stays in the extraction dialog with a Close button.

Validation: 58 regression tests passed, including versioned decoder requests and
retained errors. After the required rebuild/staging, the six archive-specific tests
passed again. A real browser with actual FileReaderSync and OPFS reproduced the
old failure, then successfully extracted the Scid 5 fixture and exposed all three
files after the fix. Closing the dialog reached the successful handoff state and
released staging resources. Temporary diagnostic assets/tab were removed. Port
8788 serves the corrected worker and helper.
