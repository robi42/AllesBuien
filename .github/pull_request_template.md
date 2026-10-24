## What and why

<!-- What this changes, and why. Link the issue it closes, if there is one. -->

## How it was checked

- [ ] `bin/check-fast`
- [ ] The tests nearest the change: `cargo test -p allesbuien-core`, or `QT_QPA_PLATFORM=offscreen cargo test -p allesbuien --test <file>`
- [ ] `bin/test` in full, or left to CI, which runs it on every pull request (over twenty minutes from cold)

## Checklist

- [ ] A line under Unreleased in `CHANGELOG.md`, for anything a user of the latest release would notice
- [ ] Every path this touches that `.factory/protected` lists is named above, with why
- [ ] A fixture in `tests/gates/` for any new gate, which the gate is watched refusing
- [ ] Tests that use fakes, with no live network and no change to the process environment

[CONTRIBUTING.md](https://github.com/robi42/AllesBuien/blob/main/CONTRIBUTING.md) says more about each of these.
