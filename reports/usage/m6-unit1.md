# Usage: milestone m6

<!-- usage:begin -->
## Generated usage tables

generated: 2026-10-30T15:35:20Z

window: 2026-10-30T12:15:25Z to 2026-10-30T15:34:49Z (`--to` is the generation time minus 1 s unless given; log turns compare at second resolution, as `scripts/usage-from-log.sh` does)

Directories: logs `/Users/dct/.factory/logs`, sessions `/Users/dct/.factory/sessions`, mission `/Users/dct/.factory/missions/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c`, prices `config/prices/2026-10-29.toml`.

### Sources

| Source | Lines | First timestamp | Last timestamp | Note |
|---|---|---|---|---|
| `logs/droid-log-single.log.2026-10-25` | 104069 | 2026-10-24T22:00:00.777Z | 2026-10-25T14:02:32.250Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-25.1` | 102938 | 2026-10-25T14:02:32.250Z | 2026-10-25T18:13:28.811Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-25.2` | 12077 | 2026-10-25T18:13:36.176Z | 2026-10-25T22:58:53.631Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-26` | 16574 | 2026-10-25T22:59:53.610Z | 2026-10-26T22:59:32.335Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-27` | 24948 | 2026-10-26T23:00:06.062Z | 2026-10-27T22:59:32.435Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-28` | 24018 | 2026-10-27T23:00:07.787Z | 2026-10-28T22:59:32.526Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-29` | 99983 | 2026-10-28T23:00:09.636Z | 2026-10-29T15:54:10.828Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-29.1` | 102664 | 2026-10-29T15:54:10.951Z | 2026-10-29T21:17:14.450Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-29.2` | 33840 | 2026-10-29T21:17:15.210Z | 2026-10-29T22:59:48.367Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-30` | 100830 | 2026-10-29T23:00:03.186Z | 2026-10-30T03:36:53.771Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log.2026-10-30.1` | 101127 | 2026-10-30T03:36:53.772Z | 2026-10-30T11:45:08.078Z | lines up to the window end; unparsed 0 |
| `logs/droid-log-single.log` | 69957 | 2026-10-30T11:45:09.330Z | 2026-10-30T15:34:49.788Z | lines up to the window end; unparsed 0 |
| `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl` | 885 | 2026-10-22T15:40:52.333Z | 2026-10-30T15:33:01.433Z | records up to the window end |
| `sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c.jsonl` | 2161 | 2026-10-22T10:18:19.202Z | 2026-10-30T15:34:49.628Z | orchestrator transcript, assistant records up to the window end |
| `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json` | 142 | - | - | feature id to skill (the Skill column of Feature size) |
| `sessions/*/*.settings.json` (sidecars) | 384 | - | - | sidecars of the sessions with a log turn up to the window end |
| `2026-10-29.toml` | 8 | - | - | models priced |

Turns not joined to a session (no `tags.sessionId`): 0.

cross-check: identical (31 sessions, first/last ts, lines, input, output, cache read, cache write and the price of every model against scripts/usage-from-log.sh --embedded)

### Tokens per role

| Role | Sessions | Turns | Input | Output | Cache read | Cache write | Reasoning | USD-eq | Unpriced turns | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| orchestrator | 1 | 138 | 458 | 92870 | 20655600 | 1200717 | 29650 | 24.82 | 0 | 2026-10-30T12:15:30.336Z | 2026-10-30T15:34:49.628Z | droid-log-single.log |
| orchestrator subagent | 13 | 211 | 450 | 220070 | 11097843 | 855080 | 44601 | 5.45 | 0 | 2026-10-30T12:15:25.788Z | 2026-10-30T15:34:49.193Z | droid-log-single.log |
| worker | 3 | 117 | 240 | 69458 | 10635616 | 730864 | 18807 | 3.59 | 0 | 2026-10-30T13:10:11.862Z | 2026-10-30T15:23:09.616Z | droid-log-single.log |
| validator | 14 | 428 | 888 | 396326 | 34879513 | 1691554 | 104271 | 14.48 | 0 | 2026-10-30T14:13:53.194Z | 2026-10-30T15:33:01.200Z | droid-log-single.log |
| self-check | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.00 | 0 | - | - | - |
| other | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.00 | 0 | - | - | - |
| total | 31 | 894 | 2036 | 778724 | 77268572 | 4478215 | 197329 | 48.34 | 0 | 2026-10-30T12:15:25.788Z | 2026-10-30T15:34:49.628Z | droid-log-single.log |

USD-eq is priced per model from the price table (input, output, cache read, cache write per million tokens); a model that ran in the window with no price row, or a zero input or output price, stops the run (exit 2, naming the model), so `Unpriced turns` is 0 here; it only counts turns before the window (Mission to date by role). A price row with `plan_*_per_m` keys is the plan price and wins over the list price (see Models that ran).

### Cost by token class

| Scope | Class | Tokens | Share of tokens % | USD-eq | Share of cost % |
|---|---|---|---|---|---|
| orchestrator | input | 458 | 0.0 | 0.00 | 0.0 |
| orchestrator | cache write | 1200717 | 5.5 | 15.01 | 60.5 |
| orchestrator | cache read | 20655600 | 94.1 | 5.16 | 20.8 |
| orchestrator | output | 92870 | 0.4 | 4.64 | 18.7 |
| orchestrator subagent | input | 450 | 0.0 | 0.00 | 0.0 |
| orchestrator subagent | cache write | 855080 | 7.0 | 2.14 | 39.2 |
| orchestrator subagent | cache read | 11097843 | 91.2 | 1.11 | 20.4 |
| orchestrator subagent | output | 220070 | 1.8 | 2.20 | 40.4 |
| worker | input | 240 | 0.0 | 0.00 | 0.0 |
| worker | cache write | 730864 | 6.4 | 1.83 | 51.0 |
| worker | cache read | 10635616 | 93.0 | 1.06 | 29.7 |
| worker | output | 69458 | 0.6 | 0.69 | 19.4 |
| validator | input | 888 | 0.0 | 0.00 | 0.0 |
| validator | cache write | 1691554 | 4.6 | 5.76 | 39.8 |
| validator | cache read | 34879513 | 94.3 | 3.96 | 27.3 |
| validator | output | 396326 | 1.1 | 4.76 | 32.9 |
| milestone | input | 2036 | 0.0 | 0.01 | 0.0 |
| milestone | cache write | 4478215 | 5.4 | 24.74 | 51.2 |
| milestone | cache read | 77268572 | 93.6 | 11.30 | 23.4 |
| milestone | output | 778724 | 0.9 | 12.30 | 25.4 |

Cost by token class: each role's and the milestone's tokens and USD-eq split by class, priced per model as in Tokens per role; share of tokens and share of cost are the class's part of that scope's total (cost follows the class, not the token count). Reasoning tokens are not a class: they are billed inside output.

### Models that ran

| Model | Price basis | Turns | Input | Cache write | Cache read | Output | Input per M | Cache write per M | Cache read per M | Output per M | USD-eq | Roles | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-sonnet-5-5 | list | 704 | 1464 | 2664115 | 51903564 | 606123 | 2.0 | 2.5 | 0.1 | 10.0 | 17.91 | orchestrator subagent, worker, validator | - |
| claude-fable-5.1 | list | 138 | 458 | 1200717 | 20655600 | 92870 | 10.0 | 12.5 | 0.25 | 50.0 | 24.82 | orchestrator | - |
| claude-opus-5-5 | list | 52 | 114 | 613383 | 4709408 | 79731 | 4.0 | 5.0 | 0.2 | 20.0 | 5.60 | validator | - |

Price provenance: the observer's price table of 25-oct-2026 (resume/roci/tools/milestones.py), secondary public sources, used for every milestone reading S to M3; the owner has seen those readings. Not estimates: list prices at the date named.

Every model with a log turn in the window is listed, declared or routed. Price basis is `plan` when the model's row in `2026-10-29.toml` carries the four `plan_*_per_m` keys (the plan's price) and `list` otherwise (the list price); per-million rates are USD.

### Tokens per session

