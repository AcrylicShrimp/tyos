# Development Versioning

tyos versions identify working development checkpoints for personal use.
Release management, tags, publishing, and automation are outside this policy.

## Source of truth

- The OS version is `[package].version` in the root `Cargo.toml`, using three
  components: `MAJOR.MINOR.PATCH`. Dependency version shorthand does not apply.
- Keep the `tyos` package entry in `Cargo.lock` consistent with it.
- App and library package versions are independent; do not bump them merely
  because the OS version changes. The OS version covers the integrated system,
  including bundled app improvements.
- Adopt the current `0.1.0` as the starting checkpoint. Do not retroactively
  renumber earlier work. Moving to `1.0.0` requires a developer decision.

## Choosing the next version

During `0.x` development, use these project conventions:

| Completed change | Bump | Example |
| --- | --- | --- |
| New working capability (`feat`) | Minor; reset patch | `0.1.3` -> `0.2.0` |
| Incompatible app-facing API/ABI change (`!` or `BREAKING CHANGE`) | Minor; reset patch | `0.1.3` -> `0.2.0` |
| Compatible bug fix or observable improvement | Patch | `0.1.3` -> `0.1.4` |
| Documentation, tests, tooling, or internal refactoring with no functional or compatibility change | None | `0.1.3` stays `0.1.3` |

Inspect actual behavior and compatibility, not just commit types. Syscall IDs,
argument/return conventions, and program loading/startup contracts are ABI
boundaries. An internal kernel type change alone is not an app-facing ABI break.
Use the highest applicable bump once for a completed work unit; incomplete
scaffolding does not earn a feature bump.

## Commit workflow

1. When asked to commit, inspect the intended changes and determine whether they
   form a working checkpoint. Agents decide routine version bumps without asking
   for separate approval. A commit request alone does not require a bump.
2. For a checkpoint, update the root version and matching lockfile entry. Include
   them in the same feature/fix commit. Explain the version transition in the
   commit body, e.g. `Version: 0.1.0 -> 0.2.0`, with any compatibility impact.
3. Build the embedded apps before the kernel. Run relevant tests and QEMU checks
   against the exact versioned tree; record the results and known limitations.
   A failed check must be resolved or reported before claiming a working checkpoint.
4. Stage only the intended work and version metadata, commit with the semantic
   convention, and report the commit hash and resulting OS version.

Keep independently requested changes in separate commits. Do not create a
version-only commit for its own sake, bump per file, or amend old commits to
retrofit versions. If the developer commits manually, respect that history;
evaluate the next requested work unit normally. Version-management delegation
does not authorize unrelated source edits or commits the developer did not ask for.

The numbering format follows [SemVer](https://semver.org/spec/v2.0.0.html).
The `0.x` bump rules and checkpoint workflow above are tyos-specific conventions.
