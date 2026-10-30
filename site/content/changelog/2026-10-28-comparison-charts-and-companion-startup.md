---
title: "Accuracy comparison charts and Companion startup settings"
date: 2026-10-28T00:00:00-05:00
slug: "2026-10-28-comparison-charts-and-companion-startup"
version: "2026.10.28"
description: "Compare accuracy and CPL for both players and each game phase, plus optional Windows sign-in startup in Companion 0.1.5 development builds."
---

## Compare both players in Analysis Summary

- Replaced the separate accuracy and CPL summary cards with two comparison charts:
  **Accuracy (higher is better)** and **Average centipawn loss (lower is better)**.
- Added paired bars for **Overall**, **Opening**, **Middlegame**, and **Endgame**.
  White uses Ironwood gold and Black uses sage green, with names in the legend
  and values above the bars.
- Kept phase move counts attributed to each player. Hover over a bar for its
  details; click a phase bar to jump to that player's first move in the phase.
- Highlighted the current phase and distinguished missing values from zero.
  Phases the game never entered show **Not reached**.
- Kept accuracy on a 0–100 scale and made the CPL scale fit the game's values.
  Both players share a scale within each chart. The existing metric calculations
  and saved-analysis selection remain in use.

See [Accuracy and CPL](/help/accuracy-and-cpl/#compare-the-players) and the
[game-phase guide](/help/game-phases-and-critical-moves/).

## Compare move-quality distributions

- Replaced the move-quality badge panels with a stacked horizontal bar for each
  player, using Ironwood's existing Best, Good, Inaccuracy, Mistake, and Blunder
  colors.
- Added classified-move totals, proportional segments, segment counts, and a
  complete category breakdown below each bar, including zero counts.
- Kept click-to-jump navigation on segments and nonzero category counts. Hover
  over a segment to see its count and percentage.
- Moved forced-mate totals into separate cards, retaining navigation and showing
  found or escaped mate events when present.

Read the [move-quality chart guide](/help/move-quality/#compare-move-quality-by-player).

## A clearer Move Analysis panel

- Added a zero-marked evaluation-swing bar and Before, After, and Swing values.
  Scores stay White-relative; mate results do not get a fabricated numeric swing.
- Moved the solid move-quality badge and CPL lost into the header.
- Placed the played line and engine-preferred line side by side with highlighted
  first moves and clearer continuation text. The played-line heading names the
  player; its border, heading, and first move use the selected move's quality
  color. The engine-preferred line uses green.
- Added the stored best-line evaluation, explicitly labeled as an estimate,
  and replaced the generic swing sentence with confirmed move details such as
  captures and checks.
- Placed compact saved-line playback controls inside the **ENGINE PREFERRED** card,
  below the continuation and evaluation. Added review
  filters for mistakes and blunders, blunders only, or all critical moves.
- Fixed excess space above the swing bar when Move quality is collapsed. Narrow
  panels stack the played and preferred lines, and move chips wrap as complete
  moves instead of breaking into vertical text.
- Added **Return to game moves** inside Engine Preferred to stop line playback
  and restore the selected played-game position.

Read the [Move Analysis guide](/help/engine-line/#compare-the-played-move-with-the-best-line).

## A separate window for the study panel

- Added **↗** beside the right-hand tabs to open the entire study panel in a
  separate browser window, including all seven tabs.
- Kept the popout connected to the main game's state: moves, analysis, scrolling,
  annotations, and playback remain synchronized without starting another engine.
- Replaced image streaming with the panel's own layout, input context, font atlas,
  and GPU renderer. Text and charts render at the popout monitor's native pixel
  density, resizing reflows the controls, and inputs update the same game directly.
- The panel leaves the main window while popped out, giving that space to the
  board. Closing the popout or choosing **Dock study panel** restores the panel.
- Added **Show main window** and **Close popout** controls. Reopening focuses the
  existing popout and preserves the same game state.
- Disabled popout input while the main panel is hidden in focus or puzzle mode.

Read the [study-panel window guide](/help/getting-started/#open-the-right-hand-panel-in-a-separate-window).

## Companion 0.1.5 development update

- Bumped the local Companion build to **0.1.5**.
- Added **Settings → Start Companion when I sign in** to its Windows status
  window. Checking it creates a Startup shortcut for your account; clearing it
  removes the shortcut. No administrator permission is required.
- Preserved the current data directory and configuration path in the shortcut,
  including paths with spaces and non-English characters.
- Added error feedback that restores the checkbox to the actual shortcut state
  if a change fails. Updates at the same application location keep the shortcut
  working; after moving the folder, clear and enable the option again.

**Companion 0.1.5 is a development build.** Its public Windows package and signed
update feed have not been published. The [public download](/downloads/companion/)
remains **0.1.4**, with its matching package URL and checksum.

Read the [Windows setup guide](/help/companion-setup/#start-automatically-when-you-sign-in)
or [startup troubleshooting](/help/companion-troubleshooting/#companion-does-not-start-when-i-sign-in).
