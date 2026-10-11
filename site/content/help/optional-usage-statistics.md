---
title: "Optional usage statistics"
description: "Choose whether to share usage statistics and change your choice later."
category: "settings"
keywords: [analytics, privacy, consent, statistics, PostHog]
weight: 70
---

Ironwood asks whether you want to allow optional usage statistics. Choose **Allow statistics** to participate or **No thanks** to decline. Playing, saving games, and engine analysis work with either choice.

Expand **Privacy details** in the choice window to read more without leaving the app. Reading those details does not select an answer.

Local development on `localhost` or `127.0.0.1` uses the same consent choice without a special URL. Consented local events are labeled `local_test`, separate from production dashboard data.

## Change your choice

Use **Analytics settings** at the bottom-right of the app to allow or decline statistics later. Your choice is remembered in that browser. Declining stops future analytics collection and clears analytics persistence; it does not delete events already submitted.

## What is measured

With your permission, PostHog receives app visits, local games started, full-game analyses started, and active usage time. Cookies and browser storage recognize returning browsers and sessions. Visitor counts therefore represent participating browsers, rather than verified people.

Game positions, move lists, notes, imported games, player names, page titles, and full URLs are excluded. Session recording and automatic interaction capture are disabled. PostHog receives network information such as your IP address and browser information.

The chess engine and local analysis run on your device. Online features contact their respective services independently of analytics. See the [privacy notice](/privacy.html) for more details.
