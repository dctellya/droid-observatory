# D-020: PII in fixtures is redacted at capture time

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-23 at the S to M0 boundary; applied to the S fixture in
`m0-fixture-redaction`, wt/m0 commit `92d2208`)

## Context

The S fixture (`fixtures/streams/2026-10-22-observatory-v1-s.*`) is a copy of this mission's own
session files. It held 83 occurrences of email addresses and of the owner's home path
(`/Users/dct`), found by the red test `fixtures_contain_no_email_or_home_path` (commit `d3e53ea`).
The B-006 scrub pass was planned for after capture; the owner moved it to capture time.

## Decision

Redact emails and `/Users/dct` paths with a deterministic rewrite (same length where possible,
e.g. `user@example.invalid`, `/home/user`), keep the original capture outside the repo with its
SHA-256 recorded next to the redacted file's SHA-256 in the decision file, and compute all byte
offsets against the redacted file, since that is the one the store indexes. Run gitleaks on the
redacted fixture before committing; same rule for every future fixture (B-006 scrub pass applies
at capture time, not after). This is `ai/RULES.md` rule 22.

Procedure as built (`scripts/redact-fixture.sh <root.jsonl>`, POSIX sh and perl, byte-wise; the
`infra-worker` skill "Fixture capture" calls it):

- It rewrites the root `.jsonl`, the `.settings.json`, the `.capture.json` and every file in
  `.children/`. A file is written only when the output differs, so a second run changes nothing
  (`make redact-check`, part of `make check`, copies `fixtures/streams`, reruns the script on
  every root and runs `diff -r`).
- Each match of the email regex becomes `user@example.invalid`. A match of 20 bytes or more is
  padded with `_` in the local part to keep its length; a shorter match takes the 20-byte token,
  so that line grows. Matches already ending in `@example.invalid` are left alone.
- `/Users/dct` becomes `/home/user` (the same 10 bytes).
- The originals are copied unchanged to `~/.droid-observatory/originals/<slug>/` before the
  rewrite. For the S fixture: `~/.droid-observatory/originals/2026-10-22-observatory-v1-s/`.
- `.capture.json` gets a `redaction` block (script, `applied_at`, rules, `originals_dir` and one
  row per `.jsonl` and `.settings.json`: path, `original_sha256`, `redacted_sha256`, `lines`).
  The `gitleaks` field records the scan of the redacted files.

Rules fixed while building it:

1. **Acceptance check** is the regex form, both commands empty:
   `rg -P "[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}" fixtures/streams | rg -v "@example\.invalid"`
   and `rg -c "/Users/dct" fixtures/streams`. The literal `rg "@"` is not a valid check: it
   matches action pins (`actions/checkout@<sha>`), the bot co-author trailer
   (`factory-droid[bot]@users.noreply.github.com`, not an email under the regex because `]` is
   not a local-part character) and Makefile recipes (`@echo`), 383 lines in the S fixture, none
   of them emails. The Rust test `fixtures_contain_no_email_or_home_path` uses the same regex.
2. **JSON escaping in `.capture.json`.** The home-path rule string is stored JSON-escaped
   (`\/Users\/dct -> \/home\/user`): the parsed value equals the spec string, but the raw bytes
   hold no literal `/Users/dct`, which `rg -c` and the test need. Any tool that rewrites
   `.capture.json` must keep that escaping.
3. **Non-personal matches are rewritten too.** The regex also matched `git@github.com` (19
   occurrences), `128x128@2x.png` (24), `docs@grafana.com`, `user@example.com` and
   `test@example.com`. Matches under 20 bytes take the 20-byte token, so those lines are longer
   than the original; line counts are unchanged. Padded addresses use `_` in the local part (for
   a 28-byte match: `user________@example.invalid`). Byte lengths of the root differ from the
   original, so all offsets (`SourcePtr`) come from the redacted bytes.
4. **`lines`** in a redaction row is `wc -l` (newline count) of the file; the settings files have
   no trailing newline, so their `lines` is one less than the number of text lines.
5. **Hyphenated session-directory form.** The form (`-Users-dct-code-...`) remained in
   `.capture.json` `source_dir` and in file contents after the first redaction. The owner extended
   the rule to it; see the addendum.

## Alternatives

Keep the fixture unredacted and private (rejected: the repository is public and replay tests need
the file in a clean clone); redact only the literal `@` or only email-shaped matches with
byte-exact padding (rejected: short matches cannot be padded, and the literal `@` is not an email
test); store a hash of the original only, without keeping the original (rejected: the owner wants
the original kept outside the repo so offsets and hashes can be re-derived).

## Consequences

Every fixture is redacted at capture, with the original under `~/.droid-observatory/originals/`
and both hashes in its decision file. The store indexes the redacted bytes. The S fixture's
byte offsets and size changed (root 7 779 681 to 7 779 749 bytes); the replay still ends with
`unknown events: 2027`. A future capture that needs a different rewrite changes this file
(rule 8) and the script's idempotence check. The hyphenated directory form is settled in the addendum.

## Addendum 2026-10-23 (M0 to M1 boundary): third rule, M0 freeze, redactor fix

Owner decisions of 2026-10-23 after the second pass (open point 5 above is settled) and the facts
the M0 worktree added. Where this section differs from the text above, this section wins.

### Third rule: the hyphenated session-directory form

`scripts/redact-fixture.sh` has a third rule, `-Users-dct` becomes `-home-user` (same 10 bytes). The
rules recorded in each `.capture.json` are, in order: `email -> user@example.invalid`,
`/Users/dct -> /home/user`, `-Users-dct -> -home-user`. `ai/RULES.md` rule 22 names the hyphenated
form. The S fixture was re-redacted from its original with all three rules (`fcfbc32`); the first
redaction, which had only two rules, is replaced. The M0 capture was redacted with the three rules
at the freeze (`c623365`).

`source_dir` in each `.capture.json` is stored in its redacted form
(`~/.factory/sessions/-home-user-code-EllyaOrg-projects-dalimaray-droid-observatory`). The real
source directory, `~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory`,
is named in this decision file only.

### Originals

| Fixture | Originals directory (outside the repo) |
|---|---|
| `2026-10-22-observatory-v1-s` | `~/.droid-observatory/originals/2026-10-22-observatory-v1-s/` |
| `2026-10-22-observatory-v1-m0` | `~/.droid-observatory/originals/2026-10-22-observatory-v1-m0/` |

Byte offsets (`SourcePtr`) are computed against the redacted file, because that is the file the
store indexes. The original is kept only to re-derive the hashes.

### Redactor fix found by mapping v1

