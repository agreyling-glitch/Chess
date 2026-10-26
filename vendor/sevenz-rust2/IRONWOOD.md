# sevenz-rust2 provenance

Upstream: https://github.com/hasenbanck/sevenz-rust2
Crates.io release: 0.24.1. License: Apache-2.0 (LICENSE).

Ironwood uses decompression only, with default features disabled. Core LZMA/LZMA2,
COPY and BCJ filters are available; encryption and optional codecs are excluded.
The lzma-rust2 0.23.0 dependency is also Apache-2.0.

Local changes in src/reader.rs bound decoder dictionaries to 256 MiB, encoded and
decoded headers to 16 MiB, and file/directory entries to 1000. These limits apply
before the associated untrusted allocations/decompression. All other source is
from the crates.io release. Exact transitive sources and licenses are included
by Companion's source packaging workflow.

Metadata stream/block counts are capped at 1000 and coder counts at 32 before
allocation. The manifest omits upstream example/integration-test targets whose
fixture assets are not distributed in Ironwood; library sources are included.