| Session | Role | Kind | Feature | Models | Turns | Input | Output | Cache read | Cache write | Reasoning | USD-eq | Unpriced turns | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c` | orchestrator | root | - | claude-fable-5.1 | 138 | 458 | 92870 | 20655600 | 1200717 | 29650 | 24.82 | 0 | 2026-10-30T12:15:30.336Z | 2026-10-30T15:34:49.628Z | droid-log-single.log |
| `b9b33259-e554-4504-a128-c93773aa0471` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 6 | 12 | 5135 | 248054 | 11510 | 305 | 0.10 | 0 | 2026-10-30T12:15:25.788Z | 2026-10-30T12:17:01.123Z | droid-log-single.log |
| `60f8be35-75ef-4878-bf2f-392f46b45c74` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 24 | 54 | 34203 | 2182229 | 220402 | 10022 | 1.11 | 0 | 2026-10-30T12:52:19.423Z | 2026-10-30T15:00:30.764Z | droid-log-single.log |
| `530ca869-25a4-458f-8302-c580c652c121` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 23 | 48 | 16161 | 856901 | 53030 | 1259 | 0.38 | 0 | 2026-10-30T12:54:43.022Z | 2026-10-30T12:57:03.299Z | droid-log-single.log |
| `d039a35e-6fe2-4b15-9e68-24197cc37c00` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 31 | 64 | 28878 | 1906284 | 89123 | 8851 | 0.70 | 0 | 2026-10-30T12:55:36.009Z | 2026-10-30T12:59:58.125Z | droid-log-single.log |
| `a759d47b-6872-405f-9695-1b4921f0c390` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 23 | 48 | 32721 | 1568315 | 89573 | 4186 | 0.71 | 0 | 2026-10-30T12:58:57.149Z | 2026-10-30T13:03:02.775Z | droid-log-single.log |
| `d7076bb5-9bb9-4b3b-8e99-d164d18948e9` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 31 | 64 | 35865 | 1626603 | 67183 | 11428 | 0.69 | 0 | 2026-10-30T13:04:06.074Z | 2026-10-30T13:09:45.690Z | droid-log-single.log |
| `300cba2b-0451-42aa-b053-953b4c5614a2` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 10 | 22 | 9289 | 346861 | 49535 | 925 | 0.25 | 0 | 2026-10-30T13:57:34.714Z | 2026-10-30T13:59:48.765Z | droid-log-single.log |
| `9872379b-381f-4e5e-9693-6c1266232c5f` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 14 | 30 | 11704 | 503437 | 43715 | 744 | 0.28 | 0 | 2026-10-30T14:10:47.231Z | 2026-10-30T14:13:21.031Z | droid-log-single.log |
| `5076cba7-7e42-462a-94b5-2990a3ec0691` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 15 | 32 | 11906 | 601683 | 45101 | 1806 | 0.29 | 0 | 2026-10-30T14:30:03.103Z | 2026-10-30T14:32:47.300Z | droid-log-single.log |
| `4f059e0e-6dc6-43e5-891d-3531349732b3` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 14 | 30 | 17386 | 639118 | 74510 | 3368 | 0.42 | 0 | 2026-10-30T14:47:37.596Z | 2026-10-30T14:50:56.914Z | droid-log-single.log |
| `dbb05bca-a42f-4dfd-b653-b49db89bd513` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 11 | 24 | 10031 | 370676 | 52554 | 754 | 0.27 | 0 | 2026-10-30T14:59:33.776Z | 2026-10-30T15:00:55.816Z | droid-log-single.log |
| `9e211f9f-c99e-461e-8cae-0a5578b5b2ae` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 7 | 16 | 5990 | 215393 | 43623 | 905 | 0.19 | 0 | 2026-10-30T15:27:48.904Z | 2026-10-30T15:29:06.683Z | droid-log-single.log |
| `4305d8fd-0dbf-40a6-aac6-bcf3efd718c2` | orchestrator subagent | subagent of e194b9e3 | - | claude-sonnet-5-5 | 2 | 6 | 801 | 32289 | 15221 | 48 | 0.05 | 0 | 2026-10-30T15:34:45.073Z | 2026-10-30T15:34:49.193Z | droid-log-single.log |
| `1bc8d998-cdd2-426c-8eb4-a507a5c1d97b` | worker | root | m6-carryover-fix-pr-gate | claude-sonnet-5-5 | 64 | 130 | 37238 | 7219973 | 415625 | 12112 | 2.13 | 0 | 2026-10-30T13:10:11.862Z | 2026-10-30T13:56:53.352Z | droid-log-single.log |
| `20b76c14-904c-45ad-9460-0b04bf5c1fea` | worker | root | final-open-pr | claude-sonnet-5-5 | 23 | 48 | 15248 | 1417672 | 147842 | 2419 | 0.66 | 0 | 2026-10-30T13:57:42.616Z | 2026-10-30T14:08:46.696Z | droid-log-single.log |
| `f5c0baea-3290-48db-8abe-1300c7144b9a` | worker | root | m6-carryover-docs-addenda | claude-sonnet-5-5 | 30 | 62 | 16972 | 1997971 | 167397 | 4276 | 0.79 | 0 | 2026-10-30T15:01:20.826Z | 2026-10-30T15:23:09.616Z | droid-log-single.log |
| `aa409f08-940d-492a-9be7-c999ec737966` | validator | root | scrutiny-validator-final | claude-opus-5-5 | 17 | 38 | 25340 | 1466489 | 218102 | 6552 | 1.89 | 0 | 2026-10-30T14:13:53.194Z | 2026-10-30T14:28:53.660Z | droid-log-single.log |
| `216f37f9-c483-437e-9427-16920945c713` | validator | subagent of aa409f08 | scrutiny-validator-final | claude-sonnet-5-5 | 7 | 16 | 9900 | 229802 | 32910 | 1673 | 0.20 | 0 | 2026-10-30T14:16:01.485Z | 2026-10-30T14:17:27.169Z | droid-log-single.log |
| `d97c2af9-4f4c-42c2-9662-4961f2ed2879` | validator | subagent of aa409f08 | scrutiny-validator-final | claude-sonnet-5-5 | 37 | 76 | 31593 | 3358400 | 121223 | 9406 | 0.95 | 0 | 2026-10-30T14:16:01.747Z | 2026-10-30T14:21:33.898Z | droid-log-single.log |
| `c7a47f01-55e7-4514-b082-64e11dfab2aa` | validator | subagent of aa409f08 | scrutiny-validator-final | claude-sonnet-5-5 | 64 | 130 | 54050 | 7686664 | 165508 | 18799 | 1.72 | 0 | 2026-10-30T14:16:02.236Z | 2026-10-30T14:26:16.848Z | droid-log-single.log |
| `916eeef6-d3e3-4f85-908f-a37e0273ff98` | validator | root | user-testing-validator-final | claude-opus-5-5 | 18 | 38 | 29309 | 1544514 | 247768 | 6147 | 2.13 | 0 | 2026-10-30T14:33:19.716Z | 2026-10-30T14:46:44.168Z | droid-log-single.log |
| `a0f419cc-0f1f-42f6-890c-70deabd683b8` | validator | subagent of 916eeef6 | user-testing-validator-final | claude-sonnet-5-5 | 47 | 96 | 33231 | 3800679 | 138289 | 9321 | 1.06 | 0 | 2026-10-30T14:36:51.233Z | 2026-10-30T14:43:13.542Z | droid-log-single.log |
| `f902e4d5-68e3-4508-94a6-c1f6d683c6d7` | validator | subagent of 916eeef6 | user-testing-validator-final | claude-sonnet-5-5 | 45 | 92 | 31639 | 3402067 | 122395 | 7899 | 0.96 | 0 | 2026-10-30T14:36:51.285Z | 2026-10-30T14:42:14.092Z | droid-log-single.log |
| `798f1689-2b13-4492-954a-ddbcf6abf66c` | validator | subagent of 916eeef6 | user-testing-validator-final | claude-sonnet-5-5 | 23 | 48 | 19749 | 1266605 | 67791 | 2765 | 0.49 | 0 | 2026-10-30T14:36:51.478Z | 2026-10-30T14:40:26.255Z | droid-log-single.log |
| `f387b7e3-8458-4f8c-8c02-8cbe90a3b019` | validator | subagent of 916eeef6 | user-testing-validator-final | claude-sonnet-5-5 | 32 | 66 | 25154 | 1580069 | 54228 | 6243 | 0.55 | 0 | 2026-10-30T14:36:51.589Z | 2026-10-30T14:41:18.826Z | droid-log-single.log |
| `463599b2-b930-4384-b705-01479509b331` | validator | subagent of 916eeef6 | user-testing-validator-final | claude-sonnet-5-5 | 41 | 84 | 36487 | 3274602 | 106861 | 9757 | 0.96 | 0 | 2026-10-30T14:36:51.862Z | 2026-10-30T14:44:50.309Z | droid-log-single.log |
| `f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38` | validator | root | user-testing-validator-final | claude-opus-5-5 | 17 | 38 | 25082 | 1698405 | 147513 | 7135 | 1.58 | 0 | 2026-10-30T15:23:18.545Z | 2026-10-30T15:33:01.200Z | droid-log-single.log |
| `89ebee66-ae29-42be-8051-fa2221b5e942` | validator | subagent of f6ab0dc7 | user-testing-validator-final | claude-sonnet-5-5 | 31 | 64 | 25861 | 2112033 | 83032 | 5778 | 0.68 | 0 | 2026-10-30T15:25:15.192Z | 2026-10-30T15:29:42.812Z | droid-log-single.log |
| `fe2e8eb4-b55f-494d-8255-7198d100091c` | validator | subagent of f6ab0dc7 | user-testing-validator-final | claude-sonnet-5-5 | 16 | 34 | 16719 | 787299 | 64096 | 3233 | 0.41 | 0 | 2026-10-30T15:25:15.361Z | 2026-10-30T15:27:54.917Z | droid-log-single.log |
| `a235722f-8585-4a37-846b-dccb400921ef` | validator | subagent of f6ab0dc7 | user-testing-validator-final | claude-sonnet-5-5 | 33 | 68 | 32212 | 2671885 | 121838 | 9563 | 0.89 | 0 | 2026-10-30T15:25:15.400Z | 2026-10-30T15:30:00.074Z | droid-log-single.log |

### Orchestrator split

| Class | Model | Turns | Input | Output | Cache read | Cache write | USD-eq | Unpriced turns | First ts | Last ts |
|---|---|---|---|---|---|---|---|---|---|---|
| decisions and boundary reports | claude-fable-5.1 | 105 | 336 | 71674 | 16296295 | 526368 | 14.24 | 0 | 2026-10-30T12:15:30.336Z | 2026-10-30T15:34:49.628Z |
| narration | claude-fable-5.1 | 33 | 122 | 21196 | 4359305 | 674349 | 10.58 | 0 | 2026-10-30T12:57:16.725Z | 2026-10-30T15:33:18.297Z |

- re-issued waits: 0 (WaitForScript calls, any `action` but `kill`, on the same `toolCallId` as an earlier wait; the first wait on each id and every kill are not counted)
- worker pauses (kills): 0 (WaitForScript calls whose `action` is `kill`: a worker or script stopped on purpose, not a wait)
- waits returning in under 60 s: 0
- WaitForScript calls in the window: 0
- orchestrator turns matched to the transcript: 138 of 138 (tolerance 3 s)
- sources: log turns of `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c` in the window; transcript `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c.jsonl`

### Subagents

| Caller role | Sessions | Turns | Input | Output | Cache read | Cache write | USD-eq | Unpriced turns | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|
| orchestrator | 13 | 211 | 450 | 220070 | 11097843 | 855080 | 5.45 | 0 | 2026-10-30T12:15:25.788Z | 2026-10-30T15:34:49.193Z | droid-log-single.log |
| validator | 11 | 376 | 774 | 316595 | 30170105 | 1078171 | 8.88 | 0 | 2026-10-30T14:16:01.485Z | 2026-10-30T15:30:00.074Z | droid-log-single.log |

### Subagent classes

| Class | Sessions | Turns | USD-eq | Declared model | Purpose | No declared model | Off declared model | Source |
|---|---|---|---|---|---|---|---|---|
| orchestrator subagents | 13 | 211 | 5.45 | - | - | 13 | 0 | - |
| validator subagents | 11 | 376 | 8.88 | - | - | 11 | 0 | - |
| worker subagents | 0 | 0 | 0.00 | - | - | 0 | 0 | - |

- no declared model: 24 of 24 subagent sessions

A subagent's class is the role of the session that started it. Declared model and purpose are read from `handoffs/*.json` and `validation/m6/**/synthesis*.json` in the mission directory (a `subagents` list or `subagentSessions` object, or a line `subagent <session id or 8-hex prefix>: model = <id>; purpose = <text>` in any string); a session with none counts under `No declared model`, and `Off declared model` counts the sessions with a turn in the window on a model other than the declared one.

### Self-check missions

| Mission | Sessions | Turns | Input | Output | Cache read | Cache write | USD-eq | Unpriced turns | Models | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| none | - | - | - | - | - | - | - | - | - | - | - | - |

### Model fallback

| Session | Role | Model | Turns | Input | Output | Cache read | Cache write | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|
| `b9b33259-e554-4504-a128-c93773aa0471` | orchestrator subagent | claude-sonnet-5-5 | 6 | 12 | 5135 | 248054 | 11510 | 2026-10-30T12:15:25.788Z | 2026-10-30T12:17:01.123Z | droid-log-single.log |
| `60f8be35-75ef-4878-bf2f-392f46b45c74` | orchestrator subagent | claude-sonnet-5-5 | 24 | 54 | 34203 | 2182229 | 220402 | 2026-10-30T12:52:19.423Z | 2026-10-30T15:00:30.764Z | droid-log-single.log |
| `530ca869-25a4-458f-8302-c580c652c121` | orchestrator subagent | claude-sonnet-5-5 | 23 | 48 | 16161 | 856901 | 53030 | 2026-10-30T12:54:43.022Z | 2026-10-30T12:57:03.299Z | droid-log-single.log |
| `d039a35e-6fe2-4b15-9e68-24197cc37c00` | orchestrator subagent | claude-sonnet-5-5 | 31 | 64 | 28878 | 1906284 | 89123 | 2026-10-30T12:55:36.009Z | 2026-10-30T12:59:58.125Z | droid-log-single.log |
| `a759d47b-6872-405f-9695-1b4921f0c390` | orchestrator subagent | claude-sonnet-5-5 | 23 | 48 | 32721 | 1568315 | 89573 | 2026-10-30T12:58:57.149Z | 2026-10-30T13:03:02.775Z | droid-log-single.log |
| `d7076bb5-9bb9-4b3b-8e99-d164d18948e9` | orchestrator subagent | claude-sonnet-5-5 | 31 | 64 | 35865 | 1626603 | 67183 | 2026-10-30T13:04:06.074Z | 2026-10-30T13:09:45.690Z | droid-log-single.log |
| `300cba2b-0451-42aa-b053-953b4c5614a2` | orchestrator subagent | claude-sonnet-5-5 | 10 | 22 | 9289 | 346861 | 49535 | 2026-10-30T13:57:34.714Z | 2026-10-30T13:59:48.765Z | droid-log-single.log |
| `9872379b-381f-4e5e-9693-6c1266232c5f` | orchestrator subagent | claude-sonnet-5-5 | 14 | 30 | 11704 | 503437 | 43715 | 2026-10-30T14:10:47.231Z | 2026-10-30T14:13:21.031Z | droid-log-single.log |
| `216f37f9-c483-437e-9427-16920945c713` | validator | claude-sonnet-5-5 | 7 | 16 | 9900 | 229802 | 32910 | 2026-10-30T14:16:01.485Z | 2026-10-30T14:17:27.169Z | droid-log-single.log |
| `d97c2af9-4f4c-42c2-9662-4961f2ed2879` | validator | claude-sonnet-5-5 | 37 | 76 | 31593 | 3358400 | 121223 | 2026-10-30T14:16:01.747Z | 2026-10-30T14:21:33.898Z | droid-log-single.log |
| `c7a47f01-55e7-4514-b082-64e11dfab2aa` | validator | claude-sonnet-5-5 | 64 | 130 | 54050 | 7686664 | 165508 | 2026-10-30T14:16:02.236Z | 2026-10-30T14:26:16.848Z | droid-log-single.log |
| `5076cba7-7e42-462a-94b5-2990a3ec0691` | orchestrator subagent | claude-sonnet-5-5 | 15 | 32 | 11906 | 601683 | 45101 | 2026-10-30T14:30:03.103Z | 2026-10-30T14:32:47.300Z | droid-log-single.log |
| `a0f419cc-0f1f-42f6-890c-70deabd683b8` | validator | claude-sonnet-5-5 | 47 | 96 | 33231 | 3800679 | 138289 | 2026-10-30T14:36:51.233Z | 2026-10-30T14:43:13.542Z | droid-log-single.log |
| `f902e4d5-68e3-4508-94a6-c1f6d683c6d7` | validator | claude-sonnet-5-5 | 45 | 92 | 31639 | 3402067 | 122395 | 2026-10-30T14:36:51.285Z | 2026-10-30T14:42:14.092Z | droid-log-single.log |
| `798f1689-2b13-4492-954a-ddbcf6abf66c` | validator | claude-sonnet-5-5 | 23 | 48 | 19749 | 1266605 | 67791 | 2026-10-30T14:36:51.478Z | 2026-10-30T14:40:26.255Z | droid-log-single.log |
| `f387b7e3-8458-4f8c-8c02-8cbe90a3b019` | validator | claude-sonnet-5-5 | 32 | 66 | 25154 | 1580069 | 54228 | 2026-10-30T14:36:51.589Z | 2026-10-30T14:41:18.826Z | droid-log-single.log |
| `463599b2-b930-4384-b705-01479509b331` | validator | claude-sonnet-5-5 | 41 | 84 | 36487 | 3274602 | 106861 | 2026-10-30T14:36:51.862Z | 2026-10-30T14:44:50.309Z | droid-log-single.log |
| `4f059e0e-6dc6-43e5-891d-3531349732b3` | orchestrator subagent | claude-sonnet-5-5 | 14 | 30 | 17386 | 639118 | 74510 | 2026-10-30T14:47:37.596Z | 2026-10-30T14:50:56.914Z | droid-log-single.log |
| `dbb05bca-a42f-4dfd-b653-b49db89bd513` | orchestrator subagent | claude-sonnet-5-5 | 11 | 24 | 10031 | 370676 | 52554 | 2026-10-30T14:59:33.776Z | 2026-10-30T15:00:55.816Z | droid-log-single.log |
| `89ebee66-ae29-42be-8051-fa2221b5e942` | validator | claude-sonnet-5-5 | 31 | 64 | 25861 | 2112033 | 83032 | 2026-10-30T15:25:15.192Z | 2026-10-30T15:29:42.812Z | droid-log-single.log |
| `fe2e8eb4-b55f-494d-8255-7198d100091c` | validator | claude-sonnet-5-5 | 16 | 34 | 16719 | 787299 | 64096 | 2026-10-30T15:25:15.361Z | 2026-10-30T15:27:54.917Z | droid-log-single.log |
| `a235722f-8585-4a37-846b-dccb400921ef` | validator | claude-sonnet-5-5 | 33 | 68 | 32212 | 2671885 | 121838 | 2026-10-30T15:25:15.400Z | 2026-10-30T15:30:00.074Z | droid-log-single.log |
| `9e211f9f-c99e-461e-8cae-0a5578b5b2ae` | orchestrator subagent | claude-sonnet-5-5 | 7 | 16 | 5990 | 215393 | 43623 | 2026-10-30T15:27:48.904Z | 2026-10-30T15:29:06.683Z | droid-log-single.log |
| `4305d8fd-0dbf-40a6-aac6-bcf3efd718c2` | orchestrator subagent | claude-sonnet-5-5 | 2 | 6 | 801 | 32289 | 15221 | 2026-10-30T15:34:45.073Z | 2026-10-30T15:34:49.193Z | droid-log-single.log |

### Model-Router transitions

| Time | Session | Role | Source | Reason | From | To | Source file |
|---|---|---|---|---|---|---|---|
| none | - | - | - | - | - | - | - |

model fallback: 587 turns on non-declared models, 0 router transitions

Declared models: orchestrator `claude-fable-5.1`; worker `claude-sonnet-5-5`; validator `claude-opus-5-5`; self-check `claude-haiku-4-5-20251001`.

### Throttled

| Pause | Resume | Seconds | Reason | Throttled | Source |
|---|---|---|---|---|---|
| none | - | - | - | - | - |

throttled: 0.000 s = 0.00 min

### Owner interventions

| Time | Kind | Feature unblocked | Detail | Source |
|---|---|---|---|---|
| none | - | - | - | - |

owner interventions: 0 (none)

Owner messages that unblocked the run: `mission_resumed` records of progress_log.jsonl (the feature is that of the session it resumes, else of the next `worker_started`), the answer to each `AskUser` call and each `StartMissionRun` carrying a `messageToWorker`, both read from the orchestrator transcript (the feature is that of the session the message resumes, else of the next `worker_started`).

### Idle time by cause

| Cause | Gaps | Minutes |
|---|---|---|
| usage limit | 0 | 0.00 |
| host | 0 | 0.00 |
| owner | 0 | 0.00 |
| unexplained | 1 | 34.94 |
| total | 1 | 34.94 |

unexplained gap: 2026-10-30T12:17:16.860Z to 2026-10-30T12:52:13.354Z (34.94 min)

A gap is the time between two consecutive events inside the window: a model turn in the Droid log (any session) or a record of progress_log.jsonl. It is idle time when it lasts more than 15 minutes. Its cause is the first that has evidence within 60 s of the gap: `usage limit` (a throttled pause in progress_log.jsonl, or a `[LLM] Chat route failure` with reason `paymentRequired`), `host` (`ECONNREFUSED`, `EADDRNOTAVAIL`, `ENOBUFS`, `ENETDOWN`, `ENETUNREACH`, or a route failure with another reason, in the Droid log), `owner` (an owner pause, `mission_paused` without a throttle reason, overlapping the gap, or an owner intervention at it); a gap with none of these is `unexplained`.

### Idle gaps

| Start | End | Minutes | Cause | Evidence |
|---|---|---|---|---|
| 2026-10-30T12:17:16.860Z | 2026-10-30T12:52:13.354Z | 34.94 | unexplained | none |

### Host health

sysctl kern.boottime: kern.boottime: { sec = 1793340755, usec = 785583 } Fri Oct 30 07:12:35 2026

| Check | Value |
|---|---|
| Boot time (UTC) | 2026-10-30T06:12:35.000Z |
| Uptime at generation | 0.39 days |
| Uptime at the window end | 0.39 days |
| TCP clock limit | 49.7 days |
| Days to the limit at generation | 49.31 |

### Model call faults

| Kind | Lines | Calls | First | Last |
|---|---|---|---|---|
| none | - | - | - | - |

stalled or retried model calls, usage limit excluded: 0

host verdict: did not inflate the numbers: no idle time by host faults; uptime 0.39 days (limit 49.7); 0 stalled or retried calls

Model call faults are the window's Droid log lines `[LLM] Chat route failure` (by reason), lines with `"retried":true` or `"providerRotated":true`, and the errno codes ECONNREFUSED, EADDRNOTAVAIL, ENOBUFS, ENETDOWN, ENETUNREACH; `Calls` counts distinct `assistantMessageId`. A usage-limit failure (`paymentRequired`) is the plan's stop, not the host's, so it is shown but left out of the stalled or retried count. The verdict says `inflated` when the window has idle time by host faults or the host's uptime at the window end passed the macOS TCP clock limit (2**32 ms, 49.7 days), and `did not inflate` otherwise.

### Lead context rewrites

| Time | Cache write | Cache read | Cache write minus read | USD-eq |
|---|---|---|---|---|
| none | - | - | - | - |

context size: 165807 tokens (median nonzero cache read of 138 lead turns)

### Lead context rewrites per milestone

| Milestone | Lead turns | Rewrite turns | Context size | Cache write | Cache read | USD-eq | Lead USD-eq | Share of lead cost % |
|---|---|---|---|---|---|---|---|---|
| m6 | 138 | 0 | 165807 | 0 | 0 | 0.00 | 24.82 | 0.0 |

context rewrites: none in 138 lead turns

orchestrator turns per feature boundary: 138 / 6 = 23.0 (target 3)

single-call share: 52 / 52 = 100.0 % (turns with exactly one tool call over turns with at least one; 0 turns with two or more calls, 86 turns with none)

A rewrite is a lead turn whose cache-write tokens exceed its cache-read tokens by at least the context size: the turn wrote a new cache of roughly the whole conversation instead of extending the old one (a compaction, a cache expiry after a long wait, or a resume). The context size is measured per window from the lead's own turns, not fixed; the USD-eq of a rewrite turn is the whole turn (its input, output, cache read and cache write), and the lead is the orchestrator role. The tokens are the log's `cachedTokensWritten` and `cacheReadInputTokens`.

The turns per feature boundary are the lead turns of the window divided by the `worker_completed` entries of the mission's progress_log.jsonl in the window (one per worker hand-off returned to the orchestrator; `worker_failed` is not one). The before/after line appears only when the window holds a compaction marker, a `mission_resumed` entry of the progress_log or a lead context rewrite above, and splits at the earliest one.

The single-call share counts the assistant records of the orchestrator transcript in the window: a turn with exactly one `tool_use` block (a Script is one call) over the turns with at least one; a turn with none is a closing text and counts in neither, and parallel calls in one turn are what the share is meant to raise a call to.

### User testing rounds

| Round | Session | Flow groups | Subagent sessions | Validator turns | Validator USD-eq | Subagent USD-eq | Total USD-eq | First ts | Last ts |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `916eeef6-d3e3-4f85-908f-a37e0273ff98` | 5 | 5 | 18 | 2.13 | 4.02 | 6.15 | 2026-10-30T14:33:19.716Z | 2026-10-30T14:46:44.168Z |
| 2 | `f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38` | 3 | 3 | 17 | 1.58 | 1.98 | 3.56 | 2026-10-30T15:23:18.545Z | 2026-10-30T15:33:01.200Z |

### User testing flow groups

| Round | Group | Subagent sessions | Turns | USD-eq | Models | First ts | Last ts |
|---|---|---|---|---|---|---|---|
| 1 | Test assertions usage-decisions | 1 | 47 | 1.06 | claude-sonnet-5-5 | 2026-10-30T14:36:51.233Z | 2026-10-30T14:43:13.542Z |
| 1 | Test assertions reports-docs | 1 | 45 | 0.96 | claude-sonnet-5-5 | 2026-10-30T14:36:51.285Z | 2026-10-30T14:42:14.092Z |
| 1 | Test assertions git-history | 1 | 23 | 0.49 | claude-sonnet-5-5 | 2026-10-30T14:36:51.478Z | 2026-10-30T14:40:26.255Z |
| 1 | Test assertions git-scope | 1 | 32 | 0.55 | claude-sonnet-5-5 | 2026-10-30T14:36:51.589Z | 2026-10-30T14:41:18.826Z |
| 1 | Test assertions gates-ci | 1 | 41 | 0.96 | claude-sonnet-5-5 | 2026-10-30T14:36:51.862Z | 2026-10-30T14:44:50.309Z |
| 2 | Round 2 waivers git gh | 1 | 31 | 0.68 | claude-sonnet-5-5 | 2026-10-30T15:25:15.192Z | 2026-10-30T15:29:42.812Z |
| 2 | Round 2 docs addenda 002 015 | 1 | 16 | 0.41 | claude-sonnet-5-5 | 2026-10-30T15:25:15.361Z | 2026-10-30T15:27:54.917Z |
| 2 | Round 2 waivers usage reports | 1 | 33 | 0.89 | claude-sonnet-5-5 | 2026-10-30T15:25:15.400Z | 2026-10-30T15:30:00.074Z |

user testing: 2 rounds, 8 flow groups, 9.71 USD-eq (validator sessions 3.71, subagents 6.00)

A round is a root session of the feature `user-testing-validator-*` with turns in the window, numbered by its first turn; its cost is the session's own turns plus every subagent session under it (nested ones too). A flow group is a subagent the validator started with the Task tool: its `callingToolUseId` in the sidecar is matched to the `description` of that Task call in the validator's transcript, and the subagents it started count in its group. Scrutiny validators are not user testing.

### E2E runs

| Run | Outcome | Specs | First attempt s | Rerun s | Started | Source |
|---|---|---|---|---|---|---|
| none | - | - | - | - | - | - |

e2e runs: none

`flaked` is a first attempt that failed and one rerun that passed: the run is green and counted as a flake, never as a pass; `failed` is a rerun that failed too (or a failure that named no spec). The runs are the records `reports/e2e/<date>/<HHMMSS>/record.json` that overlap the window (every run directory, so a flake stays counted after a later green run); the evidence of the first failure is in `reports/e2e/<date>/<HHMMSS>/first-failure/`, which is git-ignored and goes with the `e2e-reports` artifact of the CI run.

### Gates

| Kind | Gate | Command | Count | Seconds | Minutes | First | Last | Source |
|---|---|---|---|---|---|---|---|---|
| gates run | all | - | 0 | 0.000 | 0.00 | - | - | `reports/gate/*/*.json` |
| gates served from cache | all | - | 0 | 0.000 | 0.00 | - | - | `reports/gate/hits.log` |

gate runs in the window: 0
gate seconds run: 0.000
gates served from cache: 0, minutes saved: 0.00

Gate window rule: a gate run is in the table when it overlaps the window: `started_at` < `--to` or `finished_at` <= `--to`, and it did not finish before the window start; a run that started before `--to` and finished after it is in, one that started at or after `--to` is not (pass a later `--to` to include the integration's own gate runs).

Gates run are the records `reports/gate/*/*.json`; gates served from cache are the lines of `reports/gate/hits.log` by time, and the minutes saved are their stored seconds, the run each hit did not repeat.

### Gate runs

| Gate | Tree | Exit | Seconds | Started | Finished | Session | Source |
|---|---|---|---|---|---|---|---|
| none | - | - | - | - | - | - | - |

### Validation overhead

| Session | Feature | First turn | Last turn | Minutes (log) | Started | Completed | Minutes (progress_log) | Source |
|---|---|---|---|---|---|---|---|---|
| `aa409f08-940d-492a-9be7-c999ec737966` | scrutiny-validator-final | 2026-10-30T14:13:53.194Z | 2026-10-30T14:28:53.660Z | 15.01 | 2026-10-30T14:13:45.564Z | 2026-10-30T14:28:53.984Z | 15.14 | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl |
| `916eeef6-d3e3-4f85-908f-a37e0273ff98` | user-testing-validator-final | 2026-10-30T14:33:19.716Z | 2026-10-30T14:46:44.168Z | 13.41 | 2026-10-30T14:33:12.282Z | 2026-10-30T14:46:44.415Z | 13.54 | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl |
| `f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38` | user-testing-validator-final | 2026-10-30T15:23:18.545Z | 2026-10-30T15:33:01.200Z | 9.71 | 2026-10-30T15:23:10.483Z | 2026-10-30T15:33:01.433Z | 9.85 | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl |

validation overhead minutes (log): 38.13
validation overhead minutes (progress_log): 38.53

### Feature size

| Feature | Skill | Rounds | Sessions | Turns | Over 120 turns | Turn budget | Over budget | USD-eq | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|
| final-open-pr | integration-worker | 2 | `20b76c14-904c-45ad-9460-0b04bf5c1fea` | 23 | no | 80 | no | 0.66 | 2026-10-30T13:57:42.616Z | 2026-10-30T14:08:46.696Z | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| m6-boundary-cross-002-007-ruling | infra-worker | 0 | - | 0 | no | - | - | 0.00 | - | - | `-`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| m6-carryover-docs-addenda | infra-worker | 1 | `f5c0baea-3290-48db-8abe-1300c7144b9a` | 30 | no | 30 | no | 0.79 | 2026-10-30T15:01:20.826Z | 2026-10-30T15:23:09.616Z | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| m6-carryover-fix-pr-gate | core-worker | 1 | `1bc8d998-cdd2-426c-8eb4-a507a5c1d97b` | 64 | no | 60 | yes | 2.13 | 2026-10-30T13:10:11.862Z | 2026-10-30T13:56:53.352Z | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| m6-owner-screenshots-process-record | infra-worker | 0 | - | 0 | no | - | - | 0.00 | - | - | `-`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| m6-reference-csv-oracle-parity | core-worker | 0 | - | 0 | no | - | - | 0.00 | - | - | `-`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| scrutiny-validator-final | scrutiny-validator | 1 | `aa409f08-940d-492a-9be7-c999ec737966` | 17 | no | - | - | 4.77 | 2026-10-30T14:13:53.194Z | 2026-10-30T14:28:53.660Z | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |
| user-testing-validator-final | user-testing-validator | 2 | `916eeef6-d3e3-4f85-908f-a37e0273ff98`, `f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38` | 35 | no | - | - | 9.71 | 2026-10-30T14:33:19.716Z | 2026-10-30T15:33:01.200Z | `droid-log-single.log`; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl; e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json |

Rounds are the `worker_started` entries for the feature in `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/progress_log.jsonl`; turns and the first and last timestamps are those of its root sessions' log turns in the window; the skill and the `turnBudget` are the feature's `skillName` and `turnBudget` in `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json` (`-` when it has none; the features of the milestone that never ran are listed with 0 turns); USD-eq is the cost of the feature's sessions and of the subagents they started.

### Cost per assertion and per feature

| Unit | Count | Window USD-eq | USD-eq per unit | Source |
|---|---|---|---|---|
| assertion fulfilled | 0 | 48.34 | - | `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/validation-state.json` |
| feature completed | 1 | 48.34 | 48.34 | `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/features.json` |

assertions fulfilled in the window: none

Cost per assertion: the window total of Tokens per role divided by the assertions with status `passed` and `validatedAtMilestone` equal to the milestone (the state file carries no timestamps, so the milestone is its window); cost per feature divides the same total by the milestone's features with status `completed` in `features.json`. Fix rounds and re-tests are inside the window total.

weekly allowance share: unavailable (no assertion fulfilled in m6)

The orchestrator states the weekly allowance share before it dispatches the next milestone: the cost per assertion of this window (Cost per assertion and per feature) times the assertions the next milestone's features claim in `fulfills`. It is a forecast of the share, not a cap; the weekly allowance is the owner's to manage (D-038).

### Ratios

| Milestone | Assertions fulfilled | Worker cost per assertion | Coordination ratio | Validation ratio | Tooling ratio | Integration ratio | Cost per assertion | All-in cost per assertion | Source |
|---|---|---|---|---|---|---|---|---|---|
| m3 (recorded) | - | - | 0.90 | 1.40 | - | - | - | - | owner item 10: coordination about 0.9, validation about 1.4 |
| m4 (recorded) | 33 | 1.06 | 2.86 | 1.30 | 0.85 | - | 6.37 | 6.70 | owner item 10: worker cost on contract features 35 USD-eq, 1.06 per assertion; all-in 6.7 |
| m6 (this run) | 0 | - | 38.41 | 18.38 | 2.71 | 0.84 | - | - | this report: Tokens per role, Feature size, validation-state.json |

ratios: not available (no contract worker cost or no assertion fulfilled in m6)

bases: window 2026-10-30T12:15:25Z to 2026-10-30T15:34:49Z; w = worker cost on features that claim at least one contract assertion 0.79 USD-eq / 0 assertions landed (blocked is not landed); coordination = orchestrator with its subagents 30.27 / w; validation = validators with their seats 14.48 / w; tooling = worker cost on features that claim none 2.13 / w; integration = 0.66 / w

m4 kinds (recorded): 12 mechanical, 14 behavioural, 6 mixed

Cost per assertion = worker cost per assertion x (1 + coordination ratio + validation ratio + tooling ratio) (owner item 10 of the M4 boundary), on the definition the owner's delegate pinned (inbox 0145). The window is the milestone's build window, the first own worker dispatch to the integration landing. The contract worker is the worker cost on the features that claim at least one contract assertion in `fulfills` (their sessions and the subagents they started, fix rounds included), whatever their skill; w is that cost over the assertions landed (`passed` at the milestone; a blocked assertion is not landed). Coordination is the orchestrator with its subagents, validation the validators with their seats (scrutiny and user testing), tooling the worker cost on the features that claim no contract assertion; each ratio is that bucket's USD-eq over the contract worker's. Integration (the integration-worker features) is its own term, in the table and under it but outside the product. The self-check missions and the other sessions are in the window total (Cost by bucket) but in no ratio, so the cost per assertion stays at or under the all-in figure (window total over assertions landed). The recorded rows are the owner's own derivation, read by hand with other bases (the day's log, a share of all worker cost); the same decomposition has given different figures on different bases, and this report states its own in the `bases` line.

### Cost by bucket

| Bucket | Members | USD-eq | Ratio to contract worker cost |
|---|---|---|---|
| contract worker | worker features that claim at least one contract assertion | 0.79 | 1.00 |
| coordination | orchestrator and orchestrator subagent roles | 30.27 | 38.41 |
| validation | validator role, seats included | 14.48 | 18.38 |
| tooling | worker features that claim no contract assertion | 2.13 | 2.71 |
| integration | integration-worker features | 0.66 | 0.84 |
| self-check and other | self-check and other roles, worker sessions of features with no skill | 0.00 | 0.00 |
| window total | all roles | 48.34 | 61.33 |

### Weighted cost per assertion

| Basis | Units | Worker cost per unit | Cost per unit |
|---|---|---|---|
| raw assertion | 0 | - | - |
| plan-weighted standard assertion | 1.29 | 0.61 | 37.03 |
| kind-weighted mechanical assertion | 0.00 | - | - |

kinds landed: 0 mechanical, 0 behavioural, 0 mixed, 0 unclassified; weights 1.00 / 1.25 / 1.50 (unclassified 1.00) from `config/usage-ratios.toml`; blocked is not landed

plan-weighted: 1 contract features carry a turnBudget (30 planned turns), 0 have none and are left out

Raw is the contract worker cost over the assertions fulfilled; plan-weighted divides the cost of the contract features that carry a `turnBudget` by their planned turns over 23.3, so a standard assertion is 23.3 planned turns (the worker turns one assertion cost on M3's record, `config/usage-ratios.toml`); kind-weighted divides the contract worker cost by the landed assertions counted at their kind's weight (a mechanical assertion is one unit, so the unit is the cheapest assertion); all three are multiplied by the same three ratios.

### Turns per assertion by assertion kind

| Kind | Assertions | Attributed worker turns | Turns per assertion | Attributed worker USD-eq | USD-eq per assertion |
|---|---|---|---|---|---|
| mechanical | 0 | 0.0 | - | 0.00 | - |
| behavioural | 0 | 0.0 | - | 0.00 | - |
| mixed | 0 | 0.0 | - | 0.00 | - |
| unclassified | 0 | 0.0 | - | 0.00 | - |

fulfilled assertions claimed by no worker feature: 0

The kind of an assertion is the `Kind:` line under its heading in the validation contract (`unclassified` when it has none). A worker feature's turns and cost are split equally over the fulfilled assertions it claims in `fulfills`, so an assertion claimed by two features takes a share of each; features that claim nothing (fix rounds, tooling) are in no kind, and the claims of the milestone are partial where a feature leaves `fulfills` empty.

### Turns per assertion by worker skill

| Skill | Features | Turns | USD-eq | Assertions claimed | Turns per claimed assertion | Turns per landed assertion |
|---|---|---|---|---|---|---|
| core-worker | 1 | 64 | 2.13 | 0 | - | - |
| ui-worker | 0 | 0 | 0.00 | 0 | - | - |
| infra-worker | 1 | 30 | 0.79 | 0 | - | - |
| integration-worker | 1 | 23 | 0.66 | 0 | - | - |
| contract (core + ui) | 1 | 64 | 2.13 | 0 | - | - |
| all workers | 3 | 117 | 3.59 | 0 | - | - |

Turns are the log turns of the skill's root sessions in the window (features with turns only); a landed assertion is one fulfilled in the window, so the per-landed column adds up across skills and the contract row is the D-045 figure (core and ui turns over all assertions fulfilled); the per-claimed column divides by the fulfilled assertions the skill's features claim, an upper bound where `fulfills` is partial.

### Turns per landed assertion by role

| Role | Turns | Landed assertions | Turns per landed assertion |
|---|---|---|---|
| orchestrator | 138 | 0 | - |
| orchestrator subagent | 211 | 0 | - |
| worker | 117 | 0 | - |
| validator | 428 | 0 | - |
| self-check | 0 | 0 | - |
| other | 0 | 0 | - |
| total | 894 | 0 | - |

### Mission to date by role

| Role | Sessions | Turns | Input | Output | Cache read | Cache write | USD-eq | Unpriced turns | Cap USD-eq | Over cap | First ts | Last ts | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| orchestrator | 1 | 1371 | 241098 | 1071255 | 203160852 | 14644052 | 287.90 | 0 | - | - | 2026-10-25T06:56:41.622Z | 2026-10-30T15:34:49.628Z | droid-log-single.log, droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-25.2, droid-log-single.log.2026-10-29, droid-log-single.log.2026-10-29.1, droid-log-single.log.2026-10-29.2, droid-log-single.log.2026-10-30, droid-log-single.log.2026-10-30.1 |
| orchestrator subagent | 34 | 971 | 2186 | 1254320 | 88369918 | 10997839 | 76.85 | 0 | - | - | 2026-10-25T06:59:32.815Z | 2026-10-30T15:34:49.193Z | droid-log-single.log, droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-29, droid-log-single.log.2026-10-29.1, droid-log-single.log.2026-10-29.2, droid-log-single.log.2026-10-30, droid-log-single.log.2026-10-30.1 |
| worker | 86 | 4049 | 57802 | 4207091 | 495282688 | 17409758 | 135.31 | 0 | - | - | 2026-10-25T07:59:07.577Z | 2026-10-30T15:23:09.616Z | droid-log-single.log, droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-25.2, droid-log-single.log.2026-10-29, droid-log-single.log.2026-10-29.1, droid-log-single.log.2026-10-29.2, droid-log-single.log.2026-10-30, droid-log-single.log.2026-10-30.1 |
| validator | 138 | 2876 | 337450 | 3449981 | 232422713 | 19800580 | 183.50 | 0 | - | - | 2026-10-25T13:48:21.791Z | 2026-10-30T15:33:01.200Z | droid-log-single.log, droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-25.2, droid-log-single.log.2026-10-29.1, droid-log-single.log.2026-10-29.2, droid-log-single.log.2026-10-30, droid-log-single.log.2026-10-30.1 |
| self-check | 113 | 858 | 2996291 | 452979 | 17806288 | 460177 | 19.73 | 0 | - | - | 2026-10-25T13:22:41.516Z | 2026-10-30T09:24:36.305Z | droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-26, droid-log-single.log.2026-10-27, droid-log-single.log.2026-10-28, droid-log-single.log.2026-10-30.1 |
| other | 15 | 53 | 167188 | 25518 | 734892 | 77989 | 1.06 | 0 | - | - | 2026-10-25T15:17:51.561Z | 2026-10-30T06:57:42.598Z | droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-26, droid-log-single.log.2026-10-27, droid-log-single.log.2026-10-30.1 |
| total | 387 | 10178 | 3802015 | 10461144 | 1037777351 | 63390395 | 704.35 | 0 | - | - | 2026-10-25T06:56:41.622Z | 2026-10-30T15:34:49.628Z | droid-log-single.log, droid-log-single.log.2026-10-25, droid-log-single.log.2026-10-25.1, droid-log-single.log.2026-10-25.2, droid-log-single.log.2026-10-26, droid-log-single.log.2026-10-27, droid-log-single.log.2026-10-28, droid-log-single.log.2026-10-29, droid-log-single.log.2026-10-29.1, droid-log-single.log.2026-10-29.2, droid-log-single.log.2026-10-30, droid-log-single.log.2026-10-30.1 |
| milestone window total | 31 | 894 | 2036 | 778724 | 77268572 | 4478215 | 48.34 | 0 | - | - | 2026-10-30T12:15:25.788Z | 2026-10-30T15:34:49.628Z | droid-log-single.log |

Role rows and `total` are every log line up to the end of the window, no start bound. `milestone window total` is the total row of Tokens per role (the window only). Caps are those passed with `--cap role=USD`, `--cap total=USD` and `--cap milestone=USD`.

Price provenance: the observer's price table of 25-oct-2026 (resume/roci/tools/milestones.py), secondary public sources, used for every milestone reading S to M3; the owner has seen those readings. Not estimates: list prices at the date named.

Models with no price before the window: none.

### Sidecar readings

Read from the session sidecars at generation time: lifetime totals of each session, all models, not windowed, and never added to the log rows (D-029). A session still running (the orchestrator, an integrator) moves between two runs with the same `--to`; this is the only section that can, and it is the only source of credits.

sidecars read: 650, oldest mtime 2026-10-22T09:43:54Z, newest mtime 2026-10-30T15:35:19Z

#### Credits by role (sidecar, cumulative)

| Role | Credits | Sessions with a sidecar |
|---|---|---|
| orchestrator | 280732532 | 1 |
| orchestrator subagent | 4228844 | 13 |
| worker | 1435953 | 3 |
| validator | 5794512 | 14 |
| self-check | 0 | 0 |
| other | 0 | 0 |

#### Sidecar totals per session (cumulative)

| Session | Role | Model | Effort | Input | Output | Cache read | Cache write | Thinking | Credits | Active ms | Modified | Source |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c` | orchestrator | claude-fable-5.1 | high | 4622311 | 2193471 | 284361016 | 38917974 | 524422 | 280732532 | 145642861 | 2026-10-30T15:35:00Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c.settings.json` |
| `b9b33259-e554-4504-a128-c93773aa0471` | orchestrator subagent | claude-sonnet-5-5 | high | 22 | 7227 | 342583 | 29301 | 471 | 71937 | 120698 | 2026-10-30T12:17:01Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/b9b33259-e554-4504-a128-c93773aa0471.settings.json` |
| `60f8be35-75ef-4878-bf2f-392f46b45c74` | orchestrator subagent | claude-sonnet-5-5 | high | 205990 | 162654 | 12148946 | 1148513 | 48356 | 2449928 | 1349541 | 2026-10-30T15:00:30Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/60f8be35-75ef-4878-bf2f-392f46b45c74.settings.json` |
| `530ca869-25a4-458f-8302-c580c652c121` | orchestrator subagent | claude-sonnet-5-5 | high | 48 | 16161 | 856901 | 53030 | 1259 | 152001 | 143191 | 2026-10-30T12:57:03Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/530ca869-25a4-458f-8302-c580c652c121.settings.json` |
| `d039a35e-6fe2-4b15-9e68-24197cc37c00` | orchestrator subagent | claude-sonnet-5-5 | high | 64 | 28878 | 1906284 | 89123 | 8851 | 280954 | 267398 | 2026-10-30T12:59:58Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/d039a35e-6fe2-4b15-9e68-24197cc37c00.settings.json` |
| `a759d47b-6872-405f-9695-1b4921f0c390` | orchestrator subagent | claude-sonnet-5-5 | high | 48 | 32721 | 1568315 | 89573 | 4186 | 283238 | 248021 | 2026-10-30T13:03:02Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/a759d47b-6872-405f-9695-1b4921f0c390.settings.json` |
| `d7076bb5-9bb9-4b3b-8e99-d164d18948e9` | orchestrator subagent | claude-sonnet-5-5 | high | 64 | 35865 | 1626603 | 67183 | 11428 | 275773 | 342644 | 2026-10-30T13:09:45Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/d7076bb5-9bb9-4b3b-8e99-d164d18948e9.settings.json` |
| `300cba2b-0451-42aa-b053-953b4c5614a2` | orchestrator subagent | claude-sonnet-5-5 | high | 22 | 9289 | 346861 | 49535 | 925 | 100587 | 137214 | 2026-10-30T13:59:48Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/300cba2b-0451-42aa-b053-953b4c5614a2.settings.json` |
| `9872379b-381f-4e5e-9693-6c1266232c5f` | orchestrator subagent | claude-sonnet-5-5 | high | 30 | 11704 | 503437 | 43715 | 744 | 110698 | 156494 | 2026-10-30T14:13:21Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/9872379b-381f-4e5e-9693-6c1266232c5f.settings.json` |
| `5076cba7-7e42-462a-94b5-2990a3ec0691` | orchestrator subagent | claude-sonnet-5-5 | high | 32 | 11906 | 601683 | 45101 | 1806 | 116826 | 167090 | 2026-10-30T14:32:47Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/5076cba7-7e42-462a-94b5-2990a3ec0691.settings.json` |
| `4f059e0e-6dc6-43e5-891d-3531349732b3` | orchestrator subagent | claude-sonnet-5-5 | high | 30 | 17386 | 639118 | 74510 | 3368 | 169648 | 201988 | 2026-10-30T14:50:56Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/4f059e0e-6dc6-43e5-891d-3531349732b3.settings.json` |
| `dbb05bca-a42f-4dfd-b653-b49db89bd513` | orchestrator subagent | claude-sonnet-5-5 | high | 24 | 10031 | 370676 | 52554 | 754 | 107529 | 84412 | 2026-10-30T15:00:55Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/dbb05bca-a42f-4dfd-b653-b49db89bd513.settings.json` |
| `9e211f9f-c99e-461e-8cae-0a5578b5b2ae` | orchestrator subagent | claude-sonnet-5-5 | high | 16 | 5990 | 215393 | 43623 | 905 | 76216 | 80919 | 2026-10-30T15:29:06Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/9e211f9f-c99e-461e-8cae-0a5578b5b2ae.settings.json` |
| `4305d8fd-0dbf-40a6-aac6-bcf3efd718c2` | orchestrator subagent | claude-sonnet-5-5 | high | 10 | 1441 | 81693 | 24468 | 194 | 33509 | 0 | 2026-10-30T15:35:19Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/4305d8fd-0dbf-40a6-aac6-bcf3efd718c2.settings.json` |
| `1bc8d998-cdd2-426c-8eb4-a507a5c1d97b` | worker | claude-sonnet-5-5 | high | 1483 | 37252 | 7219973 | 415625 | 12112 | 854085 | 2804504 | 2026-10-30T13:56:53Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/1bc8d998-cdd2-426c-8eb4-a507a5c1d97b.settings.json` |
| `20b76c14-904c-45ad-9460-0b04bf5c1fea` | worker | claude-sonnet-5-5 | high | 1359 | 15258 | 1417672 | 147842 | 2419 | 266138 | 667219 | 2026-10-30T14:08:47Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/20b76c14-904c-45ad-9460-0b04bf5c1fea.settings.json` |
| `f5c0baea-3290-48db-8abe-1300c7144b9a` | worker | claude-sonnet-5-5 | high | 1152 | 16985 | 1997971 | 167397 | 4276 | 315730 | 1312082 | 2026-10-30T15:23:10Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/f5c0baea-3290-48db-8abe-1300c7144b9a.settings.json` |
| `aa409f08-940d-492a-9be7-c999ec737966` | validator | claude-opus-5-5 | high | 548 | 25350 | 1466489 | 218102 | 6552 | 756536 | 908156 | 2026-10-30T14:28:54Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/aa409f08-940d-492a-9be7-c999ec737966.settings.json` |
| `216f37f9-c483-437e-9427-16920945c713` | validator | claude-sonnet-5-5 | high | 16 | 9900 | 229802 | 32910 | 1673 | 81717 | 89821 | 2026-10-30T14:17:27Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/216f37f9-c483-437e-9427-16920945c713.settings.json` |
| `d97c2af9-4f4c-42c2-9662-4961f2ed2879` | validator | claude-sonnet-5-5 | high | 76 | 31593 | 3358400 | 121223 | 9406 | 382006 | 336550 | 2026-10-30T14:21:33Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/d97c2af9-4f4c-42c2-9662-4961f2ed2879.settings.json` |
| `c7a47f01-55e7-4514-b082-64e11dfab2aa` | validator | claude-sonnet-5-5 | high | 130 | 54050 | 7686664 | 165508 | 18799 | 689304 | 619526 | 2026-10-30T14:26:16Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/c7a47f01-55e7-4514-b082-64e11dfab2aa.settings.json` |
| `916eeef6-d3e3-4f85-908f-a37e0273ff98` | validator | claude-opus-5-5 | high | 564 | 29320 | 1544514 | 247768 | 6147 | 853869 | 811874 | 2026-10-30T14:46:44Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/916eeef6-d3e3-4f85-908f-a37e0273ff98.settings.json` |
| `a0f419cc-0f1f-42f6-890c-70deabd683b8` | validator | claude-sonnet-5-5 | high | 96 | 33231 | 3800679 | 138289 | 9321 | 423341 | 384676 | 2026-10-30T14:43:13Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/a0f419cc-0f1f-42f6-890c-70deabd683b8.settings.json` |
| `f902e4d5-68e3-4508-94a6-c1f6d683c6d7` | validator | claude-sonnet-5-5 | high | 92 | 31639 | 3402067 | 122395 | 7899 | 385127 | 325219 | 2026-10-30T14:42:14Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/f902e4d5-68e3-4508-94a6-c1f6d683c6d7.settings.json` |
| `798f1689-2b13-4492-954a-ddbcf6abf66c` | validator | claude-sonnet-5-5 | high | 48 | 19749 | 1266605 | 67791 | 2765 | 197500 | 217374 | 2026-10-30T14:40:26Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/798f1689-2b13-4492-954a-ddbcf6abf66c.settings.json` |
| `f387b7e3-8458-4f8c-8c02-8cbe90a3b019` | validator | claude-sonnet-5-5 | high | 66 | 25154 | 1580069 | 54228 | 6243 | 218115 | 269854 | 2026-10-30T14:41:18Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/f387b7e3-8458-4f8c-8c02-8cbe90a3b019.settings.json` |
| `463599b2-b930-4384-b705-01479509b331` | validator | claude-sonnet-5-5 | high | 84 | 36487 | 3274602 | 106861 | 9757 | 383881 | 481444 | 2026-10-30T14:44:50Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/463599b2-b930-4384-b705-01479509b331.settings.json` |
| `f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38` | validator | claude-opus-5-5 | high | 842 | 25091 | 1698405 | 147513 | 7135 | 631962 | 590759 | 2026-10-30T15:33:01Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/f6ab0dc7-c318-4ccb-bc48-cf897ee5fc38.settings.json` |
| `89ebee66-ae29-42be-8051-fa2221b5e942` | validator | claude-sonnet-5-5 | high | 64 | 25861 | 2112033 | 83032 | 5778 | 271021 | 271792 | 2026-10-30T15:29:42Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/89ebee66-ae29-42be-8051-fa2221b5e942.settings.json` |
| `fe2e8eb4-b55f-494d-8255-7198d100091c` | validator | claude-sonnet-5-5 | high | 34 | 16719 | 787299 | 64096 | 3233 | 162501 | 163891 | 2026-10-30T15:27:54Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/fe2e8eb4-b55f-494d-8255-7198d100091c.settings.json` |
| `a235722f-8585-4a37-846b-dccb400921ef` | validator | claude-sonnet-5-5 | high | 68 | 32212 | 2671885 | 121838 | 9563 | 357632 | 289144 | 2026-10-30T15:30:00Z | `-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/a235722f-8585-4a37-846b-dccb400921ef.settings.json` |

