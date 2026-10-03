---
title: "Import a game from a scoresheet"
description: "Manually enter chess moves beside a scoresheet photo, with a board preview and live validation."
category: "using-ironwood"
keywords: [scoresheet, handwritten, manual, photo, PGN, import, validation, draft, autosave, check, resignation]
weight: 34
---

Open **Game → Import → Enter scoresheet**. The workspace has four panels: a board preview and validated move list at the top, with your scoresheet photo and manual entry table below. On narrow screens the panels stack, with buttons to jump between the photo and entry table.

Choose, paste, or drop a JPG, PNG, or WebP photo. Zoom in and drag the photo to read the handwriting. Photos stay on your device. You can also enter moves without a photo.

Enter White's and Black's moves in their respective cells using standard notation: `e4`, `Nf3`, `O-O`, `exd5`, or `e8=Q`. **Tab** or **Enter** advances to the next cell. More rows are added as you reach the bottom.

Lowercase piece letters are accepted: `nc3` appears as `Nc3` in the move list. Ironwood adds check (`+`) and checkmate (`#`) to the validated notation automatically. For example, entering `nxe7` in a position where that capture gives check produces `Nxe7+` in the move list and PGN. The entry cell keeps what you typed.

Moves are checked while you type. Green borders mark legal moves, amber marks an incomplete move such as `Nf`, and red marks an illegal or missing move. Later moves remain editable but cannot be validated until earlier errors are corrected. Changing an earlier move revalidates the entire continuation without deleting your work.

The board starts in the standard starting position and follows valid entries. Click a move in the move list to review its resulting position, or click **Starting position** to return to the beginning. Focusing an entry cell shows the position before that move.

Gold highlights show the starting and ending squares of the displayed move. The move list scrolls to the latest validated move as you type, and follows the position selected by an entry cell.

Enter the players, event, date, and result above the panels. Once all entered moves are legal, **Download PGN** saves a file. **Import game** places the PGN in the existing Import Games screen; choose **Continue** there to finish importing and optionally analyze it.

Do not enter resignation or a result in a move cell. If Black resigns, leave the next Black cell empty and select **White won** in the **Result** dropdown; the PGN ends with `1-0`. If White resigns, select **Black won** (`0-1`). Select **Draw** for `1/2-1/2`, or **Unknown / unfinished** for `*`.

Moves, game details, and the photo save automatically on this device, including incomplete or illegal entries. Reopening after refreshing or closing the app restores the draft. Check the save status before leaving. **Discard draft** clears moves, details, and photo after confirmation. Downloading or importing keeps the draft. Drafts belong to this browser and device; clearing browser data removes them.
