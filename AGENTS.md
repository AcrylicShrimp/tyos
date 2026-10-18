# Agent Guidelines

## Purpose

tyos is a personal RISC-V OS project. The developer's focus is building and
understanding the kernel. Agents are kernel copilots and may implement delegated
application and user-library work.

## Ownership

| Area | Agent permissions |
| --- | --- |
| `apps/`, `crates/tyos-ulib`, `crates/tyos-ulib-macro` | Implement requested features, fixes, and relevant tests. Preserve existing startup and kernel interface contracts. |
| `crates/tyos-syscall` | Implement user-side wrappers for existing syscalls; leave syscall IDs, calling conventions, and kernel semantics to the developer. |
| App-local build/linker files and manifests | Change as needed for delegated work, preserving the existing loading contract. |
| Kernel (`src/`), all other crates, kernel/workspace build and linker configuration | Read-only: explain, investigate, review, and provide pseudocode or manual change instructions. |
| Documentation | Create or update when requested. |

Tests in read-only areas may be edited only on an explicit test-related request.
A feature request does not authorize changes to developer-owned code. When an
application needs missing kernel support, do not force a user-space workaround
that adds undue complexity or weakens the intended behavior. Explicitly request
the needed kernel capability from the developer, explaining why it is needed
and the proposed interface. Continue independent work; leave dependent work
blocked until support exists. Do not move code across boundaries to bypass
ownership. Revise these rules only when asked.

## Working Principles

- **Follow dependency conventions.** Declare dependencies in root
  `[workspace.dependencies]` and inherit them with `workspace = true`. Use only
  the major version (`"1"`); for `0.x`, include the minor version (`"0.2"`).
- **Understand first.** Read the relevant code. State consequential assumptions
  and tradeoffs; ask when uncertainty affects correctness or ownership.
- **Prefer simplicity.** Solve the requested problem with the smallest clear
  design. Avoid speculative features, dependencies, and premature abstractions.
- **Keep changes focused.** Follow existing style, preserve the developer's work,
  and avoid unrelated cleanup. Remove leftovers introduced by your own changes.
- **Define success.** Identify observable acceptance criteria before implementing.
  Verify behavior with appropriate tests or bounded QEMU runs, including relevant
  failure paths. Build success alone does not prove runtime behavior.
- **Finish delegated work.** Iterate within scope until the criteria are met or a
  concrete blocker remains. Report what changed, what was verified, and what is
  still uncertain. Keep explanations and process proportional to the task.

Read-only diagnostics, builds, and QEMU runs are allowed throughout the project.
During kernel reviews, pay particular attention to privilege transitions, memory
ownership, task lifetime, linker layout, and syscall ABI boundaries.

## Commits

Use `type: summary` or `type(scope): summary` with a short, imperative summary.
Types: `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `perf`, `chore`.
Scope is optional and names the affected subsystem, e.g. `shell`, `ulib`, or `mem`.
Keep each commit focused on one logical change.

These principles adapt the bounded scope, measured verification, and simplicity
criteria in [Karpathy's autoresearch instructions](https://github.com/karpathy/autoresearch/blob/master/program.md)
to this project's human-led kernel development.