<!-- usage:end -->

## Unit record and caps (integrator)

Written by hand by the M6 boundary integrator `m6-boundary-integrate-unit1` from the generated tables above and the scribe's record
(`MD/contract-work/m6-unit1-record.md`); every figure is pasted from the tables or the record, none is computed here except the two
quotients named. The generated block is the copy of the tool's output (`MD/contract-work/m6-unit1-usage.md`):
`python3 scripts/usage-report.py --milestone m6 --from 2026-10-30T12:15:25Z --to 2026-10-30T15:34:49Z --cross-check` (generated
2026-10-30T15:35:20Z; the first run read DIFFERENT because the live orchestrator window moved, the rerun with the same `--to` reads
`identical`: 31 sessions, 894 turns). The window starts at the M5 close (`--milestone m6`, first line 2026-10-30T12:15:25.788Z) and the
unit closed at 2026-10-30T15:33:01Z, the end of `user-testing-validator-final` round 2; the report ends at 15:34:49Z.

**Per-role tokens (rule 7, D-029).** The `Tokens per role` table above is the per-role sum of `scripts/usage-from-log.sh`, and the
`cross-check` line reads `identical` against `scripts/usage-from-log.sh --embedded` for every session (first and last line, lines,
input, output, cache read, cache write and the price of each model). Spot check of the orchestrator row, run for this record
(`sh scripts/usage-from-log.sh $L/droid-log-single.log.2026-10-30.1 $L/droid-log-single.log --session e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c
--from 2026-10-30T12:15:25Z --to 2026-10-30T15:34:49Z --embedded --prices config/prices/2026-10-29.toml`, exit 0, 4.732 s wall):
`2026-10-30T12:15:30.336Z 2026-10-30T15:34:49.628Z 138 458 92870 20655600 1200717` (first ts, last ts, turns, input, output, cache
read, cache write: the same seven values as the `orchestrator` row), `join=embedded unjoined=0 embedded_vs_metrics_disagree=0`.

