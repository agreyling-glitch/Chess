# Companion public Help and download pages

## 0.1.8 public release — October 30, 2026

Windows x64 Companion 0.1.8 includes optional sign-in startup, configurable server-side backup folders, authenticated SQLite snapshot backup/restore, and indexed collection filtering for the unified Saved Games window. Metadata editing, Browser/Companion copies and moves, and collection management are available from Saved Games. The former Your game archive window is retired.

Publish a new immutable 0.1.8 ZIP with the existing release key and public Windows update feed. Retain every previously published versioned ZIP. The download metadata must match the exact validated package and signed feed. Public setup remains discoverable, and owner tokens, exact-origin checks and update-signature verification remain mandatory. Windows x64 is the only public download; other platforms remain developer-supported.

The release workflow packages matching source and license notices, validates the exact ZIP in isolated and clean Windows environments, checks upgrade/recovery, publishes the ZIP before the signed manifest, then verifies anonymous delivery and protected private channels. See `windows-public-release-0.1.8.md` for the completed validation record. Historical 0.1.4 evidence remains in `windows-public-release-0.1.4.md`.

Pages updated with the release:

- `/downloads/companion/`
- `/help/companion-setup/`
- `/help/companion-engines-and-storage/`
- `/help/companion-troubleshooting/`
- `/help/saved-games-and-backups/`
- `/features/`

`site/data/companion.toml` supplies the exact versioned package URL and SHA-256. Do not substitute a development ZIP or change a published ZIP to refresh its documentation.
