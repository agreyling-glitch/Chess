# Automatic local imports (private preview)

Companion creates `Import` and `Reports` beside its configuration and data folders.
With an archive enabled, `automatic_import` defaults to `true`; setting it to `false`
in the local configuration disables the worker. No public site entry point is added.
All browser Companion access remains behind the existing diagnostics flag.

Drop complete `.pgn`, `.zip` or `.7z` files into `Import`. Only top-level files are queued;
folders, temporary extensions and other formats are ignored. Files must have unchanged
size and modification time across two scans (normally two seconds apart). A producer
that writes slowly should write a temporary extension and rename to `.pgn`/`.zip`/`.7z` when
finished. ZIP contents are read in memory, never extracted to paths on disk.

Idle means no engine search, startup or option validation is running, no engine
registration is active, and no foreground archive request is active. Loaded idle
engines and connected browsers may remain open. There is a two-second quiet period
after engine activity. The worker handles one source at a time, validates outside
the SQLite lock, commits batches of 128 games, and pauses between batches for engine
activity. Update validation blocks startup work; update installation and import
commits use the same exclusive reservation. Shutdown preserves unfinished input.

Each source is atomically claimed under `Import/.processing/<job-id>`. Its recovery
record contains only its original filename, kind, timestamp and SHA-256 fingerprint.
Each committed batch and its imported/duplicate/failed counts are recorded in SQLite
in the same transaction as the games. Restarting verifies the original source
fingerprint, validates the contiguous saved-batch prefix, and skips committed
games. Scid skips use direct index seeks; PGN skips scan game boundaries without
legal-move replay or document preparation. Neither games nor report statistics
are counted twice. Partial batches must end their source file. Missing or invalid
receipts stop recovery with the source retained. Existing receipt formats remain
supported.
Existing archive duplicate/version rules are unchanged. Annotated versions, browser
records, notes and saved analyses are retained.

A UTF-8 text report under `Reports` records the filename, UTC start/end, outcome,
processed/imported/already-saved/failed counts and any unattempted or unknown remainder.
Reports have unique job IDs and are flushed and atomically renamed before source
cleanup. Successful files, including files containing only duplicates, are deleted.
Files containing invalid games or archive errors move to `Import/Failed` for review.
Move a corrected file back to `Import` to retry it. Report-write failures leave the
claimed source in place for retry; shutdown or a crash resumes it on the next start.
A changed recovery source is retained, never silently skipped or deleted.

Limits: 64 MiB input; 64 MiB total uncompressed PGNs; 1,000 ZIP entries. Stored and
DEFLATE ZIPs are supported. Encryption, ZIP64 and multi-part archives are rejected.
Symlinks/reparse points are rejected for queue folders, input and report files.

The native DEFLATE implementation uses flate2 1.1.10 (MIT OR Apache-2.0), already
present in the workspace lockfile. Its exact sources and license texts are vendored
by the Companion packaging script; the dependency inventory is updated accordingly.

## 7z and native Scid imports

A single `.7z` may contain PGNs and/or complete Scid 4 or Scid 5 sets. Include
matching `.si4/.sn4/.sg4` or `.si5/.sn5/.sg5` files with the same base name in the
same archive directory. Nested directories are supported; incomplete or mixed
sets are rejected. Loose Scid files are not queued. Scid sources are read-only;
custom tags, pre-game comments and Scid-specific flags are omitted. Deleted games
are skipped and counted separately; games shorter than two full moves are discarded.

7z sources and expanded data are limited to 64 GiB, with at most 1,000 entries,
16 MiB headers, 256 MiB decoder dictionaries and 128 MiB Scid name tables (at most four million name records). LZMA,
LZMA2, COPY and BCJ filters are supported. Encrypted archives and optional codecs
must be extracted first and repacked using a supported codec. The 64 MiB limit
above continues to apply to loose PGN and ZIP imports, not 7z imports.

