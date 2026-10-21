# Ironwood release checks

Always rebuild the browser WASM bundle after application changes with `npm run build:wasm`; a compile check alone is not sufficient.

The production PWA update flow was checked for a previous release. Do not require a recurring PWA test reminder or confirmation before GitHub pushes. Choose validation appropriate to the changes being released.

# Licensing requirements

Only recommend or implement solutions with a clear, straightforward licensing path for Ironwood's intended use and distribution. Verify licensing before proposing or integrating engines, libraries, model weights, artwork, or other third-party assets. Do not assume a repository's code license also covers its model weights or other assets. Exclude options with unclear terms, pending permissions, or complicated licensing obligations; prefer suitable alternatives with explicit, easy-to-comply-with licenses.

# Ironwood Companion private preview

Use `scripts/package-companion.ps1` with its default output for Windows development
updates. Keep the current build at `output/companion/current` and reuse that path;
do not create a new output folder for each Companion change. Stop a running copy
before replacing it, and preserve its configured data directory.

Ironwood Companion is unreleased and must remain hidden by default behind the Companion checkbox in Help → About → Version diagnostics. Gate all add-on entry points and integration controls with that shared feature flag. This does not apply to existing pop-out windows such as the 3D board.

Until the user explicitly says Companion is ready to reveal, do not announce or document it in public features pages, changelogs, release notes, user help, or other public-facing content. Internal implementation plans, server developer documentation, and validation records are allowed. Do not add a Companion changelog entry for private-preview work.
