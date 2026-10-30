# Eden LAN Online: what Elite Smash testing needs (2026-10-30)

**Status 19:56Z:** the Eden agent's rating runtime (v8) provides items 1, 2
and 4 below and real Elite Smash changes were tested with it (see
`SAFETY.md`). Seeds need an existing stored report for the fighter. Open:
with both consoles searching at the same moment, a Quickplay search joined
an Elite Smash session at once (2 s, run e08); on Nintendo's servers an
Elite player meets non-Elite players only after waiting. Measured in Eden
by the diagnostic build: Elite threshold 1642, Elite search widening after
120 s.

For the Eden agent. The `elite-rematch` plugin keeps the Quickplay rematch
when a player enters or leaves Elite Smash. Testing it for real needs a GSP
that persists and can cross the Elite Smash threshold. No change to the
game, Deluxe or the plugin is asked for here.

## How the game does it (13.0.5 binary, Eden runs)

- The game computes GSP itself after each match and keeps it in a local
  cache; Eden shows it moving (Lucario 9,068,494 -> 7,254,021 after a loss,
  Shulk 9,068,494 -> 10,882,966 after a win; run e01, 2026-10-30).
- Elite Smash ("VIP" in the game) is decided by the game: the fighter's
  cached rating against a threshold from the game's own data (parameter
  1201; `.text+0x17049d0`). No server reply carries the threshold.
- The server only stores and returns the rating, per player and fighter:

| Call | When | Request | Reply the game expects | Eden today |
|---|---|---|---|---|
| DataStore 0x73 / 84 | entering Online | none | Map<u8 fighter slot, rating record> + an aggregate | operator override file (29 B), same value for every fighter |
| DataStore 0x73 / 82 | match start and end | version, fighter slot (u8), three u32 values, time (u64), flag (u32) = 30 B | success, no payload | empty success, nothing stored |
| DataStore 0x73 / 21 | first match | metadata object with a 100 x 9-byte event history | u64 object id | override file (8 B) |
| DataStore 0x73 / 38 | match start and end | update of that object (id, mask 0x58, history) | success, no payload | empty list |

Sources: `elite-rematch-work/re/FINDINGS.md` (Q5), Yoga runs al-a5/al-b5
(2026-10-18), companion `internal/initshape/initshape.go` (Surface,
2026-10-30). Calls 82/84 go through the game's `TaskRating`; 21/38 through
its evaluation-history writer.

## Needed

1. **Store what 82 writes, return it from 84**, per player and fighter, so
   GSP survives a restart. An empty 84 map is not "zero GSP": the game then
   fills slots from local defaults.
2. **A test switch**: per profile or PID, an 84 reply that puts one fighter's
   rating just above, or just below, the Elite Smash threshold, so the next
   match moves it across. Acceptance: after one match a console shows the
   game's "Elite Smash" notice (`pop_online_vip_start`) instead of
   "Rematch?" (`pop_result_battle_again`); then the reverse (leaving).
3. **Mixed matchmaking.** On Nintendo's servers a player waiting in Elite
   Smash is eventually matched with non-Elite players (the game relaxes its
   own search after a delay, parameter 1199) and their rematches work. Eden's
   matcher must let such searches meet.
4. **Graceful rematch decline** (already planned in the Surface
   `REMATCH_FIX_PLAN.md`): the one who declines leaves; the one who accepted
   stays without a communication error.

## Not needed

The threshold, voice chat, arenas, rankings, real accounts, or any recorded
Nintendo bytes.