| Line | Cap USD-eq | Measured USD-eq | Verdict |
|---|---|---|---|
| Unit 1 window (Tokens per role, `total`) | 49 (SPEC §9 F-367; stop-and-decide under R-055) | 48.34 | under the cap, 0.66 left |
| orchestrator | 9 (slice) | 24.82 | over its slice by 15.82; boundary-closure line 7 (3.96 orchestrator plus 2.51 subagent, 6.46 at 13:04Z) is inside the orchestrator and subagent lines |
| orchestrator subagents | 6 | 5.45 | under, 0.55 left |
| workers | 6 | 3.59 | under, 2.41 left |
| scrutiny | 5 | 4.77 | under, 0.23 left |
| user testing | 15 | 9.71 | under, 5.29 left; two rounds, 6.15 then 3.56 (`User testing rounds` above) |
| self-check | 1 | 0.00 | no scratch mission in the window |
| Mission to date | 1700 notify-and-ask line (D-049, stop-and-decide under R-055, never a wall) | 1446.87 + 48.34 = 1495.21 | under the line, 204.79 left |

Mission to date is the recorded M5 close (1446.87, F-346) plus this window's total (48.34), as at M5: the tool's own
`Mission to date by role` table above reads 704.35 because its rows start at 2026-10-25T06:56Z and the earlier logs have rotated out (BACKLOG B-254).
The sum of the five role lines is 24.82 + 5.45 + 3.59 + 4.77 + 9.71 = 48.34. Previous reading at 15:27Z: 45.26 (orchestrator 23.72,
orchestrator subagents 5.21, workers 3.59, scrutiny 4.77, user testing 7.97).