The email rule consumed the letter of a JSON backslash escape when a local part followed it
directly (`\\n128x128@2x.png` became `\\user@example.invalid`, an invalid `\\u` escape). The red test
is `8c91e32` (redacted fixture lines stay JSON) and the fix is `a3d4e9f`: the email rule has a
lookbehind, so an address never starts right after a backslash. Six M0 children (`571a9e79`,
`82ad1581`, `ca58108e`, `d0304e2e` and two more) and two S children (`82ad1581`, `ca58108e`) were
re-redacted from the originals. Line counts and original hashes are unchanged; the redacted hashes
were updated in the rows below and in both headers. The tables below are the state at the
`wt/m0` tip `88866ef`, merged at the M0 integration.

### Hashes

The tables are `.redaction.files` of each `.capture.json`, taken with
`jq -r '.redaction.files[] | "\(.path) \(.original_sha256) \(.redacted_sha256)"'` from the merged
tree. Paths are relative to `fixtures/streams/`; `lines` is `wc -l`. A file with no match is
byte-identical after redaction (equal hashes). This file holds no other 64-hex string: a check
compares the pairs with `grep -oE '[0-9a-f]{64}'` per fixture. The hash of each `.capture.json`
itself cannot be a row (the file holds its own table); it is in `SPEC.md` §10.

#### S fixture (52 rows)

