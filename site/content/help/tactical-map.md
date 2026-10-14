---
title: "Tactical Map"
description: "Explore patterns, opponent threats, move changes, positional activity, knight routes, and pin-aware safety."
category: "analysis"
lastmod: 2026-10-14T00:00:00-05:00
keywords: [attack vs defense, defenders, legal recaptures, pins, knight reach, destination safety, positional activity, opponent threats, what changed, tactical map]
weight: 18
---

Tactical Map helps you inspect the displayed position without running engine analysis. Pattern and threat lines, Knight reach, and attacker/defender bubbles work in both board views.

## Open and arrange the map

Click the **bullseye icon** immediately to the left of **2D/3D** above the board. Notes and Draw on board have separate icons to its left. Drag or resize the floating window to arrange your workspace. Click the bullseye again or its close button to hide the map and overlays.

The map has three tabs: **Insights** for Attack vs Defense, Opponent threats, and What changed; **Activity** for positional activity and Knight reach; and **Patterns** for Danger, Pins, and Forks. Each group sits in a sunken section. Resize the window vertically to give the threat and change lists more room. The lists share available height; a shorter list leaves spare room for the longer one, and excess entries scroll.

## Attack vs Defense

Enable **Attack vs Defense** to show bubbles beside every real piece. On 3D boards, the badges and numbers lie flat on the square's surface and follow rotation, tilt, and zoom. Hover a badge or piece for details.

- **Red, top-left:** geometric enemy attackers, including pinned attackers.
- **Green, top-right:** **geometric defenders / legal recapturers**. The first number includes pinned defenders. The second counts distinct friendly pieces able to recapture after at least one candidate legal enemy capture.

A small legend appears directly below the checkbox:

| Bubble | Meaning |
| --- | --- |
| `2/-` | Two geometric defenders. No legal enemy capture on the square, so recaptures do not apply. |
| `2/0` | Two geometric defenders, but neither can legally recapture after the candidate captures. |
| `2/1` | Two geometric defenders; one can legally recapture after at least one candidate capture. |

Hover a piece for **capture-specific** details. A defender may work against one attacker but not another. A capture can also uncover a new recapturer that was not originally a geometric defender, so the second number can exceed the first.

Legal recapture checks account for pins, opened lines, and king safety. A pinned piece may still recapture along its pin line. A king cannot recapture onto an attacked square. These checks do not calculate whether an exchange wins material or account for intervening moves.

Geometric control respects blockers and pawn attack directions. A piece does not defend itself, and personal ghost annotations do not affect the counts. Zero counts are shown. Bubbles update with the position and remain in the same screen corners when the board is flipped. Disable the option or close the map to hide them.

## Opponent threats

Enable **Opponent threats** to list candidate **checks**, **captures**, and **forks** for the side opposite the current side to move. The list assumes you leave the position unchanged for a hypothetical opponent turn; it does not predict your next move. All promotion choices are considered.

Click a row to trace the candidate without playing it:

- **Pink:** the moving piece's source and destination, with a move arrow.
- **Gold:** checking or fork attack lines after the candidate move, including discovered checks.

Click the same row again to clear it. Changing position clears the displayed selection. If the current side is in check, resolve check before exploring a hypothetical opponent turn. Captures may lose material, and a fork candidate is not a forced win.

## What changed?

**What changed?** compares the displayed game position with the immediately preceding position. Step through the move list to update its explanations and overlays:

- **Blue:** an opened attack line from a stationary bishop, rook, or queen to a stationary enemy piece.
- **Gold dashed lines:** lost support, drawn from the defender's **previous square** to the surviving piece it no longer defends. The previous square may now be empty or occupied by a capturer.
- **Red:** a newly threatened piece or an additional attacker against an already attacked piece.

The list includes lost defenders on pieces that are not currently attacked. It tracks moved surviving pieces where their identities can be established. At the starting position or without a matching previous game position, step forward in a game to make a comparison. An unchanged position produces no changes.

These comparisons use geometric control, including pinned pieces. They explain changed relationships, not engine-proven threats. Disable **What changed?** to hide its list and markings.

## Positional activity

With Tactical Map open, **click any real piece on the main board**, of either color. The separate **Positional activity** section displays a score and reasons. In review, clicking inspects without moving the piece. During a live game against the AI, normal move selection still works: activity follows the piece you click, and clicking a legal destination plays the move. Knight reach does not need to be enabled.

| Score | Rating |
| --- | --- |
| 0–34 | Restricted |
| 35–54 | Balanced |
| 55–74 | Active |
| 75–100 | Dominant |

