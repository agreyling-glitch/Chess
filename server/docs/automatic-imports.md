# Automatic local imports (private preview)

Companion creates `Import` and `Reports` beside its configuration and data folders.
With an archive enabled, `automatic_import` defaults to `true`; setting it to `false`
in the local configuration disables the worker. No public site entry point is added.
All browser Companion access remains behind the existing diagnostics flag.

Drop complete `.pgn` or `.zip` files into `Import`. Only top-level files are queued;
folders, temporary extensions and other formats are ignored. Files must have unchanged
size and modification time across two scans (normally two seconds apart). A producer
that writes slowly should write a temporary extension and rename to `.pgn`/`.zip` when
finished. ZIP contents are read in memory, never extracted to paths on disk.

Idle means no engine search, startup or option validation is running, no engine
registration is active, and the archive has spare admission capacity. Loaded idle
engines and connected browsers may remain open. There is a two-second quiet period
after engine activity. The worker handles one source at a time, validates outside
the SQLite lock, commits batches of 128 games, and pauses between batches for engine
activity. Update validation blocks startup work; update installation and import
commits use the same exclusive reservation. Shutdown preserves unfinished input.

Each source is atomically claimed under `Import/.processing/<job-id>`. Its recovery
record contains only its original filename, kind, timestamp and SHA-256 fingerprint.
Each committed batch and its imported/duplicate/failed counts are recorded in SQLite
in the same transaction as the games. Restarting a job replays parsing but reuses
committed batch results, so neither games nor report statistics are counted twice.
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
