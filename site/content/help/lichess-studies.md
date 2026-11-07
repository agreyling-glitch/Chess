---
lastmod: 2026-11-05T00:00:00-06:00
title: "Lichess studies"
description: "Read chapters, explore variations, practice moves, and keep your own notes."
category: "using-ironwood"
keywords: [Lichess, study, chapters, variations, repertoire, practice, PGN]
weight: 85
---

Open **Training → Study**. Paste a public Lichess study URL into the import box and choose **Import study**. The study opens in Study mode at its starting chapter. Public studies must allow downloads.

When no study is open, choose one of the selectable **Saved studies** cards. **Storage location** switches between Browser and a connected Companion; **Connections…** manages connections. **Choose another study…** returns to the picker, while **Return to current study** restores the loaded workspace. Opening a saved study from Saved Games also uses Study mode.

The general **Import games → Lichess** workflow, also available from **Saved Games → Manage library → Import games…**, accepts study or chapter links. A successful fetch opens the study directly, without a Paste tab or **Continue** step. For private study content, export PGN from Lichess and import it in Ironwood.

## Read and explore

Use the chapter selector between **Previous** and **Next** to move through the study. The reading panel shows the author's introduction and comments at the selected position. Choose a continuation to follow the main line or a variation. **Moves and comments** shows numbered moves, variations and linked commentary together. The chapter menu stays closed until you need it, so the lesson remains in view. Video links in comments open directly. Left and right arrow keys move back and forward; Home returns to the chapter's start when a text field does not have focus.

The header shows the selected chapter's **ECO code and opening name** beneath the author. Ironwood uses the chapter's tags, with offline opening recognition as a fallback; unidentified openings leave the line hidden. The whole lesson panel has a sunken border and subtle grain. Gold move labels stand apart from softer comments, and hover highlights keep their size so move rows stay in place.

You can also play moves on the board. A studied move follows that branch. In **Read & explore**, a different legal move creates a local variation. **Back** returns to the preceding position. **Main line** returns to the end of the author's main line.

## Your notes and drawings

Write in **Your note** to keep a personal observation at the current position. The author's text stays separate, including after a PGN export and reimport. The **Annotations** tab at the top of the Study panel provides **Drawings**, **Notes**, and **Chess Symbols** for study positions; your notes, glyphs, and drawings follow the position when you switch chapters or branches.

Ironwood saves the workspace with your saved games, including the selected chapter, variations, and local notes. Under **Source and export**, **Copy study PGN** copies every chapter and its supported annotations. Exporting the workspace as PGN from Saved Games also includes the complete study. Moving within a line keeps its analysis active. Switching branches keeps separate analysis snapshots so returning restores that line’s results. A manual board flip stays in effect while you navigate. Back up your browser library to preserve the complete editable workspace and its local state.

In the Browser saved library, choose **Studies** to find your studies. Each Lichess study has one entry containing its chapters. Older individual chapter saves are combined using their Lichess study ID; notes, analysis, favorites, and collection membership are kept. Original chapter records are also retained inside the consolidated record in browser backups. Studies with the same title but different source IDs remain separate.

## Practice

Choose **Practice**, then **Play White** or **Play Black**. The initial side follows the study’s orientation. Changing chapters automatically plays the other side’s opening move when needed. Play the next move on the board or enter its notation with the keyboard. Any continuation in the study is accepted. Ironwood plays the other side's main reply. An incorrect move leaves the position unchanged so you can try again. **Reveal move** gives a continuation when you need help. Correct moves show the available explanation, and the panel tracks correct moves and attempts for the session. Choose **Chapter start** to restart the line, or **Read & explore** to see its branches again.

## Refresh from Lichess

Under **Source and export**, **Refresh study** downloads the current public study. Your notes, glyphs, and drawings are matched by chapter source and move path. If the author removes a branch with local work, Ironwood keeps the earlier chapter as a **local copy**, so you can recover those observations. Removed chapters are also retained locally. A failed request leaves your copy intact. Refresh is a download; it does not upload your changes to Lichess.

Private and unlisted study refresh through sign-in is not available. You can continue reading, practicing, and annotating an imported study offline once Ironwood is cached. Fetching or refreshing requires an internet connection. Only import study content you own or have permission to use.