Unpacking streams to generated private files under the claimed job directory;
archive paths are never used as destination paths. Links, special files, unsafe
paths and duplicate names are rejected. Every entry is drained and checked for
integrity before database writes begin. Allow free space for the expanded files
plus the archive database. Hashing and unpacking pause for engine activity and
shutdown retains the source for restart. Scid indexes and game files are read from
disk in batches; PGNs stream from disk. A private extraction manifest binds the
source hash to generated filenames, sizes and SHA-256 file hashes. Recovery
reuses those files after verifying their hashes; missing, incomplete or altered
files cause fresh extraction from the verified source. The marker is published
only after full extraction/integrity checking and file synchronization. Older
jobs without a manifest need one fresh extraction before reuse is possible.
Temporary unpacked files are removed after the durable report is written.

The decoder is sevenz-rust2 0.24.1 with lzma-rust2 0.23.0 (Apache-2.0), configured
for decompression without optional codecs or encryption. The native Scid reader
reuses chess-scid-rw 1.0.0 (GPL-3.0-or-later). Vendored sources, licenses and local
resource-limit changes are included in the development package.


## App progress (private preview)

Authenticated `GET /api/v1/archive/import-progress` returns the current or last
folder job: filename, phase/message, total/processed games, committed imported,
duplicate, short, deleted and failed counts, games per second and ETA seconds.
Polling uses the archive-read budget and never locks the archive database.
Only Companion-enabled clients with a connected default archive request it.
Import Games, the Scid import dialog and the Companion archive window show it.

Scid totals come from index headers/sizes. PGNs use a streaming boundary-count
pass without legal-move validation or document construction. Counting and
unpacking show an indeterminate bar; ETA starts once importing begins. Speed is
newly committed games per second since import preparation finished, including
waits; skipped saved batches and recovery preparation are excluded. ETA is hidden while waiting, unpacking or
verifying the source. Completed speed/counts persist until the next job, including
across browser reloads; Companion restart begins a fresh rate measurement while
recovering durable job counts. No browser credentials or source paths are exposed
in the progress response.


## Import admission and background indexing

Position indexing and folder imports both use the bounded background work queue.
Background database use does not make Companion non-idle for import admission;
the queued work still serializes safely on the archive connection. Foreground
archive requests (including CPU preparation) have their own activity counter;
that counter stays active through cancellation until the actual work finishes.
Imports pause for those client requests, engine searches/setup, the two-second
engine quiet period, and update reservations. Waiting messages identify the
specific reason instead of referring to an idle Companion in general.


## Native status window

The open Windows Companion status window refreshes once per second from in-memory
status/progress, independently of software-update checks. It includes comma-formatted
import counts, speed, ETA and the specific wait reason. Timer refreshes neither
activate/reopen minimized or closed windows nor reset text selection/scroll. The
manual Refresh button remains available. Unchanged text is not repainted.

Recovery displays a separate resuming stage and starts rate/ETA measurement only
after locating the first unsaved batch. Saved Scid game blobs are not decoded,
validated, converted or resubmitted. All source/archive duplicate rules still
apply to new batches. The final source fingerprint check and durable-report
cleanup remain required.

The native status text now uses an overflow-only gold scrollbar rather than an
always-visible system scrollbar. Visibility is measured at the full text width
so removing overflow restores the available space. Gold thumb/track rendering,
dragging, track paging, mouse-wheel preferences and normal text keyboard scrolling
are local UI behavior; no import or archive data is changed. Scroll positions are
clamped after automatic text updates so short text cannot remain scrolled out of
view when the scrollbar disappears.

The main application's bottom Companion dock also polls opening-index progress
every two seconds while visible and enabled. It shows comma-formatted indexed
and queued counts, recent indexing speed and the estimated time for the current
backlog. Continuing imports can increase that backlog. Transactional derived
`explorer_progress` counters make status reads constant-size; existing archives
backfill these once during startup. Older Companions use a slower polling interval.
