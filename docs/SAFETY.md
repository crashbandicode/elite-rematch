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

## Real Elite Smash changes (Eden, 2026-10-30, 19:08-19:56Z)

Same-PC pair on one host, the Eden agent's rating server (v8) with seeded
Lucario ratings, no forced popup. The game's own values, logged by the
diagnostic build: Elite Smash at rating >= 1642, Elite searches widen to
non-Elite players after 120 s. The modded console ran this plugin; the
vanilla console ran no elite-rematch plugin (desync detector only). The
loser ran off the stage once at the start; each run was a match, Yes on
both, the full rematch, No on both.

| Run | Vanilla | Modded | Popup on the modded console | Rematch | Detector |
|---|---|---|---|---|---|
| 1 | non-Elite (1500), wins | Elite (1645), loses: bumped out | "The Elite Smash battle has been canceled. Rematch?" | played to the end | no mismatch |
| 2 | Elite (1800), wins | Elite (1645), loses: bumped out | same | played to the end | no mismatch |
| 3 | non-Elite (1500), loses | non-Elite (1641), wins: enters | "You can now participate in Elite Smash. Rematch?" | played to the end | no mismatch |
| 4 | Elite (1800), loses | non-Elite (1641), wins: enters | same | played to the end | no mismatch |

In runs 4 and 1 (repeated) the Elite console searched alone first and was
paired with the non-Elite one when its search widened (about 124 s), as on
Nintendo's servers. The game chose the notice again after the rematch: the
rematch stays in the session of the first match, so the game compares
against that tier. Since 2026-10-30 ~20:10Z the plugin shows the notice once
per lobby and the plain "Rematch?" after later rematches; a new search
(`.168a5b8`, which runs for a new search but not for a rematch) or any
answer but Yes starts over. Release build (sha256 a85435fc...) in Eden,
20:17-20:34Z: entering, three popups "notice + Rematch?", "Rematch?",
"Rematch?"; leaving, "notice + Rematch?" then "Rematch?"; all rematches
played to the end, no desync.

## Other plugins on the same sites (0.2.0, 2026-10-30)

Each site is read before it is hooked (`model::entry`):

- the 13.0.5 instructions: hooked as usual;
- another plugin's Skyline far jump: the plugin chains onto it. The message
  lookup calls that plugin's hook as its original; an inline hook goes on a
  small stub that then jumps to that plugin's handler. Both hooks run, and
  the jump's address word is never relocated as code;
- another plugin's one-word branch: Skyline relocates it, and this hook must
  come out as a one-word branch too, or the site is put back as it was;
- other plugins' hooks further on in the 64 checked bytes are accepted;
  anything else leaves that one site alone.

Only the popup hook is required. Without the message lookup the notice shows
alone, still with No and Yes; without both the choice and the search hook it
shows on every popup.

Eden, offline, with the 14 other plugins on the test Switch's card
(arcropolis, css_preserve, elite_stages_counterpick, gsp_visualizer,
imgui_smash, nro_hook, param_hook, smash_gamepad_overlay,
smash_replay_to_mp4, ssbu_desync_detector, ssbu_online_deluxe_rewrite,
ssbu_refresh_clock, ssbu_shaderdb_probe, training_modpack):

- 22:16Z: every site stock and hooked (Skyline wrote a one-word branch for
  the inline hooks, a far jump for the lookup); boot and main menu normal.
- 22:19Z, diagnostic build hooking each site first as another plugin would:
  `branch/near` on the three inline sites, `chain/far` on the lookup; the
  menu text, which reaches the game only through the other hook, was normal.

## Not verified yet

- The Switch, and Nintendo's servers.