### Validation share

Validation is scrutiny 4.77 plus user testing round 1 6.15 plus round 2 3.56 = 14.48 USD-eq, the `validator` role line above.

- Share of the window: 14.48 / 48.34 = 0.2995, so **30.0 %**.
- Against the build line (workers, 3.59): 14.48 / 3.59 = 4.033, so 403.3 %.
- Time: validator sessions 15.01 + 13.41 + 9.71 = 38.13 minutes (`Validation overhead` above, log basis; 38.53 on the progress log basis)
  over the 197.6-minute window (12:15:25Z to 15:33:01Z): 19.3 %. The window includes the owner-ruling pause and the docs worker.

### Rounds

| Round | Assertions re-tested | Passed | Failed | Blocked | Head | Seats |
|---|---|---|---|---|---|---|
| Scrutiny, 1 run (`scrutiny-validator-final`, `aa409f08`) | 38 claims over 3 features | 26 | 2 | 10 unsure | `c6b3749` | 3 Sonnet seats (`d97c2af9`, `c7a47f01`, `216f37f9`) |
| User testing 1 | 38 | 23 | 15 | 0 | `c6b3749` | 5 flow groups (`798f1689`, `f387b7e3`, `463599b2`, `f902e4d5`, `a0f419cc`) |
| User testing 2 (affected-only) | 11 | 9 | 2 | 0 | `273eb9e` | 3 flow groups (`fe2e8eb4`, `89ebee66`, `a235722f`) |