Reasons can include geometric mobility, central control, pressure on enemy pieces, friendly support, pawn attacks, absolute pins, a rook's open or semi-open file, and a supported advanced knight post. King ratings also note whether enemy rooks or queens remain.

The score is an explainable heuristic, **not a material value, engine evaluation, or tactical safety verdict**. Mobility is measured before king-safety checks, and geometric support includes pinned pieces. An active piece can still be lost tactically. Select another piece to inspect it; changing position clears the old activity card.

## Fork candidate strength

Fork pattern cards distinguish **strong fork candidates** (orange: checking a king while attacking a queen or rook, or attacking multiple major pieces, with no immediate legal capture of the forking piece), **checking fork candidates** (gold), and **capturable fork candidates** (muted cyan). Cards name the targets and show legal captures of the forking piece when available. Matching colors appear in the selected pattern's board outlines and arrows. These labels describe immediate legal replies, not an engine-proven material win.

## Knight reach: two hops

Enable **Knight reach · two hops**, then click a knight on the main board or choose its square beside **Knight**. Either color can be selected. In 3D, ghosts use the selected 3D knight model and square outlines follow the board's rotation, tilt, and zoom. Hover a ghost to focus its route; in review, click it to lock the route, just as in 2D.

- **Ghost knights:** first-hop destinations. For the side to move, these are legal moves. For the other side, they show geometric reach, including pinned moves.
- **Outlined squares:** potential second-hop destinations, assuming other pieces stay where they are and accounting for captures along the route.
- **Gold outlines:** potential second-hop forks against at least two enemy kings, queens, or rooks.
- **Coral rings:** first-hop destinations occupied by an enemy piece.

Hover a ghost to focus its branch and connecting arrows. **In review, click a ghost to lock its route** so it stays visible when the pointer moves away. Click the same ghost again or use **Unlock route** to release it. Click another ghost to switch branches. Changing position or selecting a different knight clears the old route. During live play, board clicks select and move pieces; use hover previews, or enter review to lock a route.

Second hops are potential reach, not a legal two-move sequence: the opponent's reply, checks, captures, and pins can change available moves. Preview ghosts are temporary and do not change the game or create saved drawings.

### Destination safety

**Destination safety** is enabled by default within Knight reach. Hover a first-hop ghost or a second-hop destination to inspect the hypothetical position after that route. With a locked branch, hover its second-hop squares; when no destination is hovered, safety remains visible for the locked first hop.

- **Red lines:** geometric enemy attackers.
- **Gray lines:** geometric friendly support.
- **Green lines:** pieces able to make a legal recapture after at least one candidate enemy capture.
- **Amber lines:** geometric defenders unable to recapture after any of those captures because their king would remain or become unsafe.

The destination label shows **G / R**: geometric defenders and legal recapturers. A dash in place of R means there is no legal enemy capture to recapture. Hover for the specific recaptures available against each attacker.

Safety accounts for the knight leaving its original square, captured pieces disappearing, and opened attack lines. It checks both the enemy capture and each friendly recapture for king safety. It does not evaluate exchange profitability or predict intervening replies.

## Filter and inspect patterns

The **Patterns** section contains **Danger**, **Pins**, and **Forks** buttons with counts, directly above the finding cards. Click a button to show or hide that kind of pattern. Select a card to outline its relevant squares and trace attack lines; select it again to clear the selection.

- **Danger:** surviving pieces on unchanged squares that are attacked and gained attackers or lost defenders after the last move. Coral outlines mark them.
- **Pins:** pieces shielding their king from an enemy bishop, rook, or queen. Purple outlines mark absolute pins.
- **Forks:** legal moves for the actual side to move that make the moving piece attack at least two enemy pieces other than pawns. Select a candidate to see its destination and targets in the card's matching color. All promotion choices are considered.

These cards are geometric clues, not engine-confirmed material wins. They are separate from **Opponent threats**, which explores the other side's hypothetical moves, and **What changed?**, which explains changes after the last move. The map does not provide a skewer category or a complete vulnerability heatmap.

## Availability and saved annotations

The map is hidden during rated training games, active online play, best-move exercises, and prediction previews. Its controls and selections are temporary for the current session. They do not alter your game, saved notes, or personal drawings, and are not exported as annotations.

Use [Draw on board](/help/drawing-on-the-board/) for persistent arrows and shapes, or [Notes](/help/taking-notes/) to record conclusions. Use engine analysis to check whether a candidate survives the opponent's replies.
