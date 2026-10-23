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
5. **Open point for the owner (M0 to M1 boundary list).** The hyphenated session-directory form
   (`-Users-dct-code-...`) remains in `.capture.json` `source_dir` and in file contents. The
   rule and the test cover the literal `/Users/dct` only. The owner decides whether the rule
   extends to the hyphenated form.

### S fixture hashes

Files outside the table, SHA-256 (full file bytes):

| File | Original (outside the repo) | Redacted (in the repo) |
|---|---|---|
| `2026-10-22-observatory-v1-s.capture.json` | `e9983eafa367cab187ff11ea1e30a192e6bed3beb98f9bfc35f96db5602e0ab1` | `c8ce39520641efbda16b53d05f63bc4f2eea9b0bfa7b85c3ed3c6ab73072ee31` |

The 52 rows below are `.redaction.files` of `fixtures/streams/2026-10-22-observatory-v1-s.capture.json` (every `.jsonl` and `.settings.json`: the
root, then `2026-10-22-observatory-v1-s.children/`). The capture file's own hash is above; it holds this table, so it
cannot be one of its rows. Paths are relative to `fixtures/streams/`; `lines` is `wc -l`.
A file with no match is byte-identical after redaction (equal hashes).

| Path | Original SHA-256 | Redacted SHA-256 | Lines |
|---|---|---|---|
| `2026-10-22-observatory-v1-s.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.jsonl` | `39dba297aa1f16a1cf2f489af8e34e60f0c42b3a454280ec7e750209326a7cb8` | `cab35de7f7fec268245e9de7a959f1899131c6c7a6df87b388ab3940af533e74` | 98 |
| `2026-10-22-observatory-v1-s.children/05820c7d-bd0e-4fe8-b869-cebb72727faa.settings.json` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | `8d9f90c7f497782cfc082129bd700ed280c9ecb5aee4d7de248bc4324ca0f599` | 50 |
| `2026-10-22-observatory-v1-s.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.jsonl` | `d417200900c2f363c07365935bb09bc17f8e6ca2678906287387438c5715abd2` | `e45f5d8a48b86a4f15d39d6de17d7304b3db3457928dd3971eb0db43ad88839a` | 230 |
| `2026-10-22-observatory-v1-s.children/0b1286d9-4bf0-41ff-a14a-67ca7852ddc4.settings.json` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | `7db36ba849aabd2fccb951eab574a3dc9bd25cc97319b22ecf0831f275364d83` | 50 |
| `2026-10-22-observatory-v1-s.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.jsonl` | `857070cf8b56a8cf25fa56d55d6012038604dbb3e5a32862743f6d1f7e700eab` | `57d7db1ffddded17857c35608791830e933d624d13207355c559e359a964e6dd` | 85 |
| `2026-10-22-observatory-v1-s.children/101b9e5d-3e26-424c-bb8d-6c5ad1d1e356.settings.json` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | `fdd11da3105a597435557038dda85f40201cd6bf4e66725aba676e922b146e6a` | 50 |
| `2026-10-22-observatory-v1-s.children/12af01f5-f1d7-4487-8595-767298110266.jsonl` | `442a32a41d288dfaac184be04c778f606b3cd2a932b58cf48b2be410731a096d` | `2c494b81f10be38bd0dadb2698961d9b9086b5c633ac5eb69261d5894f2b9a3b` | 186 |
| `2026-10-22-observatory-v1-s.children/12af01f5-f1d7-4487-8595-767298110266.settings.json` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | `a5674c4df5846fa08c035ad21a2e5d411c5ccd1a77a5a6979cbb3d64b2e18397` | 63 |
| `2026-10-22-observatory-v1-s.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.jsonl` | `59907103396d712cd43c759c1ef3c9d0bed62e2e7db388e3afde9ac44811b21f` | `9fb6d5fcbbec73a7eeca08a7bcd72d5ff2cc8ccb072a87258c1b1f57f3499d2b` | 74 |
| `2026-10-22-observatory-v1-s.children/2abf7f4f-100f-4c9c-b12d-78675596ee0f.settings.json` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | `474b1415d99bf56e34aac424455ecd30984a41f0f4f860487c053bc978030eed` | 50 |
| `2026-10-22-observatory-v1-s.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.jsonl` | `3f15fde6bddd780eda75fa5f3fc89cb6b957ed13e010238546c7805f8bf95557` | `2fa4f8d0fae222f46f295eae37656015d8ea378656662b7e0b5ae919c0eab462` | 115 |
| `2026-10-22-observatory-v1-s.children/35fca0ab-5f7e-4924-9d1a-8bf250e785af.settings.json` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | `3a2ae34b9a124c49e6fcdfaaa855def9522c639e0047a04af6032259edaa8cbc` | 50 |
| `2026-10-22-observatory-v1-s.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.jsonl` | `8be9dc51fb4062fea76c5db256a9537887f0bf50cb20007d498f65f4d538edbf` | `fc4783fb790c12518d32fd319a88a63486e151e1d4e168f24198b30acd59faf0` | 69 |
| `2026-10-22-observatory-v1-s.children/3d385317-7e9b-45de-b116-6ebc278d6ec0.settings.json` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | `f2ac57cf11a39c8b9b5b48ad73671b84d03ea6b95d87a738e969366ecafcbe1f` | 50 |
| `2026-10-22-observatory-v1-s.children/44e19307-7533-41a1-8f66-1234a1386401.jsonl` | `e06ec8de1d850b2459ad749d58a90b6e167ff16a95f1c9cfe7e6fdd3ebfd9984` | `b57a5054950f4191e6378832f317ded476a3d1b27a2157b08e804188e2336550` | 83 |
| `2026-10-22-observatory-v1-s.children/44e19307-7533-41a1-8f66-1234a1386401.settings.json` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | `96a83f374e10a5f5d078c48c168d3b5653d2613301dc01bc73ad43fcfd91e234` | 50 |
| `2026-10-22-observatory-v1-s.children/59ee8736-e49e-4091-9ade-536696563973.jsonl` | `e054989587d78e42b19e6c0326d7ee99eff74e9dfe094b44f4585470a9803ebd` | `9f81ccf4630dee6b1696569d3ddc28560def2f95f8236fba436d8cc2a8d82b41` | 38 |
| `2026-10-22-observatory-v1-s.children/59ee8736-e49e-4091-9ade-536696563973.settings.json` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | `7f98eb58d3e2b47108592910e27af002fc298d00c7ccfe33a0cf0c5c946b271b` | 118 |
| `2026-10-22-observatory-v1-s.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.jsonl` | `aad0b0ecfa1723c7bfa0950eb5e96fb8dea248b5c9005d8e07e9b833fce20cb2` | `c139e95c59de70b956d7d52643339f3a8d1a305c4faee1ac14761684696bd58d` | 32 |
| `2026-10-22-observatory-v1-s.children/73cd8242-3b2c-46ef-b15c-3d2cb7088061.settings.json` | `09e8eb863245debe0d06f06e4e9fe8fb3bf6c71f6fb2c913ae6ae3f1a1c171a3` | `09e8eb863245debe0d06f06e4e9fe8fb3bf6c71f6fb2c913ae6ae3f1a1c171a3` | 51 |
| `2026-10-22-observatory-v1-s.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.jsonl` | `0db91bb11624bb9ba1ed998d69352c1fa2471919b62f5f7fae70f7eea264e55b` | `c55c72cf496ef8e8623354fe197f62082fadf0caf1384741f210017701512af1` | 70 |
| `2026-10-22-observatory-v1-s.children/777c6203-0e4b-45db-8064-21ba6f1e9a51.settings.json` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | `81d190bd2d85bcf36732d7ff88ae462881772261a8eedfe8428d7eeab35d3f38` | 50 |
| `2026-10-22-observatory-v1-s.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.jsonl` | `c8300fb51464864dea74dc65591f6cfbdb085083a1d611df207c7a0435cfa6a8` | `d2ce34bb52cf26cfdd978a7fed98e11ddb190eb6564762d941e990efaaf83b5e` | 61 |
| `2026-10-22-observatory-v1-s.children/7be541b9-cee5-4643-b588-ae6c1f31c74c.settings.json` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | `5dafa3a99c55d823789057c1c22965b4946ab1690187fe3615d3eaa83118eca1` | 50 |
| `2026-10-22-observatory-v1-s.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.jsonl` | `1e1308e9091ac36a71fb5e56358ae750e167e6f155cbf78577ff5f3e76008bd5` | `53ee836d3aee296c9eec89e6bdb00d6849f3e34820dd2e0503c58cdb6b65e1cd` | 221 |
| `2026-10-22-observatory-v1-s.children/82ad1581-fe75-4436-ac9c-9bc1e20a789b.settings.json` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | `f32c229456440bb8df960b71c724d56d2d4f705607859388216355d7c2bdd458` | 50 |
| `2026-10-22-observatory-v1-s.children/8ca62365-1d34-4e94-9473-df95d2229445.jsonl` | `84219418b5701591097fd7dc88c192c173e4f4e78bd393c6ec0426f3e9b6d4c7` | `35437c4c4abf6e0689ecd6144c78b9ad52c53b9654e4b6224cf02db802e05e40` | 51 |
| `2026-10-22-observatory-v1-s.children/8ca62365-1d34-4e94-9473-df95d2229445.settings.json` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | `31e2092f42a25c0dfcf13ae4c40dc01ada3ebf673949b31ec9a0981b27b0eaf7` | 64 |
| `2026-10-22-observatory-v1-s.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.jsonl` | `a75fa68900cd011b9d17d192f24d244b9f5c9bb3fedc7e761f939c1de5296959` | `0ab006b07d42e1efd8fdfb8e9b9c32d424faa0434d71b8c6cbda0f6e9b765f1e` | 132 |
| `2026-10-22-observatory-v1-s.children/9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d.settings.json` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | `9b4e94aee7758bd5150bb3e8d5168a8a5e59152ff7f41925def55f1df7d8c175` | 50 |
| `2026-10-22-observatory-v1-s.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.jsonl` | `28a2aa75beb165ee4e9e7f2312a0aae22b0e38e9583b8d7ab0003b576c3bde79` | `dfbf63e768cfb73d24a10e03cf812cb55915bcea701c6d09194c66e4d301e43e` | 126 |
| `2026-10-22-observatory-v1-s.children/9d874ce8-7d72-4d9b-8c98-5f176e7ebd30.settings.json` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | `773607ece63ecfee70bfc4e7c224a80266f7e020b28439f56db9131c7f13c128` | 50 |
| `2026-10-22-observatory-v1-s.children/b19ddede-7907-444a-a16d-28ac6d1c122e.jsonl` | `a1cb9b2d737b45013a7196e1a9cc1af08a581e86d980de65e1e3d6323baa549f` | `cc1db466c3ad6f8ffad079bf037035ba46f00b723da76716fb70cf3ba14a6162` | 72 |
| `2026-10-22-observatory-v1-s.children/b19ddede-7907-444a-a16d-28ac6d1c122e.settings.json` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | `29c4bd36a22725a3d06f7d767cbe6de534440b6b493b9f477ebec336fe436a3d` | 50 |
| `2026-10-22-observatory-v1-s.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.jsonl` | `d4bb130104fa844c2f4550627142fed9765f6f0ea3f25d11777538f696f91477` | `4aee0cf6b7fdefd86f3c7f6cf7e72e57df8bb6365f7d93014707c3fb27e46b9e` | 45 |
| `2026-10-22-observatory-v1-s.children/b82fb5f0-0e21-47d5-b3ae-fd37b86e9116.settings.json` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | `90e399b0deb8b9d30a130b099daac844fdbdca4c5ab6a889514bd398b2d3966e` | 50 |
| `2026-10-22-observatory-v1-s.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.jsonl` | `8ff2af85091d5ac5b3a270e9711d374ca9125cb83ac80e481fe824979adb09fa` | `461e482a8c925001d247634e09ee62c0dd397d59c212036f62fc3722e49dbd79` | 93 |
| `2026-10-22-observatory-v1-s.children/c911e77f-129c-45d7-bcb4-6a0136a28cb6.settings.json` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | `aaa0eecbb1418f7a87a615df1be119abf50febfc741b9471b6f54e1a937fac0c` | 50 |
| `2026-10-22-observatory-v1-s.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.jsonl` | `75638ad233cf39a733d6245e9f2c34d701cf9505fdeb7adc674d43a80af4bce0` | `9e1f9c20d85cc7996d11bb4ddedd1d3195d72ebf78568a7b24529215d59fb49e` | 144 |
| `2026-10-22-observatory-v1-s.children/ca58108e-3da3-4ed6-bf3e-0a7073d5833b.settings.json` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | `8fcd13620f7a4b98780da727bfa02ab15967ce893793e720a5a407767ef29cbc` | 51 |
| `2026-10-22-observatory-v1-s.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.jsonl` | `f1b9c035808c67f31d5d21af1fe30090755dd9cb91809e9ee7241b482eb3569b` | `d742f57a74d3d21a73fccf1432bea1df0125a237266b60cf08e84d1192cfc617` | 80 |
| `2026-10-22-observatory-v1-s.children/cef762e9-5544-40a6-a9fa-0b9b3593f743.settings.json` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | `506fbd93080b30e23b4b111f63c683eb8bc4c3c19ab31f044b9242fefba7666e` | 50 |
| `2026-10-22-observatory-v1-s.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.jsonl` | `05cdfa7a8b2907aa9befa42cfeae61c2f9dc44e7c79d455a573053c94bc20bfc` | `208e1919a566c253d50463983e3cb3927e479fead5d01abb80c9ee800a7d4a63` | 116 |
| `2026-10-22-observatory-v1-s.children/d1083556-f835-4f44-a0cf-c907eb18e1ab.settings.json` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | `899549e28b2b0546bda151c01cb1239179533efc4a71e123ca13e062e7ba3c50` | 50 |
| `2026-10-22-observatory-v1-s.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.jsonl` | `505347611712c111eb65b7a3939c7f2c26da874fcc7b9fa07ffe03f4cbb83d61` | `430d3db5d4fae535003372aa4747f41c523f13949c6dfdffbe5a775fcdbdc182` | 231 |
| `2026-10-22-observatory-v1-s.children/d7960fc7-c0a0-4f02-b8cf-408b7a8d3b92.settings.json` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | `0aad7db0b1507e93697cbb330e30f5b69fa5703a7f793e7448344fa032283fcb` | 50 |
| `2026-10-22-observatory-v1-s.children/e2e3d113-d82b-4548-8809-4ebd269d1629.jsonl` | `ae46cfd55454332e662c7822b9cc70889335353f38fc24a5873a2765e8e62a48` | `689470e7c266ddc0a70483daf1526bf7235f9eb70038f99800efb361fbfb1990` | 70 |
| `2026-10-22-observatory-v1-s.children/e2e3d113-d82b-4548-8809-4ebd269d1629.settings.json` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | `e41abf17e322b0fc613e38f7cb3ffb04173a2169295c4d45a5f25beddfd63420` | 118 |
| `2026-10-22-observatory-v1-s.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.jsonl` | `9e9da826f322d68532a7513e668d47619d61da71e0281a84f0401973eb7aad7b` | `4337fbd1c53fe4d804eae383b3b2d52db75808d2c9616814c9d372a2ddd0e7fd` | 87 |
| `2026-10-22-observatory-v1-s.children/e5cb8077-02f7-4d30-acb4-349e8ee0abea.settings.json` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | `5f1a2a231220eae97fff0bc61dadd595d980fb0e20f5a39e323882faac3c92f6` | 50 |
| `2026-10-22-observatory-v1-s.jsonl` | `48bd772f9f7d9a110c4a8948b727654cd3464a01a76027f516aa203bbe03e11b` | `f23e0ca6b48689d441a167094bde04c97fe20a290aaccb49e76b164362011039` | 2027 |
| `2026-10-22-observatory-v1-s.settings.json` | `42f6b820dc6c797cf408994df08f19dd5954a16610e923ed358b1f986f0113d9` | `42f6b820dc6c797cf408994df08f19dd5954a16610e923ed358b1f986f0113d9` | 453 |

Checks run on 2026-10-23 by the worker: `shasum -a 256 -c` of every redacted hash against the
committed files and of every original hash against the originals directory, 0 mismatches;
`gitleaks detect --no-git --no-banner --redact --source fixtures/streams` (gitleaks 8.30.1)
reported no leaks.

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
(rule 8) and the script's idempotence check. The hyphenated directory form is open point 5.
