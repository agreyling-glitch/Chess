---
title: "Set up Companion for Windows"
description: "Install Companion, connect your browser, and understand where your games are saved."
category: "settings"
keywords: [Companion, Windows, install, download, setup, token, connect, address, multiple companions]
weight: 61
draft: false
---

## Download and start

Use the [official Companion download page](/downloads/companion/) for the Windows package, requirements, and publisher-signature information. Extract the complete ZIP into a permanent folder. Open `current` and run **ironwood-companion.exe**. Look for **Ready** in the status window.

The tray icon keeps Companion available after closing its status window. Use **Quit** from the tray menu to stop it completely. The browser cannot use Companion while it is stopped.

## Connect the browser

1. In Ironwood, choose **Help → Companion setup…** for download and installation instructions.
2. Open **Help → Companion connections…**.
3. Set **Companion name** to something recognizable, such as **Win11**.
4. For Companion on the same computer, use `http://127.0.0.1:8790` as **Companion address**.
5. In the Companion status window choose **Show token…**, then copy that value into **Connection token**. This is the connection credential, not a release-download credential.
6. Choose whether this connection is the **Default archive** and an **engine provider**, then choose **Connect / Save**.

Select **Remember this connection on this device** only if you want the browser to retain the address and token. Otherwise it lasts for this tab. Treat the token as a password; it authorizes access to your Companion.

## Choose where games are saved

The location list in **Storage information**, **Saved games**, and **Import** contains **Browser** and your named Companion connections. Select the destination before importing or saving there. Browser and Companion archives are separate libraries; connecting does not automatically migrate or synchronize them.

Each Companion has its own archive and engine installations. Use names such as **Win11** and **QNAP** to tell them apart. One connection can be the default archive; multiple connections can provide engines. Engine choices include the Companion name.

## Use another computer

`127.0.0.1` always means the computer running the browser. It will not reach a Companion on another machine. The normal Windows setup listens locally; renaming a connection does not enable network access. A remote installation needs its own reachable address, server configuration, and permitted browser origin. Do not expose the connection token or Companion port to the public internet.

The address is where Companion runs. An allowed origin is the website permitted to call it, such as `https://ironwoodchess.com`; adding an origin does not change the listening address. Remote/NAS setup is outside the Windows quick-start guide.

## Next steps

Read [Engines and storage with Companion](/help/companion-engines-and-storage/) to add an engine or import a collection. For connection failures, see [Companion troubleshooting](/help/companion-troubleshooting/).
