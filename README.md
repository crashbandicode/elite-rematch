# elite-rematch

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

Skyline plugin for Super Smash Bros. Ultimate 13.0.5 that keeps the online
rematch when a player enters or leaves Elite Smash. In the unmodified game
the rematch is taken away after that match, even if both players want to play
again. With the plugin the rematch stays, and an indicator shows the Elite
Smash change.

**Status (2026-10-30):** not built. The investigation of how the game decides
the Elite Smash change and the rematch has not started; nothing here is tested.

## Plan

- Find in the 13.0.5 binary what removes the rematch, and what the server
  sends for GSP and Elite Smash.
- Keep the rematch in both directions and show the change on screen.
- Safe online: no change to the match itself, no random draws; the written
  argument goes in `docs/` before any release.
- Server features that Eden's LAN Online lacks for testing:
  `docs/EDEN-GAPS.md`.

## License

MIT
