---
title: "Import games from handwritten scoresheets"
date: 2026-10-03T00:00:00-05:00
description: "Manual scoresheet entry with live validation, board preview, reference photos, and saved drafts."
slug: "2026-10-03-scoresheet-import"
version: "2026.10.03"
---

## Added

- **Enter scoresheet** in **Game → Import**, with four panels for the board preview, validated move list, reference photo, and White/Black move entry.
- Local photo selection, paste, and drop, with zoom and drag controls for reading handwriting.
- Live validation that distinguishes incomplete, illegal, and missing moves. Correcting an earlier entry rechecks later moves without deleting them.
- Automatic local saving of moves, game details, and the photo. Drafts restore after refreshing or reopening the app; **Discard draft** clears them after confirmation.
- PGN download and handoff to the existing game import flow, with player names, event, date, and result.
- A searchable [scoresheet import guide](/help/import-a-scoresheet/), including saved drafts, check notation, and recording resignation through the Result dropdown.

## Improved

- Scoresheet entry accepts lowercase piece notation and generates standard notation, including check and checkmate, in the move list and PGN.
- The move list follows the latest validated move and the position selected by an entry cell. Gold board highlights show the displayed move's starting and ending squares.
- Scoresheet panels stack on narrow screens, with shortcuts between the photo and entry table.
