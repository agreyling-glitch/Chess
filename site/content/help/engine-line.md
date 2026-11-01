---
title: "Best move and engine line"
description: "Choose a prediction engine, compare candidate arrows and scores, and distinguish saved alternatives from live analysis."
category: "analysis"
keywords: [best move, engine line, PV, principal variation, variation, multipv]
weight: 50
lastmod: 2026-11-01T00:00:00-05:00
---

## Best move

The **best move** is the selected analysis engine's current top choice in a position. It is the move the engine prefers at the search depth and settings used. A longer search can change the choice, especially in complicated positions.

## Choose the prediction engine

In **Prediction**, click **Analyze position** to start a search, or **Stop analysis** to stop it. The small arrow beside the button opens **Analysis engine**. Choose **Stockfish 19 (browser)** or an available engine from a connected Companion. The current engine is named beneath the button. The menu also offers installed-engine refresh and Companion connections.

Switching engines clears the old live score, PV lines, and prediction markers. If analysis was running, it resumes once the new engine is ready. Saved full-game runs remain separate from live predictions. The selected analysis engine can differ from the engine that played the game.

## Show candidate moves and scores

Enable **Show best next move** in the bordered On/Off card near the top of Prediction. Its green state indicates that board arrows and scores are enabled. Ironwood compares every available PV for the displayed position, independently of the line selected in the table, and ranks the moves for the side to play.

- A **green circle and arrow** mark the strongest candidate, with a green score bubble at the destination square's bottom-right corner.
- **Red circles, arrows, and score bubbles** mark other candidates. Red means an alternative to the current top choice, not necessarily a mistake.
- Scores use **White's perspective**, matching the panel's evaluation; mate results use an **M** label. Duplicate candidate moves share one marker. Different moves to the same square have stacked bubbles.

While stepping through a PV, only lines containing the displayed position can provide its next move. Lines from other positions do not supply prediction markers. The control remembers your choice, and prediction hints are unavailable during active rated training.

## Engine line or PV

An **engine line** is a sequence of moves the selected engine expects if both sides continue with strong play. Engine programs call this the **principal variation**, or **PV**. Read it as “one likely strong continuation,” not as a prediction that a human opponent will play those exact moves.

For example, `1. e4 e5 2. Nf3` means White moves a pawn to e4, Black responds by moving a pawn to e5, and White moves a knight to f3. The letters name pieces and board squares. Clicking through the line on the board is often easier than reading the notation alone.

## Compare the played move with the best line

After full-game analysis, select a move and look at **Move Analysis** in the
**Analysis → Critical Analysis**. The header shows its move-quality badge and CPL lost when a
numerical loss is available. A bar marks **Equal** at zero and shows the change
between **Before** and **After**. **Swing** is the absolute difference in pawns;
all scores use White's perspective. Mate scores remain labeled as mate results
instead of being converted into a numerical swing.

The played-line heading names the player who made the selected move, alongside
**ENGINE PREFERRED**. Its border, heading, and first-move highlight match the move's
quality: gold for Best, green for Good, yellow for Inaccuracy, orange for Mistake,
and red for Blunder. The engine-preferred line uses green. The engine line stays readable during
playback. In a narrow panel, the two lines stack vertically and complete move
chips wrap onto the next row. Its **best-line estimate** is the stored engine search score for that
continuation, not a new search after its first move. A short description identifies
confirmed move details such as a capture or check; it does not infer an unverified
tactical plan.

Compact **first / previous / Play / next / last** controls sit inside the
**ENGINE PREFERRED** card, below its line and evaluation. They replay the saved engine line without changing the played game or
saved analysis. Right-click **Play** to choose the speed. **Return to game
moves** appears beneath these controls only while a saved engine-line preview is active.
Click it to stop playback and restore the game position you were reviewing.
Scrolling over the board or Game Moves also leaves this preview and steps through
the played game. Scroll down for the next move, or up for the previous move.
The Game Moves scrollbar remains available to browse the list.

Use the review filter to step through **Mistakes & blunders**, **Blunders only**,
or **All critical moves** (including inaccuracies). The Previous and Next buttons
skip moves outside that selection and stop at its ends. **Try the best move**
opens the existing best-move exercise when the engine recommends an alternative.

## Keep the saved recommendation visible

**Compare saved line** is enabled by default. Its checkbox is beneath the saved recommendation in **Move Analysis**, under **Analysis → Critical Analysis**. You can hide it there; the setting is remembered across positions and reloads.

The saved recommendation may belong to the position **before** the move you are reviewing. Ironwood draws it as a **dashed blue arrow**, with a translucent blue ghost of the original piece when that piece has left its source square. The board label identifies the earlier position. The real pieces and recorded game remain unchanged, and live green/red predictions can remain visible alongside it.

For example, if the saved analysis preferred **Kf7** from g7 but Black played **Kf6**, the board after Kf6 can show a blue ghost king on g7 and a dashed arrow to f7. This is the saved alternative for Black's previous turn, not a legal move suggestion for White's current turn. On the 3D board, the ghost uses the selected piece model and follows camera rotation, zoom, and board orientation.

The comparison is hidden while previewing a saved line or stepping through a prediction, and during active rated training.

## Multiple lines

**MultiPV** asks the selected engine to show several leading candidate moves instead of only its top choice. This helps compare plans. The second line is not necessarily bad; it is simply ranked below the first by that search. Showing more lines may reduce how deeply each one can be examined with the same time or computing budget.

## Expand the prediction into boards

Click the gray raised **Expand** button beneath the **Engine prediction** table in the **Prediction** tab, under **Engine Prediction**. A separate window shows one mini board after each predicted move, with its move label and highlighted source and destination squares. Each board has a sunken frame and coordinates on all four sides, following your board orientation. Click a mini board to inspect that position on the main board when active play or training does not prevent navigation.

The title includes the variation number, evaluation, first move, and search depth. Each **Expand** click captures a separate prediction: change the selected variation or analyze another position, then expand again to compare lines side by side. Existing windows keep their captured moves rather than following later engine updates.

Resize a window to scale its boards and change the grid layout, or use the title's **Maximize** and **Restore** controls. Small windows keep pieces readable and scroll additional rows. Each window has a gold border and one shared sunken panel with a visible paper-grain background. Closing one leaves the others open.

## Navigate the docked prediction

In **Prediction**, select a candidate in the variation selector above the table; it includes the evaluation, first move, and depth. Click a move or use the navigation buttons beneath the table to inspect the continuation. Scrolling over the board or prediction table steps through the selected line. Open **Moves** and select a move in **Game Moves** to return to the played game.

The plain column headings identify White and Black, with a narrow move-number column. Changing a variation updates the docked prediction, while already expanded windows keep their snapshots.
