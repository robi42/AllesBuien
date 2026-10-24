# beads

This directory configures [beads](https://github.com/gastownhall/beads), the tracker the maintainer's own agent pipeline works from. The issues live in a Dolt database that is not part of a clone: it syncs to this repository's `refs/dolt/data`, and `bd bootstrap` fetches it, after which `bd ready` lists what is actionable and `bd show <id>` reads one issue.

You need none of it to report a bug or to contribute: use [GitHub issues](https://github.com/robi42/AllesBuien/issues), and read [CONTRIBUTING.md](../CONTRIBUTING.md).

bd writes its stock README here only when there is none, so this one is kept.
