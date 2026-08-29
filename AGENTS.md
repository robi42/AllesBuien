# AGENTS.md — AllesBuien

## Overview

Native Omarchy weather application implemented as a Rust workspace.

## Structure

```
AllesBuien/
├── core/    Weather, radar, notification, configuration, and theme logic
├── app/     Thin application binary; Qt integration arrives in todo 13
└── bin/     Build, test, and installation entrypoints
```

## Where to Look

| Task | Location |
|------|----------|
| Domain logic | `core/src/` |
| Application entrypoint | `app/src/main.rs` |
| Tooling commands | `bin/` |

## Commands

```bash
bin/build
bin/test
```

## Conventions

- Keep platform-independent behavior in `allesbuien-core`.
- Run all repository commands under the pinned Rust toolchain.

## Anti-Patterns

- Do not add CXX-Qt to a crate before todo 13.
- Do not write build outputs or package artifacts outside ignored paths.
