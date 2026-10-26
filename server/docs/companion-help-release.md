# Companion public Help and download pages

Published with owner authorization October 26, 2026.

- https://ironwoodchess.com/downloads/companion/
- https://ironwoodchess.com/help/companion-setup/
- https://ironwoodchess.com/help/companion-engines-and-storage/
- https://ironwoodchess.com/help/companion-troubleshooting/
- https://ironwoodchess.com/features/#desktop-engines-and-archive

All four pages are non-draft and indexable. Normal site builds include them. The
Help index, relevant existing articles, Features navigation, site header/footer
and homepage link to Companion. Application Help exposes setup and connections;
the About checkbox was removed. Old preview preferences do not hide public setup.

`site/data/companion.toml` records version 0.1.4, the immutable public Windows ZIP
URL, the clean-tested checksum and `available = true`. There is no release-access
token requirement for public downloads. Owner connection tokens and update
signature verification remain mandatory.

The public Windows build is x64 only. Keep Linux/macOS/QNAP developer-only.
Update the data file together with a newly validated immutable release; do not
replace a versioned ZIP. Production builds must continue to exclude future drafts.

The loopback preview server on port 8791 is optional and is not the public site.
See `windows-public-release-0.1.4.md` for deployment and verification evidence.