- Scrutiny: one run, status pass; the 2 failed and 10 unsure claims were re-read by Opus; no blocking issue (`blockingIssues` empty);
  the three features reviewed were `final-integrate-m5`, `final-process-record` and `final-open-pr`. Cost 4.77 USD-eq (the synthesis
  states none).
- User testing round 1 failed VAL-PROC-002, 003, 005, 011, 015, 016, 023, 027, 028, 029, 032, 034, 035, 036 and 046 (15). Round 2
  passed 002, 011, 015, 023, 027, 034, 035, 036 and 046 and failed 005 and 028. VAL-PROC-003, 016, 029 and 032 were not re-graded in
  round 2 by owner ruling (row 117, F-368): failed on the record. The 23 round 1 passes were carried as evidence.
- After round 2 (`milestoneTotalsAfterRound2`): in scope 38, passed 32, failed 6 (VAL-PROC-003, 005, 016, 028, 029, 032);
  `validation-state.json` at the hand-off: 272 assertions, 252 passed, 11 failed, 6 pending, 2 blocked, 1 `handed_to_qa`.
- Models: the three validators `claude-opus-5-5` (`aa409f08`, `916eeef6`, `f6ab0dc7`); the eleven Sonnet seats `claude-sonnet-5-5`;
  Model-Router lines 0 for each.

