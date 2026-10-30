# Why the plugin is safe online (13.0.5)

Static study of the 13.0.5 image, 2026-10-30. Eden results are added below
as they are measured.

## What it changes

One thing: the text and buttons of the popup shown after an online match,
before it is displayed.

- The result component (`.text+0x147f960`) asks whether the local fighter
  just entered (`+0x180`, `.16ba170`) or left (`+0x188`, `.16ba360`) Elite
  Smash and picks one of three popup records. When it picked an Elite Smash
  record, the plugin parses the ordinary (rematch) record into the same
  descriptor with the game's own parser (`.32edf70`), the call the game
  makes for every popup, then puts the Elite Smash text back. The descriptor
  is the game's reusable popup descriptor, reparsed before every popup.
- The choice is handled by the game: button 2 is the rematch (code 7), any
  other button leaves (code 1), the same mapping for all three records
  (`.147f4d0..147f508`).

- The text: while such a popup is up (from that parse until a button is
  chosen), the game's message lookup (`.37a4450`) is answered, for the Elite
  Smash notice's label only, with that notice, a line break and the game's
  own "Rematch?" (`pop_result_battle_again`), both looked up through the
  same function so they are in the console's language. Every other lookup
  returns the game's own text unchanged.

## What it does not change

- No random draw, no match simulation: the popup runs after the match's
  lockstep has ended, and nothing in the match is read or written
  (`ssbu-desync-detector` `WHAT-IS-CHECKED.md`, "UI and HUD that write no
  battle state").
- Not the rating (GSP), its upload (DataStore 0x73/82) or the cache, not
  the Elite Smash classification (`.17049d0`), not the old tier
  (snapshot `+0x57`), not the network messages, timers or masks.
- Not the mode switch: when the player stops rematching (any choice but the
  rematch), the game re-classifies the fighter and moves between Quickplay
  and Elite Smash as usual (`.2316538..65c4`). A rematch (code 7) skips that
  step in the unmodified game too, which is why the next match keeps the
  current mode.

## Both consoles

With the plugin, a console whose fighter changed tier takes the same path
as a console whose fighter did not: the ordinary rematch popup and, on Yes,
the game's own rematch preparation (`.169edd0`, peer messages). So:

- Opponent without the plugin and no tier change: it sees the ordinary
  popup; two Yes answers give the normal rematch.
- Opponent without the plugin whose own tier changed: it sees the game's
  Elite Smash popup without a rematch and leaves; this console's Yes ends
  as any declined rematch does.
- Mixed tiers in one session already happen without the plugin: a player
  waiting in Elite Smash is matched with non-Elite players after a while
  (search widening, `.16963d0` / parameter 1199) and can rematch them.

## Eden results (2026-10-30)

Two consoles (Surface and Yoga) over Eden LAN Online, Quickplay 1v1, the
Eden-only diagnostic build on both. Eden can not produce a real tier change
(see `EDEN-GAPS.md`), so one console was forced to show an Elite Smash
record.

| Record (logged by the plugin) | Style | Text | Buttons |
|---|---|---|---|
| ordinary | 2 | `pop_result_battle_again` ("Rematch?") | No, Yes (default) |
| entered Elite Smash | 0 | `pop_online_vip_start` ("You can now participate in Elite Smash.") | none |
| left Elite Smash | 0 | `pop_online_vip_end` ("The Elite Smash battle has been canceled.") | none |

- Plugin off, Elite Smash record forced: the notice closed by itself after
  about 1.5 s (button 0) and that console left; the other console's
  "Rematch?" ended with No. This is the behaviour the plugin changes.
- Plugin on, the same forced record: the console showed "You can now
  participate in Elite Smash." with No and Yes and the usual countdown; both
  consoles chose Yes, met in the rematch waiting room (both "Ready!") and
  played the rematch to its end; after it, both chose No and left (twice,
  14:30Z and 14:54Z).
- Plugin on, "left" record forced: "The Elite Smash battle has been
  canceled." with No and Yes. This console chose Yes and the other No: the
  Yes side waited in the rematch room and then got Eden's communication
  error, Eden's known decline gap (`EDEN-GAPS.md` item 4), not the plugin.
- `ssbu-desync-detector` on both consoles: no check mismatch in any of these
  sessions, the rematch matches included.
- Mixed install, combined text (15:22-15:33Z): the console with the plugin
  showed "You can now participate in Elite Smash. Rematch?" with No and Yes;
  the other console ran no elite-rematch plugin at all. Two rematches in a
  row (three matches), the detector on both: every session ended with
  `desync_seen=0` and the same frame counts on both consoles.
- Release build on both consoles (15:36-15:43Z): installed with all hooks;
  a match, a rematch (Yes on both), then No on both; the plugin left both
  ordinary popups as they were, and the detector saw no mismatch.

## Not verified yet

- A real tier change (needs GSP crossing the threshold) and the mode switch
  after the last rematch: Eden tests force only the popup record.
- The Switch.