| Path | Original SHA-256 | Redacted SHA-256 | Lines |
|---|---|---|---|
| `2026-10-22-observatory-v1-s.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.jsonl` | `39dba297aa1f16a1cf2f489af8e34e60f0c42b3a454280ec7e750209326a7cb8` | `cf9880e7b01c3217ea008058e12b9418c0f4721200bd86dd3008343d133f70b6` | 98 |
| `2026-10-22-observatory-v1-s.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.settings.json` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | 50 |
| `2026-10-22-observatory-v1-s.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.jsonl` | `d417200900c2f363c07365935bb09bc17f8e6ca2678906287387438c5715abd2` | `e8be4dae52b1c8f0fe36990f6e6230da0a9344adf252f6ecb9d99402230c9bda` | 230 |
| `2026-10-22-observatory-v1-s.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.settings.json` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | 50 |
| `2026-10-22-observatory-v1-s.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.jsonl` | `857070cf8b56a8cf25fa56d55d6012038604dbb3e5a32862743f6d1f7e700eab` | `a71d1b4c44d373cc4ec90d24f8d23ea5498aa55c6d08493d52026e9ad1801613` | 85 |
| `2026-10-22-observatory-v1-s.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.settings.json` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | 50 |
| `2026-10-22-observatory-v1-s.children/12af01f5-f1d7-4487-8595-767298110266.jsonl` | `442a32a41d288dfaac184be04c778f606b3cd2a932b58cf48b2be410731a096d` | `2c494b81f10be38bd0dadb2698961d9b9086b5c633ac5eb69261d5894f2b9a3b` | 186 |
| `2026-10-22-observatory-v1-s.children/12af01f5-f1d7-4487-8595-767298110266.settings.json` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | 63 |
| `2026-10-22-observatory-v1-s.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.jsonl` | `59907103396d712cd43c759c1ef3c9d0bed62e2e7db388e3afde9ac44811b21f` | `2e0d2c473092f5edf0d3853e26d7676e0abc5eff4906c2eb4065c6597f5fd4c1` | 74 |
| `2026-10-22-observatory-v1-s.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.settings.json` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | 50 |
| `2026-10-22-observatory-v1-s.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.jsonl` | `3f15fde6bddd780eda75fa5f3fc89cb6b957ed13e010238546c7805f8bf95557` | `8ace9a93d44bf4d5320b14dc1b3e827e9ad70f3b9570bb6bc6f0524e64c0aa4b` | 115 |
| `2026-10-22-observatory-v1-s.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.settings.json` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | 50 |
| `2026-10-22-observatory-v1-s.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.jsonl` | `8be9dc51fb4062fea76c5db256a9537887f0bf50cb20007d498f65f4d538edbf` | `fc4783fb790c12518d32fd319a88a63486e151e1d4e168f24198b30acd59faf0` | 69 |
| `2026-10-22-observatory-v1-s.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.settings.json` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | 50 |
| `2026-10-22-observatory-v1-s.children/44e19307-7533-41a1-8f66-1234a1386401.jsonl` | `e06ec8de1d850b2459ad749d58a90b6e167ff16a95f1c9cfe7e6fdd3ebfd9984` | `13aec7db88c925b1d8b9cb584892e517d3dd59f9bd9c4b0513b5947fa7f0e738` | 83 |
| `2026-10-22-observatory-v1-s.children/44e19307-7533-41a1-8f66-1234a1386401.settings.json` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | 50 |
| `2026-10-22-observatory-v1-s.children/59ee8736-e49e-4091-9ade-536696563973.jsonl` | `e054989587d78e42b19e6c0326d7ee99eff74e9dfe094b44f4585470a9803ebd` | `9f81ccf4630dee6b1696569d3ddc28560def2f95f8236fba436d8cc2a8d82b41` | 38 |
| `2026-10-22-observatory-v1-s.children/59ee8736-e49e-4091-9ade-536696563973.settings.json` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | 118 |
| `2026-10-22-observatory-v1-s.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.jsonl` | `aad0b0ecfa1723c7bfa0950eb5e96fb8dea248b5c9005d8e07e9b833fce20cb2` | `3af4cecabe2e04166f864418d6ba0832047ad6b85df11fc5c9becbb7f9510167` | 32 |
| `2026-10-22-observatory-v1-s.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.settings.json` | `09e8eb863245debe0d06f06e4e9fe8fb3bf6c71f6fb2c913ae6ae3f1a1c171a3` | `09e8eb863245debe0d06f06e4e9fe8fb3bf6c71f6fb2c913ae6ae3f1a1c171a3` | 51 |
| `2026-10-22-observatory-v1-s.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.jsonl` | `0db91bb11624bb9ba1ed998d69352c1fa2471919b62f5f7fae70f7eea264e55b` | `c55c72cf496ef8e8623354fe197f62082fadf0caf1384741f210017701512af1` | 70 |
| `2026-10-22-observatory-v1-s.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.settings.json` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | 50 |
| `2026-10-22-observatory-v1-s.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.jsonl` | `c8300fb51464864dea74dc65591f6cfbdb085083a1d611df207c7a0435cfa6a8` | `fb21999e80d153c18154f22e6e6ca85b098beb319efb425dfdabd78ba81f4de9` | 61 |
| `2026-10-22-observatory-v1-s.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.settings.json` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | 50 |
| `2026-10-22-observatory-v1-s.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.jsonl` | `1e1308e9091ac36a71fb5e56358ae750e167e6f155cbf78577ff5f3e76008bd5` | `4eae8e01e6aaf2023e1242e06a604132c5a0db2b06ee1057db66473499185e1d` | 221 |
| `2026-10-22-observatory-v1-s.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.settings.json` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | 50 |
| `2026-10-22-observatory-v1-s.children/8ca62365-1d34-4e94-9473-df95d2229445.jsonl` | `84219418b5701591097fd7dc88c192c173e4f4e78bd393c6ec0426f3e9b6d4c7` | `35437c4c4abf6e0689ecd6144c78b9ad52c53b9654e4b6224cf02db802e05e40` | 51 |
| `2026-10-22-observatory-v1-s.children/8ca62365-1d34-4e94-9473-df95d2229445.settings.json` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | 64 |
| `2026-10-22-observatory-v1-s.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.jsonl` | `a75fa68900cd011b9d17d192f24d244b9f5c9bb3fedc7e761f939c1de5296959` | `0ab006b07d42e1efd8fdfb8e9b9c32d424faa0434d71b8c6cbda0f6e9b765f1e` | 132 |
| `2026-10-22-observatory-v1-s.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.settings.json` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | 50 |
| `2026-10-22-observatory-v1-s.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.jsonl` | `28a2aa75beb165ee4e9e7f2312a0aae22b0e38e9583b8d7ab0003b576c3bde79` | `25f5a79840151cb21fcfeacda5b2c49dc2120d5180cf55d47f19830e01ee7b2b` | 126 |
| `2026-10-22-observatory-v1-s.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.settings.json` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | 50 |
| `2026-10-22-observatory-v1-s.children/b19ddede-7907-444a-a16d-28ac6d1c122e.jsonl` | `a1cb9b2d737b45013a7196e1a9cc1af08a581e86d980de65e1e3d6323baa549f` | `cc1db466c3ad6f8ffad079bf037035ba46f00b723da76716fb70cf3ba14a6162` | 72 |
| `2026-10-22-observatory-v1-s.children/b19ddede-7907-444a-a16d-28ac6d1c122e.settings.json` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | 50 |
| `2026-10-22-observatory-v1-s.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.jsonl` | `d4bb130104fa844c2f4550627142fed9765f6f0ea3f25d11777538f696f91477` | `4aee0cf6b7fdefd86f3c7f6cf7e72e57df8bb6365f7d93014707c3fb27e46b9e` | 45 |
| `2026-10-22-observatory-v1-s.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.settings.json` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | 50 |
| `2026-10-22-observatory-v1-s.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.jsonl` | `8ff2af85091d5ac5b3a270e9711d374ca9125cb83ac80e481fe824979adb09fa` | `0c95885f7a056d3181860e556a1695bf1d04b862b30f0d419c7cc81f46cc6ae6` | 93 |
| `2026-10-22-observatory-v1-s.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.settings.json` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | 50 |
| `2026-10-22-observatory-v1-s.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.jsonl` | `75638ad233cf39a733d6245e9f2c34d701cf9505fdeb7adc674d43a80af4bce0` | `3b19c53aafcece1b07a71eb2da8c9105cead5d6b88f6f6d4232d935b3ed20dd5` | 144 |
| `2026-10-22-observatory-v1-s.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.settings.json` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | 51 |
| `2026-10-22-observatory-v1-s.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.jsonl` | `f1b9c035808c67f31d5d21af1fe30090755dd9cb91809e9ee7241b482eb3569b` | `d742f57a74d3d21a73fccf1432bea1df0125a237266b60cf08e84d1192cfc617` | 80 |
| `2026-10-22-observatory-v1-s.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.settings.json` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | 50 |
| `2026-10-22-observatory-v1-s.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.jsonl` | `05cdfa7a8b2907aa9befa42cfeae61c2f9dc44e7c79d455a573053c94bc20bfc` | `208e1919a566c253d50463983e3cb3927e479fead5d01abb80c9ee800a7d4a63` | 116 |
| `2026-10-22-observatory-v1-s.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.settings.json` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | 50 |
| `2026-10-22-observatory-v1-s.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.jsonl` | `505347611712c111eb65b7a3939c7f2c26da874fcc7b9fa07ffe03f4cbb83d61` | `8c40df04e87a7769462f5189f0bebca9e4d783666ba862e0b1a2eef8af4cc749` | 231 |
| `2026-10-22-observatory-v1-s.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.settings.json` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | 50 |
| `2026-10-22-observatory-v1-s.children/e2e3d113-d82b-4548-8809-4ebd269d1629.jsonl` | `ae46cfd55454332e662c7822b9cc70889335353f38fc24a5873a2765e8e62a48` | `689470e7c266ddc0a70483daf1526bf7235f9eb70038f99800efb361fbfb1990` | 70 |
| `2026-10-22-observatory-v1-s.children/e2e3d113-d82b-4548-8809-4ebd269d1629.settings.json` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | 118 |
| `2026-10-22-observatory-v1-s.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.jsonl` | `9e9da826f322d68532a7513e668d47619d61da71e0281a84f0401973eb7aad7b` | `2cb9edd54574e9b64e2b7d04ee097a9bb7fea8b8beaa6ce7b937725648895bdb` | 87 |
| `2026-10-22-observatory-v1-s.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.settings.json` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | 50 |
| `2026-10-22-observatory-v1-s.jsonl` | `48bd772f9f7d9a110c4a8948b727654cd3464a01a76027f516aa203bbe03e11b` | `e9746b0979596003373d584abb273a921c476ff6f43d2c47d76284d81f370b25` | 2027 |
| `2026-10-22-observatory-v1-s.settings.json` | `42f6b820dc6c797cf408994df08f19dd5954a16610e923ed358b1f986f0113d9` | `42f6b820dc6c797cf408994df08f19dd5954a16610e923ed358b1f986f0113d9` | 453 |

#### M0 fixture (112 rows)