### Verdict condition (inbox 1319): facts only

- As-worded failures after round 2: **2** (VAL-PROC-005 and 028), both wording errors in the pass 41 text, corrected by wording
  only in pass 42 (F-370) and regraded from the round 2 evidence by one Sonnet seat, no new run.
- In-unit contract passes: **1** (pass 41, owner-ruled ~14:55Z, F-368 and F-369); pass 42 is a boundary amendment made after the unit
  window closed at 15:33:01Z. The freeze began at 13:06Z with the dispatch of `m6-carryover-fix-pr-gate`.
- Validation cost 14.48 against build cost 3.59 (workers line): the condition "validation under the build's cost" is not met in
  unit 1 on these lines. The verdict is the owner's; none is written here.
- Dismissed items later failed: none. No `DismissHandoffItems` call in rows 113 to 118; the assertion greps of rows 112 and 114
  each ended in a recorded action (a B-row, an AGENTS.md bullet or a move to a feature).

### Record items

- Rule 30 (fact): the hand-off of the docs worker `m6-carryover-docs-addenda` (`f5c0baea-3290-48db-8abe-1300c7144b9a`) holds no `claims`
  array and no `model =` line.
- Turn budget (rule 29): `m6-carryover-fix-pr-gate` had `turnBudget` 60 and ran 64 turns (`Feature size` above: over budget, 2.13 USD-eq).
- Over-cap lines: window total 48.34 against 49 (0.66 left); the orchestrator line 24.82 against its slice 9. The owner ruling of
  ~15:45Z (row 120) proceeds to the M6 cut under a boundary allowance of 14 (F-371).
- Open for the M6 cut: the narrative slips of the round 2 hand-off (B-264 count, VAL-PROC-011's m3 label, the missing run id in the
  `reports/ci` m5 file, the untracked `gh pr create` build-log) ride this boundary's addenda commits; the README M5 row still quotes
  163.57 and 71.27 beside the later 186.13 and 81.83 measurement (VAL-PROC-015 boundary note).
