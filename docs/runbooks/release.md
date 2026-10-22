# Release

**When:** the mission PR has merged into `main` and the human decides the result is a
release (D-009). Agents do not push tags.

## Steps

```sh
git switch main && git pull --ff-only
git tag -a v<MAJOR.MINOR.PATCH> -m "v<MAJOR.MINOR.PATCH>"
git push origin v<MAJOR.MINOR.PATCH>
gh run watch                         # workflow: release
gh release view v<MAJOR.MINOR.PATCH>
```

`release.yml` refuses a tag that is not on `main` or not semver. With the Tauri app present
it attaches the signed `.dmg` and its sigstore bundle; before that, the release has notes
only.

## If it fails

- `verify` failed: the tag is on the wrong commit. Do not move the tag; push a new patch tag
  from the right commit and delete the GitHub release draft if one exists.
- `bundle` failed: same fix path as the gate's `bundle` job ([red-gate.md](red-gate.md)).
- Notes are wrong: they come from PR titles and labels (`.github/release.yml`); edit the
  release text on GitHub and fix the label for next time.
