---
title: "Import a chess puzzle from another site"
description: "Bring a puzzle position into Ironwood with a FEN or a board screenshot."
category: "using-ironwood"
keywords: [puzzle, import, other site, website, FEN, screenshot, image, position editor, solve]
weight: 15
---

You can bring a puzzle's **starting position** into Ironwood in two ways: copy its [FEN](/help/fen/) if the source offers one, or import a screenshot of its board. Ironwood does not import a puzzle by pasting a website URL. It also does not receive the source site's solution or scoring rules; you are setting up the position to explore it yourself.

## If the site provides a FEN

1. On the source site, look for **Share**, **Export**, **Copy FEN**, or a similar control. Copy the complete FEN line. A complete FEN has six space-separated parts, including whose turn it is.
2. In Ironwood, open **Game → New game** and choose **Play Both Sides**. Choose **Starting position → Custom FEN**.
3. Paste the FEN into the custom-position box. Ironwood should show **Valid position**. If it says **Needs attention**, check that you copied the full line and not just the piece-placement part.
4. Start the game. Play the puzzle move, then make the opponent's replies yourself to explore the line. For an engine opponent instead, choose **You vs Engine** before starting.

If you prefer to inspect or adjust the position first, click **Edit board / Import screenshot** in the New Game dialog, paste the FEN into the editor's FEN box, and click **Load FEN**. After checking it, click **Use in Ironwood** to return to the New Game dialog.

## If you have a screenshot

1. Save or copy an image that clearly shows the entire board. Crop closely around the 64 squares if possible; avoid clocks, move lists, borders, and other page details.
2. In Ironwood, open **Game → New game → Play Both Sides → Starting position: Custom FEN → Edit board / Import screenshot**.
3. Choose the image file, paste the image into the editor, or drop it onto the editor. Recognition runs locally on your device. Its model may need a first-use download.
4. Compare **every square** with the source image. Correct wrong or missing pieces using the palette. Orange outlines flag less-certain squares. Use **Flip view** if you only want to see the other side at the bottom; use **Rotate position** only if the imported pieces themselves are turned around.
5. Set **Side to move** from the puzzle prompt. Check castling rights and en passant availability if the puzzle depends on them. A screenshot cannot reveal these rules. If the source offers a FEN, use that instead for reliable position details. For an ordinary puzzle without those special rights, `-` is appropriate in those fields.
6. Click **Use in Ironwood**. Confirm **Valid position** in the New Game dialog, then start and try your move.

For more detail on placing pieces and the position fields, see [Using the Position Editor](/help/position-editor/). Ironwood checks whether the setup is a legal chess position, but it does not know which move the original puzzle author intended. You can use [analysis](/help/getting-started/) to compare ideas after trying the puzzle.
