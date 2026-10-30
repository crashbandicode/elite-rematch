# Eden LAN Online: what Elite Smash testing needs (2026-10-30)

For the Eden agent. The `elite-rematch` plugin keeps the Quickplay rematch
when a player enters or leaves Elite Smash. Testing it in Eden needs a GSP
that moves and an Elite Smash state that can change. Today neither happens in
LAN Online. No change to the game, Deluxe or the plugin is asked for here.

Elite Smash is "VIP" inside the game (`is_vip`, `MODE_VIP`,
`SEQUENCE_ONLINE_MELEE_ANYONE_VIP`). The decision to enter or leave it is made
by the game from the fighter's GSP. The GSP itself reaches the game in server
replies.

## What the game calls and what Eden answers

Source: Yoga runs al-a5 / al-b5, 2026-10-18 (`/home/intpa/cov-runs/al-*/console.log`),
companion `internal/initshape/initshape.go` and `internal/nsoserver/server.go`
read on the Surface 2026-10-30.

| Call | Request | When | Eden reply now |
|---|---|---|---|
| DataStore 0x73 / 82 | 30 B | each console, match start and match end | empty body |
| DataStore 0x73 / 38 | 1025 B | each console, match start and match end | empty list |
| DataStore 0x73 / 21 | 966 B | match start (first match) | override file, 8 B |
| DataStore 0x73 / 8 | 37 B | before a search | NotFound |
| DataStore 0x73 / 60, 61, 83, 84, 95 | 0-36 B | entering Online | override files or empty |
| Utility 0x6e / 7, 9, 10, 11 | 4-26 B | entering Online | override files |
| MatchmakeExtension 0x6d / 40, 11, 1, 58 | 839, 422, 4, 4 B | each search | implemented |
| MatchMaking 0x15 / 41, 42 | 4, 5 B | session URLs; host update after a rematch decline | implemented (42 being fixed) |

The server registers no DataStore or MatchmakeReferee protocol of its own;
`initshape` answers DataStore and Utility with neutral, protocol-shaped
values. So GSP never changes and no console ever enters or leaves Elite Smash
in LAN Online.

## Needed

1. **GSP per player and fighter that changes after each match.** The match-end
   DataStore calls (82, 38) are the likely report/read pair. Their exact
   meaning and reply layout are being worked out from the 13.0.5 binary;
   this section will name them when confirmed.
2. **An Elite Smash threshold** the game compares GSP against. Where it
   comes from (a reply above, e.g. the 15,704-byte `0x73_60` override, or
   another call) is still open.
3. **A test switch**, per profile or PID: set a fighter's GSP just above or
   just below the threshold, so the next match moves it across. Acceptance:
   after one match one console has entered Elite Smash and the other has not;
   then the reverse (leaving).
4. **Mixed matchmaking.** On Nintendo's servers a player waiting in Elite
   Smash is eventually matched with non-Elite players, and their rematches
   work. Eden's matcher must let an Elite Smash search (0x6d/40 with the
   VIP flag in its criteria) meet a Quickplay search, at least after a wait.
5. **Graceful rematch decline** (already planned in the Surface
   `REMATCH_FIX_PLAN.md`): the one who declines leaves; the one who accepted
   stays without a communication error. Needed for one-console-only tests.

## Not needed

Voice chat, arenas, rankings, real accounts, or any recorded Nintendo bytes.