| Path | Original SHA-256 | Redacted SHA-256 | Lines |
|---|---|---|---|
| `2026-10-22-observatory-v1-m0.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.jsonl` | `39dba297aa1f16a1cf2f489af8e34e60f0c42b3a454280ec7e750209326a7cb8` | `cf9880e7b01c3217ea008058e12b9418c0f4721200bd86dd3008343d133f70b6` | 98 |
| `2026-10-22-observatory-v1-m0.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.settings.json` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | 50 |
| `2026-10-22-observatory-v1-m0.children/067dcdb4-b35f-4e4d-a021-16307c9b4531.jsonl` | `5f195492f1ed53e1b33492cc76d0b557f40bd14e272d38eca77007501cd33972` | `5029c10c2c7adf08a9ca4e7162deeac69e911508a3214878b0ae26917e977f30` | 117 |
| `2026-10-22-observatory-v1-m0.children/067dcdb4-b35f-4e4d-a021-16307c9b4531.settings.json` | `78f8a3ed7f796c85d12f5b1d64de0447fca646ee181340ea9ea0b54db9128554` | `78f8a3ed7f796c85d12f5b1d64de0447fca646ee181340ea9ea0b54db9128554` | 50 |
| `2026-10-22-observatory-v1-m0.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.jsonl` | `d417200900c2f363c07365935bb09bc17f8e6ca2678906287387438c5715abd2` | `e8be4dae52b1c8f0fe36990f6e6230da0a9344adf252f6ecb9d99402230c9bda` | 230 |
| `2026-10-22-observatory-v1-m0.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.settings.json` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | 50 |
| `2026-10-22-observatory-v1-m0.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.jsonl` | `857070cf8b56a8cf25fa56d55d6012038604dbb3e5a32862743f6d1f7e700eab` | `a71d1b4c44d373cc4ec90d24f8d23ea5498aa55c6d08493d52026e9ad1801613` | 85 |
| `2026-10-22-observatory-v1-m0.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.settings.json` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | 50 |
| `2026-10-22-observatory-v1-m0.children/12af01f5-f1d7-4487-8595-767298110266.jsonl` | `442a32a41d288dfaac184be04c778f606b3cd2a932b58cf48b2be410731a096d` | `2c494b81f10be38bd0dadb2698961d9b9086b5c633ac5eb69261d5894f2b9a3b` | 186 |
| `2026-10-22-observatory-v1-m0.children/12af01f5-f1d7-4487-8595-767298110266.settings.json` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | 63 |
| `2026-10-22-observatory-v1-m0.children/180b4a60-7d14-4c31-b27a-2a0b0a3980f9.jsonl` | `cf4665119bd168388d920f2ee554b2f864bcae5979d988cf66de83eed497e40d` | `9184f8942741790cfa6efcd99d0f851e82511bbdf5d8abed40cf380343324b78` | 163 |
| `2026-10-22-observatory-v1-m0.children/180b4a60-7d14-4c31-b27a-2a0b0a3980f9.settings.json` | `911df2f113d7d3ca55b3d2bce196effdedaf585f50613f4aa9db81b058297a16` | `911df2f113d7d3ca55b3d2bce196effdedaf585f50613f4aa9db81b058297a16` | 50 |
| `2026-10-22-observatory-v1-m0.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.jsonl` | `59907103396d712cd43c759c1ef3c9d0bed62e2e7db388e3afde9ac44811b21f` | `2e0d2c473092f5edf0d3853e26d7676e0abc5eff4906c2eb4065c6597f5fd4c1` | 74 |
| `2026-10-22-observatory-v1-m0.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.settings.json` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | 50 |
| `2026-10-22-observatory-v1-m0.children/2c04bd42-00fc-4c8b-a8bc-00ff515bac46.jsonl` | `1b9da13188087ce5de9fc399355c5e60e708e4f1df1dae68a188875e2467452c` | `6ede97e5472163ddd19a570ec6031a70d5f39acadb48b08734a1ff1b37691bd3` | 213 |
| `2026-10-22-observatory-v1-m0.children/2c04bd42-00fc-4c8b-a8bc-00ff515bac46.settings.json` | `48421b07f46c9e0fefb03b4d18a7c200ec6dd5333549d3f5129102ae2bed2aad` | `48421b07f46c9e0fefb03b4d18a7c200ec6dd5333549d3f5129102ae2bed2aad` | 51 |
| `2026-10-22-observatory-v1-m0.children/35e4968d-c2b6-445b-b932-64904f463a7a.jsonl` | `0df2b8d03d6567a4095b670be6c7c9531a9abe9c9ab1b31c68fd54947446f46f` | `6eb571e074408769c0a76a005acc49496e4f0afff7fa13ea3e33dccc0a3ac3ce` | 74 |
| `2026-10-22-observatory-v1-m0.children/35e4968d-c2b6-445b-b932-64904f463a7a.settings.json` | `2808c7ac0993437e031190c8342832fcb565ccc442bf200b282d2b2a80d2b658` | `2808c7ac0993437e031190c8342832fcb565ccc442bf200b282d2b2a80d2b658` | 50 |
| `2026-10-22-observatory-v1-m0.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.jsonl` | `3f15fde6bddd780eda75fa5f3fc89cb6b957ed13e010238546c7805f8bf95557` | `8ace9a93d44bf4d5320b14dc1b3e827e9ad70f3b9570bb6bc6f0524e64c0aa4b` | 115 |
| `2026-10-22-observatory-v1-m0.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.settings.json` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | 50 |
| `2026-10-22-observatory-v1-m0.children/36d06ed0-bebe-4318-a481-3876dbc9543b.jsonl` | `c9150b77cf493d4b7996085ed46947d94ce6123db56c381e7fcbdfe5feea0fae` | `721dae017593c10e011736908620796dc06a477848253e69f8f2d6f7d178fbf6` | 146 |
| `2026-10-22-observatory-v1-m0.children/36d06ed0-bebe-4318-a481-3876dbc9543b.settings.json` | `5710afe90336453786073c2adadddbd02c4b738250f4ac2afe180b00d800e1b2` | `5710afe90336453786073c2adadddbd02c4b738250f4ac2afe180b00d800e1b2` | 51 |
| `2026-10-22-observatory-v1-m0.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.jsonl` | `8be9dc51fb4062fea76c5db256a9537887f0bf50cb20007d498f65f4d538edbf` | `fc4783fb790c12518d32fd319a88a63486e151e1d4e168f24198b30acd59faf0` | 69 |
| `2026-10-22-observatory-v1-m0.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.settings.json` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | 50 |
| `2026-10-22-observatory-v1-m0.children/3e7bc50a-b8c7-4f15-b219-79594e5f83f7.jsonl` | `624b32b4420fb28b480849bd7de018ee181bbd3a96be283c2cfb021917b5f4ff` | `3ead8c722a31a3250318756845d3d466c90a2150b19542d31e277e2684679fbe` | 67 |
| `2026-10-22-observatory-v1-m0.children/3e7bc50a-b8c7-4f15-b219-79594e5f83f7.settings.json` | `64a3dc0d990704405deac3af2c9dfcfc22569df08de805e307ef1a3a4f780cb4` | `64a3dc0d990704405deac3af2c9dfcfc22569df08de805e307ef1a3a4f780cb4` | 50 |
| `2026-10-22-observatory-v1-m0.children/4162070f-576b-49e0-b1e4-7ed0d84027b2.jsonl` | `23cda1cf2e66b7324db44d0c148252814f7dd60e6601e2d36aeb25f534fc4e7d` | `d24bfccaca1f3989068b3a12160059b8a51d6a74934db770289e736d6a6fe1d3` | 65 |
| `2026-10-22-observatory-v1-m0.children/4162070f-576b-49e0-b1e4-7ed0d84027b2.settings.json` | `6751a4950db6c7df734efa9b6a6d194a14e0bd9e0d9d8f3bf87fd722e0384b23` | `6751a4950db6c7df734efa9b6a6d194a14e0bd9e0d9d8f3bf87fd722e0384b23` | 50 |
| `2026-10-22-observatory-v1-m0.children/44e19307-7533-41a1-8f66-1234a1386401.jsonl` | `e06ec8de1d850b2459ad749d58a90b6e167ff16a95f1c9cfe7e6fdd3ebfd9984` | `13aec7db88c925b1d8b9cb584892e517d3dd59f9bd9c4b0513b5947fa7f0e738` | 83 |
| `2026-10-22-observatory-v1-m0.children/44e19307-7533-41a1-8f66-1234a1386401.settings.json` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | 50 |
| `2026-10-22-observatory-v1-m0.children/49847d4f-b024-4858-94cf-ff0e92e2e439.jsonl` | `1307c2ce462bf3f73c57a3da4d0f078b309010cefb4cdf6f0ea0cb0063ed636a` | `69376a279ec41a1d34accc086ceb18b2a0f6d25cdcd062c6a9683b026132ac4a` | 83 |
| `2026-10-22-observatory-v1-m0.children/49847d4f-b024-4858-94cf-ff0e92e2e439.settings.json` | `fa51232661782cf6a75a774424716b0798923d0e3bb20ee619068bd4be199474` | `fa51232661782cf6a75a774424716b0798923d0e3bb20ee619068bd4be199474` | 50 |
| `2026-10-22-observatory-v1-m0.children/4dd52b9e-97b7-4e01-9020-a956fd5d1949.jsonl` | `6cc3f601c04b45f12c210b0002270b90779060fa6cabc576cf42eb11eee71e1c` | `ad17bb75cce0241f9760856945e63c749c57ded3d139214e56980c8c895578a4` | 84 |
| `2026-10-22-observatory-v1-m0.children/4dd52b9e-97b7-4e01-9020-a956fd5d1949.settings.json` | `9956fdaae11836ca9b07c11880ecc93da04b064cd5f16aba7e73d03b56ee90dd` | `9956fdaae11836ca9b07c11880ecc93da04b064cd5f16aba7e73d03b56ee90dd` | 51 |
| `2026-10-22-observatory-v1-m0.children/571a9e79-c07e-43f5-899f-0e4f188a6a61.jsonl` | `bb4d11243fa8dd0c0456e9f9b3c62c8c46f2532bd205c3e6e8c0f7d2e7c35591` | `93a6b8789e27169f0cdcb5f8542bc68d64aa6c12392b906a869d89d37473ddd0` | 81 |
| `2026-10-22-observatory-v1-m0.children/571a9e79-c07e-43f5-899f-0e4f188a6a61.settings.json` | `e660b6668f0d77228320266f7516bbf88287b6b371fe38a6df071c45be275dc4` | `e660b6668f0d77228320266f7516bbf88287b6b371fe38a6df071c45be275dc4` | 50 |
| `2026-10-22-observatory-v1-m0.children/57905379-0ef4-4ec7-aabc-48c7082dcff4.jsonl` | `1663163fb94ebf2521064ea9265e8e6a9073e6600e9955512f6cf7c60ba68dc6` | `59e46ef01c6fd6cca2ef3f87d88126f8b294826c260c49a25fe7d68349fc49b4` | 71 |
| `2026-10-22-observatory-v1-m0.children/57905379-0ef4-4ec7-aabc-48c7082dcff4.settings.json` | `16788353e604dbb9f46bc1758bfe09cc74616f617abe2646ad6accda2c28615c` | `16788353e604dbb9f46bc1758bfe09cc74616f617abe2646ad6accda2c28615c` | 51 |
| `2026-10-22-observatory-v1-m0.children/59ee8736-e49e-4091-9ade-536696563973.jsonl` | `e054989587d78e42b19e6c0326d7ee99eff74e9dfe094b44f4585470a9803ebd` | `9f81ccf4630dee6b1696569d3ddc28560def2f95f8236fba436d8cc2a8d82b41` | 38 |
| `2026-10-22-observatory-v1-m0.children/59ee8736-e49e-4091-9ade-536696563973.settings.json` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | 118 |
| `2026-10-22-observatory-v1-m0.children/5f9d68a5-c673-485f-823e-d7f69638c070.jsonl` | `ca13b32ce2af48f401ad0529fa3f1960f8ce84a32d4e8d3ea46124e7be670e9d` | `86cccc9841ad40c756f56a3d71fae380a0f0d4f60d4d0b328538464b44f0896f` | 75 |
| `2026-10-22-observatory-v1-m0.children/5f9d68a5-c673-485f-823e-d7f69638c070.settings.json` | `b24d81a4b80a573a0beacfeaf462f0c0d299c5c7300bc70bc80bb7386278d153` | `b24d81a4b80a573a0beacfeaf462f0c0d299c5c7300bc70bc80bb7386278d153` | 50 |
| `2026-10-22-observatory-v1-m0.children/676e401d-cf77-4397-be4e-cb5aca5e669b.jsonl` | `5c4c68826f8ec6c5d6c9df9f9be5f96e173601ee281e010d7a3aeb99a6297e9e` | `143011767323967f819652729b5adf421628b701eb4c969466735d2d32004d6d` | 78 |
| `2026-10-22-observatory-v1-m0.children/676e401d-cf77-4397-be4e-cb5aca5e669b.settings.json` | `7a04f4b72685bc534f5a91019419a983624bf7c644341f6fc888a880c2235db7` | `7a04f4b72685bc534f5a91019419a983624bf7c644341f6fc888a880c2235db7` | 50 |
| `2026-10-22-observatory-v1-m0.children/6d6f7721-055e-4571-83e9-23ec160693fe.jsonl` | `867f421dba3b236de45a77f6204d54bbaa3968020d59d2322023a3792c5076c1` | `a5e72ea097016409ee8fa4b2ab5b321779cda24b9e46f9603fb4df9d3fdcb252` | 25 |
| `2026-10-22-observatory-v1-m0.children/6d6f7721-055e-4571-83e9-23ec160693fe.settings.json` | `cf43f6fad0e66e1e732290d3a44817f19381d504240dd624c5cb9b1dfd66750d` | `cf43f6fad0e66e1e732290d3a44817f19381d504240dd624c5cb9b1dfd66750d` | 50 |
| `2026-10-22-observatory-v1-m0.children/6eb88099-278d-4ddb-9f5d-23f1255cdbc5.jsonl` | `cbc66b2382b68594a06d31814ba916bd6cff3ca0be4c8e41b2b8d79b1e82a33f` | `ae956f0e63e0ff77f1a359c1f2dc164ddbc81c8650e26d3cdc76f7e506a9bba0` | 131 |
| `2026-10-22-observatory-v1-m0.children/6eb88099-278d-4ddb-9f5d-23f1255cdbc5.settings.json` | `4d16178eeeb6687d0ddbe19edb118eb4b78c86a45670a14d474e2b1a6ef82915` | `4d16178eeeb6687d0ddbe19edb118eb4b78c86a45670a14d474e2b1a6ef82915` | 100 |
| `2026-10-22-observatory-v1-m0.children/72e5184a-e450-432c-b01c-9c3323bc7617.jsonl` | `f2810d51d890db5f53a5720de8ea2e4019a16580235e4e78f52d78d3a182993d` | `45e4db3608a13fdfc2820a95198e95e75b436caf1e769f62fb80a2e5d9aa773f` | 103 |
| `2026-10-22-observatory-v1-m0.children/72e5184a-e450-432c-b01c-9c3323bc7617.settings.json` | `1372ed2f44982c41d1ddffe7dc4dc9460b84c325ce80d5f04dbcde3795e91976` | `1372ed2f44982c41d1ddffe7dc4dc9460b84c325ce80d5f04dbcde3795e91976` | 51 |
| `2026-10-22-observatory-v1-m0.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.jsonl` | `d809ea5098108a57aafbc6f5febe6c91c01080437fecd0d14c4cf45cd8d387f9` | `af0a9a16db5f40655a3f63be65458daafe617f56283ac03de2fcf8456bd7b5eb` | 83 |
| `2026-10-22-observatory-v1-m0.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.settings.json` | `4bb9d21d5c6e80553211c3ae6c6a2f374b68f569cdd45cdbbd4d2c7488fd9ea2` | `4bb9d21d5c6e80553211c3ae6c6a2f374b68f569cdd45cdbbd4d2c7488fd9ea2` | 51 |
| `2026-10-22-observatory-v1-m0.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.jsonl` | `0db91bb11624bb9ba1ed998d69352c1fa2471919b62f5f7fae70f7eea264e55b` | `c55c72cf496ef8e8623354fe197f62082fadf0caf1384741f210017701512af1` | 70 |
| `2026-10-22-observatory-v1-m0.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.settings.json` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | 50 |
| `2026-10-22-observatory-v1-m0.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.jsonl` | `c8300fb51464864dea74dc65591f6cfbdb085083a1d611df207c7a0435cfa6a8` | `fb21999e80d153c18154f22e6e6ca85b098beb319efb425dfdabd78ba81f4de9` | 61 |
| `2026-10-22-observatory-v1-m0.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.settings.json` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | 50 |
| `2026-10-22-observatory-v1-m0.children/82070292-f272-4110-8a74-ef2b31236e67.jsonl` | `f20d80e78a327c378a04ba6e1641fa5b20a309d86ab6189fc5ec6892be9069f2` | `db74cbd5c410175640114e24a4be3a9270854d0037545f36152d7e4583f1bbda` | 71 |
| `2026-10-22-observatory-v1-m0.children/82070292-f272-4110-8a74-ef2b31236e67.settings.json` | `0b4643cc8b3e563ec0e9529464c442d621c5a512b2b2a5aca15322305a186e76` | `0b4643cc8b3e563ec0e9529464c442d621c5a512b2b2a5aca15322305a186e76` | 118 |
| `2026-10-22-observatory-v1-m0.children/82a61cf6-5508-4b01-921a-c275a720e0c8.jsonl` | `10df300e4c6af467a8f5538d27fb6af8ac37099791bdb304ede5aa4ee83ab853` | `0f84ef6078f942ac1319465b59df4c61bb5b84b7b68aec3e63a7c05ce717f7e6` | 84 |
| `2026-10-22-observatory-v1-m0.children/82a61cf6-5508-4b01-921a-c275a720e0c8.settings.json` | `86881b2bd010372e6008f98a665b2c8ba68e78d3ed65d98ac378a44ec9967a63` | `86881b2bd010372e6008f98a665b2c8ba68e78d3ed65d98ac378a44ec9967a63` | 51 |
| `2026-10-22-observatory-v1-m0.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.jsonl` | `1e1308e9091ac36a71fb5e56358ae750e167e6f155cbf78577ff5f3e76008bd5` | `4eae8e01e6aaf2023e1242e06a604132c5a0db2b06ee1057db66473499185e1d` | 221 |
| `2026-10-22-observatory-v1-m0.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.settings.json` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | 50 |
| `2026-10-22-observatory-v1-m0.children/8ca62365-1d34-4e94-9473-df95d2229445.jsonl` | `84219418b5701591097fd7dc88c192c173e4f4e78bd393c6ec0426f3e9b6d4c7` | `35437c4c4abf6e0689ecd6144c78b9ad52c53b9654e4b6224cf02db802e05e40` | 51 |
| `2026-10-22-observatory-v1-m0.children/8ca62365-1d34-4e94-9473-df95d2229445.settings.json` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | 64 |
| `2026-10-22-observatory-v1-m0.children/94334a8c-8a4d-4784-99a5-3253d732f9bb.jsonl` | `a4534d9f5e9a700de82d6501e25c77f90b8cbaa4267a2052f9aa574675925cac` | `ee1c1822bf2bfc3fbd2c41b4b7f473b6fd3eb3014fa3771caf07debebf18d60c` | 88 |
| `2026-10-22-observatory-v1-m0.children/94334a8c-8a4d-4784-99a5-3253d732f9bb.settings.json` | `b560f6270493726aa1d5624a65cd45461b068e77b8993952232fd466cd0d420e` | `b560f6270493726aa1d5624a65cd45461b068e77b8993952232fd466cd0d420e` | 68 |
| `2026-10-22-observatory-v1-m0.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.jsonl` | `a75fa68900cd011b9d17d192f24d244b9f5c9bb3fedc7e761f939c1de5296959` | `0ab006b07d42e1efd8fdfb8e9b9c32d424faa0434d71b8c6cbda0f6e9b765f1e` | 132 |
| `2026-10-22-observatory-v1-m0.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.settings.json` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | 50 |
| `2026-10-22-observatory-v1-m0.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.jsonl` | `28a2aa75beb165ee4e9e7f2312a0aae22b0e38e9583b8d7ab0003b576c3bde79` | `25f5a79840151cb21fcfeacda5b2c49dc2120d5180cf55d47f19830e01ee7b2b` | 126 |
| `2026-10-22-observatory-v1-m0.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.settings.json` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | 50 |
| `2026-10-22-observatory-v1-m0.children/acd7d48a-19d6-43f9-82e1-a606c2e917ff.jsonl` | `626e5c56c77845207531b97ce00250243929fb0aaed49a864b2d3b617181bf18` | `4298cfe3a32c269657d849bbbeaf2dc6e7cf7917065d8a26291cb194203d86b8` | 100 |
| `2026-10-22-observatory-v1-m0.children/acd7d48a-19d6-43f9-82e1-a606c2e917ff.settings.json` | `fe64effae9bef53788cda244f8adfc12b2ab5c6f969eea6a761d188fbe9db208` | `fe64effae9bef53788cda244f8adfc12b2ab5c6f969eea6a761d188fbe9db208` | 50 |
| `2026-10-22-observatory-v1-m0.children/b19ddede-7907-444a-a16d-28ac6d1c122e.jsonl` | `a1cb9b2d737b45013a7196e1a9cc1af08a581e86d980de65e1e3d6323baa549f` | `cc1db466c3ad6f8ffad079bf037035ba46f00b723da76716fb70cf3ba14a6162` | 72 |
| `2026-10-22-observatory-v1-m0.children/b19ddede-7907-444a-a16d-28ac6d1c122e.settings.json` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | 50 |
| `2026-10-22-observatory-v1-m0.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.jsonl` | `d4bb130104fa844c2f4550627142fed9765f6f0ea3f25d11777538f696f91477` | `4aee0cf6b7fdefd86f3c7f6cf7e72e57df8bb6365f7d93014707c3fb27e46b9e` | 45 |
| `2026-10-22-observatory-v1-m0.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.settings.json` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | 50 |
| `2026-10-22-observatory-v1-m0.children/b8c3d386-030e-4602-9781-586b1e928883.jsonl` | `40e998f8105dd36fee58648b4c943086080737b05aa4a6214aa448d0ac151833` | `6160a4b1aa33124a97f12f092107d62800dcd0ea155ad560d92c589c1bdf036a` | 113 |
| `2026-10-22-observatory-v1-m0.children/b8c3d386-030e-4602-9781-586b1e928883.settings.json` | `7e99a73733c28dbd1c6c5c23fd39186ed2cefed5941b808358aa84adeff7af43` | `7e99a73733c28dbd1c6c5c23fd39186ed2cefed5941b808358aa84adeff7af43` | 51 |
| `2026-10-22-observatory-v1-m0.children/bfc3f3f7-9075-4ae5-ac74-1bd0a840baf9.jsonl` | `a66df011d91c87d22d5eae17c4f0e3f27c1624da1b47eaef923d04b3d0377018` | `979dfd05798d359132f14e11561b3b2d094fb01e85b5e1ea5bf56f050fef9b3f` | 91 |
| `2026-10-22-observatory-v1-m0.children/bfc3f3f7-9075-4ae5-ac74-1bd0a840baf9.settings.json` | `ab4df8070b82f6f345778cd50d2b03f434850466fbf75c468da4b68cadf7773d` | `ab4df8070b82f6f345778cd50d2b03f434850466fbf75c468da4b68cadf7773d` | 51 |
| `2026-10-22-observatory-v1-m0.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.jsonl` | `8ff2af85091d5ac5b3a270e9711d374ca9125cb83ac80e481fe824979adb09fa` | `0c95885f7a056d3181860e556a1695bf1d04b862b30f0d419c7cc81f46cc6ae6` | 93 |
| `2026-10-22-observatory-v1-m0.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.settings.json` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | 50 |
| `2026-10-22-observatory-v1-m0.children/c9f10788-827c-4595-a3d6-65a205b096bf.jsonl` | `9e4d1ea6a2f700df401c44eb431455d4b2d50d69d55c8064e24d3ec55bf0636c` | `6460bae42af2bf5e92b93c14671da2b1c30e92d1b124ce3ad88b42e5d79fbaea` | 88 |
| `2026-10-22-observatory-v1-m0.children/c9f10788-827c-4595-a3d6-65a205b096bf.settings.json` | `a6d5d917dbb20aba1d625e2dd2d2103d08773084fee915ef36a442d3846b64a7` | `a6d5d917dbb20aba1d625e2dd2d2103d08773084fee915ef36a442d3846b64a7` | 50 |
| `2026-10-22-observatory-v1-m0.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.jsonl` | `75638ad233cf39a733d6245e9f2c34d701cf9505fdeb7adc674d43a80af4bce0` | `3b19c53aafcece1b07a71eb2da8c9105cead5d6b88f6f6d4232d935b3ed20dd5` | 144 |
| `2026-10-22-observatory-v1-m0.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.settings.json` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | 51 |
| `2026-10-22-observatory-v1-m0.children/cb3408d4-9cad-434f-b3dd-d24ef1e5d1d6.jsonl` | `aa6af47c36089b110261f2fc4a26efad3bc1a7e486d746eda184d294c7aa3d1c` | `400d508708e9a7b75241c54b4d63aed341ecb5efd1592c041a2725708597f67f` | 91 |
| `2026-10-22-observatory-v1-m0.children/cb3408d4-9cad-434f-b3dd-d24ef1e5d1d6.settings.json` | `6d2d6c75a82c071e65a654b51567ba8d5f414af34157e1a2ff197c6495c7ac50` | `6d2d6c75a82c071e65a654b51567ba8d5f414af34157e1a2ff197c6495c7ac50` | 51 |
| `2026-10-22-observatory-v1-m0.children/cedd249c-c940-4d09-be06-bee90f98f76d.jsonl` | `079f683524cc8a864909ed1cb5bd771f5cea45130fe4cf31fbc5e04cb93e8e00` | `0b94d6962b3376a4b66ec930fde9435ebb59765584a71e097061f6d12a20bf5f` | 455 |
| `2026-10-22-observatory-v1-m0.children/cedd249c-c940-4d09-be06-bee90f98f76d.settings.json` | `2aef5e605db729b7b55423768da9dc01a426c93b776bb8faf96075f2604de578` | `2aef5e605db729b7b55423768da9dc01a426c93b776bb8faf96075f2604de578` | 50 |
| `2026-10-22-observatory-v1-m0.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.jsonl` | `f1b9c035808c67f31d5d21af1fe30090755dd9cb91809e9ee7241b482eb3569b` | `d742f57a74d3d21a73fccf1432bea1df0125a237266b60cf08e84d1192cfc617` | 80 |
| `2026-10-22-observatory-v1-m0.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.settings.json` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | 50 |
| `2026-10-22-observatory-v1-m0.children/cf2e2456-b1e6-48e5-9a20-dea6ff22ffbe.jsonl` | `86c2b7d65acb469d891553ea70281772a025f77e30bd162662bed14526f1e722` | `e54ace5f70a0663cfb997f9b0c8d0676438ee8e0347917fd6ae4497d11a19f4b` | 119 |
| `2026-10-22-observatory-v1-m0.children/cf2e2456-b1e6-48e5-9a20-dea6ff22ffbe.settings.json` | `1fe756b87bb3d3ba6e9b4922aa9fe2626df3afdd1ef5f8d8ae3f1f168bcc2b7a` | `1fe756b87bb3d3ba6e9b4922aa9fe2626df3afdd1ef5f8d8ae3f1f168bcc2b7a` | 50 |
| `2026-10-22-observatory-v1-m0.children/d0304e2e-652e-443e-bd19-4afd804a90ed.jsonl` | `4adbb32cffff98ecc61d5cac7b184ed659817c4116d9830450f813cbe34ce3fd` | `df88809d6cfb9735311c07e1c04ba3009f321c1312f1f099b1f02687b79f9ab1` | 107 |
| `2026-10-22-observatory-v1-m0.children/d0304e2e-652e-443e-bd19-4afd804a90ed.settings.json` | `167fe105703ca3e8761d88ae390f4ce2b93cf5ce63fd23ac92cfea29ee774866` | `167fe105703ca3e8761d88ae390f4ce2b93cf5ce63fd23ac92cfea29ee774866` | 84 |
| `2026-10-22-observatory-v1-m0.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.jsonl` | `05cdfa7a8b2907aa9befa42cfeae61c2f9dc44e7c79d455a573053c94bc20bfc` | `208e1919a566c253d50463983e3cb3927e479fead5d01abb80c9ee800a7d4a63` | 116 |
| `2026-10-22-observatory-v1-m0.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.settings.json` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | 50 |
| `2026-10-22-observatory-v1-m0.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.jsonl` | `505347611712c111eb65b7a3939c7f2c26da874fcc7b9fa07ffe03f4cbb83d61` | `8c40df04e87a7769462f5189f0bebca9e4d783666ba862e0b1a2eef8af4cc749` | 231 |
| `2026-10-22-observatory-v1-m0.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.settings.json` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | 50 |
| `2026-10-22-observatory-v1-m0.children/e2e3d113-d82b-4548-8809-4ebd269d1629.jsonl` | `ae46cfd55454332e662c7822b9cc70889335353f38fc24a5873a2765e8e62a48` | `689470e7c266ddc0a70483daf1526bf7235f9eb70038f99800efb361fbfb1990` | 70 |
| `2026-10-22-observatory-v1-m0.children/e2e3d113-d82b-4548-8809-4ebd269d1629.settings.json` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | 118 |
| `2026-10-22-observatory-v1-m0.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.jsonl` | `9e9da826f322d68532a7513e668d47619d61da71e0281a84f0401973eb7aad7b` | `2cb9edd54574e9b64e2b7d04ee097a9bb7fea8b8beaa6ce7b937725648895bdb` | 87 |
| `2026-10-22-observatory-v1-m0.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.settings.json` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | 50 |
| `2026-10-22-observatory-v1-m0.children/e78e6d4f-35c4-4d93-b256-06af5323bd45.jsonl` | `1b2839f1d795225e27fc84c8afbb6e8f0a6c5ef00271f38bf96bec1f9317672a` | `77af0ff69e1ae293d50ece3e7df515f0d4780b78b91b07200b9611efb776fe40` | 88 |
| `2026-10-22-observatory-v1-m0.children/e78e6d4f-35c4-4d93-b256-06af5323bd45.settings.json` | `3e57f2cb501d9210cfdc686eccde1ba19a733608bb0a6d569b7713132c28af58` | `3e57f2cb501d9210cfdc686eccde1ba19a733608bb0a6d569b7713132c28af58` | 50 |
| `2026-10-22-observatory-v1-m0.children/ee0c614a-d10e-4cbd-8ddc-c45826172336.jsonl` | `4b400efa80be85f7f81eb1924374b9af668c1c984d8ca353d511b910c1d52b74` | `869068145de8b43cfcd69b903d1fe9bd5da1bd86908f4850519fdcd326930294` | 105 |
| `2026-10-22-observatory-v1-m0.children/ee0c614a-d10e-4cbd-8ddc-c45826172336.settings.json` | `0958dac7f8bc6c6ddcbc4dd9babdf363bf199d74692527a0034a2a191e1d553b` | `0958dac7f8bc6c6ddcbc4dd9babdf363bf199d74692527a0034a2a191e1d553b` | 50 |
| `2026-10-22-observatory-v1-m0.children/fef523bb-b669-4ebc-aca2-b6b115cf4b9a.jsonl` | `38a0ba34c3c212f5339a014d875ce7d2af1d3c9384f6cbfcab9b6e961cd2abdb` | `ef2ff9a3d22c73bf79a3c9122659f3d8d02cf69f9445969f39d5f6b8f4981180` | 79 |
| `2026-10-22-observatory-v1-m0.children/fef523bb-b669-4ebc-aca2-b6b115cf4b9a.settings.json` | `b9035a822c569b3eda9254a67b8b6949ebc6e9049ec154431af6979e04f9b3c8` | `b9035a822c569b3eda9254a67b8b6949ebc6e9049ec154431af6979e04f9b3c8` | 51 |
| `2026-10-22-observatory-v1-m0.jsonl` | `134db160d53d7457d6076f9e6af3dba707ad3da2b50bddd62617a7705b93a830` | `ab23da5510478bb1c519adb78e1f47f959398477bfd802a9329bc71318a9a58c` | 2820 |
| `2026-10-22-observatory-v1-m0.settings.json` | `efef27eb7fe491e2cef90bd83634230e4b845a742d00141658c88dd6154076ec` | `efef27eb7fe491e2cef90bd83634230e4b845a742d00141658c88dd6154076ec` | 501 |

Checks run at the M0 integration: gitleaks 8.30.1 (`detect --no-git --no-banner --redact`) over the
redacted files reported no leaks (S 25.74 MB, M0 64.14 MB), recorded in each header `gitleaks` field.
Re-run at the M1 integration over `fixtures/streams` (both captures and the usage-log extract): `scanned ~68213177
bytes (68.21 MB) in 11.5s, no leaks found`, exit 0; the M0 header `gitleaks` field carries that line (the 64.14 MB
figure predates the re-redaction of 8 children at `a3d4e9f`; a later run measured 64.83 MB at `f66a9f5`).

### Consequences of the addendum

The S replay no longer ends with `unknown events: 2027` (see Consequences above): the mapper
knows the kinds since M0 (`D-023-mapping-v1.md`). Fixture hashes are frozen from the M0
integration; a change needs a decision file. The writer of the `redaction` block and the hash-table
generator were throwaway scripts at the M0 freeze; `BACKLOG.md` B-026 turns them into `scripts/`
with a test before the next capture.
