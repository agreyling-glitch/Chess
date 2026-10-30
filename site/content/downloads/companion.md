---
title: "Download Ironwood Companion"
description: "Use desktop chess engines and a local game archive alongside Ironwood in your browser."
layout: "companion-download"
draft: false
---

## Install in three steps

1. **Download and extract.** Save the Windows ZIP and choose **Extract All**. Put the extracted folder somewhere permanent, such as Documents. Keep the `current` folder and its supporting files together.
2. **Start Companion.** Open `current`, then run **ironwood-companion.exe**. The status window should say **Ready**. No Rust, Node, Python, or Visual Studio installation is required for Companion itself.
3. **Connect Ironwood.** In Companion, choose **Show token…**. In Ironwood, open **Settings → Companion connection…**. Give it a name, enter `http://127.0.0.1:8790` and the connection token, then choose **Connect / Save**.

Keep Companion running while using its engines or archive. [Follow the complete setup guide](/help/companion-setup/).

**Included in Companion 0.1.8:** choose **Start Companion when I sign in**
in the status window’s Settings row to enable or disable Windows sign-in startup.
[Read how sign-in startup works](/help/companion-setup/#start-automatically-when-you-sign-in).

## What you can use it for

- **Desktop engines:** install a supported engine directly from its publisher, or register an engine you already have.
- **A game archive:** save games on the Companion computer and choose that location in Saved games and Import.
- **Folder imports:** place PGN, ZIP, or 7z files in the Import folder and follow progress in Ironwood.

Ironwood also works without Companion. Your browser's existing games remain in Browser storage until you explicitly copy or move them.

## Before you start

This Windows release has been tested on **Windows 11, Intel/AMD 64-bit**. Native Windows ARM, macOS, Linux, and NAS packages are not offered here. Engine and model requirements vary; check the engine's requirements before installing it.

The executable does **not** have a Windows publisher signature. Windows may display an unknown-publisher or reputation warning. Download only from this official page and verify the package if unsure; do not turn off Windows security. If Windows blocks it or you cannot establish the source, stop and ask for help. Ironwood's signed update verification is separate from Windows publisher trust.

## Games, settings, and updates

Games, engines, and settings live separately from the extracted application folder, normally in `%LOCALAPPDATA%\Ironwood\Companion`. The **Archive**, **Engines**, **Import**, and **Reports** links in the status window open the relevant folders.

**Updating from 0.1.4 or 0.1.7:** download and extract the 0.1.8 ZIP, quit Companion from its tray, and replace the application files in the same folder. Preserve your data directory. Those older versions have a restart-path bug in automatic updating; if an attempt fails, rollback retains the previous version. Do not keep retrying that automatic update. After this one-time manual update, 0.1.8 uses the corrected updater.

Use **Check for updates…** in Companion. When a verified compatible update is available, **Update and restart** becomes available. Updates are checked against Ironwood's release key and package checksum. A failed or unavailable update check does not mean your installation is current.

For a manual update, quit Companion from its tray menu before replacing the application files. Preserve the data folder and back up your archive. [Read the storage and engine guide](/help/companion-engines-and-storage/).

## Build from source

Browse the [Ironwood source on GitHub](https://github.com/agreyling-glitch/Chess). Companion is the `ironwood-server` Rust package. Windows x64 is the published download. **Linux, macOS, and QNAP/NAS builds remain developer-supported** and need platform-specific validation. The native tray and status window are Windows features; use the terminal server on other platforms.

### 1. Install build tools

Install [Rust and Cargo using rustup](https://rust-lang.org/tools/install/) (**Rust 1.95 or newer**), Git, and your platform's compiler tools:

- **Windows:** Visual Studio Build Tools with **Desktop development with C++** and a Windows SDK. Use the MSVC Rust toolchain.
- **macOS:** install the Xcode Command Line Tools with `xcode-select --install`.
- **Linux:** install a C/C++ toolchain, `pkg-config`, and OpenSSL development headers. On Debian or Ubuntu: `sudo apt install build-essential pkg-config libssl-dev ca-certificates git`.

SQLite is compiled from the bundled dependency. Building Companion does not require building the browser app or installing Node, Python, or a separate SQLite server.

### 2. Get the source and build

For development, clone the complete repository and build from its root:

```sh
git clone https://github.com/agreyling-glitch/Chess.git
cd Chess
cargo build --locked --release -p ironwood-server --bin ironwood-server
```

Keep `Cargo.lock`, `crates`, and `vendor` with the checkout; the `server` folder alone is not enough. Cargo builds for the current computer and architecture. For another platform, build on that platform or configure its Rust target, compiler, linker, and system libraries.

For **source matching the published package**, extract the Windows ZIP above and open its `source` directory instead of cloning GitHub. It includes dependency sources and Cargo configuration. Run the same Cargo build command there. Use this source if the GitHub checkout does not yet contain Companion. The ZIP's Windows executables do not run on other operating systems.

### 3. Start and connect

On Linux or macOS, from the source root:

```sh
./target/release/ironwood-server --data-dir ./companion-data
```

On Windows, in PowerShell:

```powershell
.\target\release\ironwood-server.exe --data-dir .\companion-data
```

The example initializes a private `companion-data` directory containing configuration, the archive, and an `owner-token` file. For ongoing use, choose a permanent, private data directory outside the checkout. Keep the server running; stop it with Ctrl+C.

Read `owner-token` privately. In Ironwood, open **Settings → Companion connection…** and use that value as the connection token. Use `http://127.0.0.1:8790` when the browser is on the same computer. Connection tokens and allowed website origins remain required. Another computer needs an explicitly configured remote setup; changing the connection name does not make the server reachable.

For the Windows desktop development package, run `scripts/package-companion.ps1` from the repository root in PowerShell. It builds into `output/companion/current`. Stop a running copy before replacing it and preserve its configured data directory.

### Containers, engines, and updates

The repository includes `deploy/ironwood-server/compose.yaml` for Linux Docker builds and a QNAP guide at `server/docs/qnap-container-station-installation.md`. These are developer workflows, not additional public platform downloads. Containers and NAS installations need persistent storage and explicit network configuration.

Use engines built for your operating system and architecture; Windows engine executables do not run as native Linux or macOS engines. Update source builds by rebuilding and replacing the stopped program, preserving its data directory. The public Windows update feed is not a cross-platform installer.

Companion is licensed under **GPL-3.0-or-later**. Retain license and dependency notices. If distributing your own binaries, provide their matching source as required by the license. See `LICENSE` and `server/docs/licensing.md` in the source.

## Need a hand?

Start with [connection and startup troubleshooting](/help/companion-troubleshooting/). Never include your connection token, owner-token file, or private configuration in a screenshot or support report.


**Companion 0.1.8** includes a configurable server-side backup folder, SQLite backup/restore for large archives, and collection filtering in Saved Games. See [backup and restore help](/help/saved-games-and-backups/).
