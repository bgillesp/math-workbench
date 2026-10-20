# Workflow

How a change goes from an issue to `main`. The [definition of done](#definition-of-done) at the
end is the checklist for every PR.

## Planning and issues

Work is tracked in GitHub Issues and prioritized in a GitHub Project, which holds the roadmap,
milestones, priorities, and status board. A human developer maintains the Project, by hand or in
a dedicated planning session. Agents working on issues read it but don't change it.

Issues carry labels:

- `kind/bug`, `kind/feature`, `kind/chore`, or `kind/question`
- `area/<crate>` for the crate a change mainly touches
- `needs-triage` until a human developer has reviewed and prioritized the issue

Status lives in the Project's Status field rather than in labels. Agents may read issues, comment
on progress, link PRs with `Closes #N`, and file `needs-triage` issues for out-of-scope findings.

## Change tiers

| Tier | Examples | Design doc | Tests first |
|---|---|---|---|
| Minor | typo, dependency bump, rename | No | n/a |
| Standard | bug fix, small feature within one crate | No (the issue is enough) | Yes |
| Major | new tile category, new crate, format change, public API change | Yes, accepted before implementation | Yes |

When the tier isn't clear, ask.

## Starting work

Each session takes one issue, on its own branch in its own git worktree, so several sessions can
run in parallel. Name the branch `<issue>-<short-slug>`:

```sh
git fetch origin
git worktree add ../math-workbench-42 -b 42-typed-wire-validation origin/main
```

For work that will span sessions, keep [branch notes](#branch-notes) and open a draft PR early.

## Major changes

1. **Design.** Draft a design doc from the issue at `docs/design/NNNN-short-title.md`, where NNNN
   is the issue number, starting from the [template](../design/TEMPLATE.md). It cites the ADRs it
   works under, and its acceptance criteria are concrete enough to turn into tests. A human
   developer reviews and accepts it before implementation starts. If the work requires changing
   a decision, a superseding ADR is accepted first.
2. **Tests.** Write failing tests from the acceptance criteria, as a separate commit. A human
   developer reviews the test diff before implementation. It's the executable spec, and the most
   valuable review in the PR.
3. **Implement** until the tests pass. Never weaken, delete, or `#[ignore]` an approved test. If
   one looks wrong, stop and raise it.
4. **Document.** Update the architecture overview, glossary, and crate docs. Set the design doc
   to `implemented`. Empty `docs/wip/`.
5. **Merge.** Mark the PR ready for review. It goes through CI, an AI review pass, and human
   review, and is then squash-merged.

Standard changes skip the design doc, since the issue says what's needed, but tests still come
first. Minor changes need neither.

## Branch notes

`docs/wip/` holds a branch's working notes (plan, progress, findings), committed on the branch so
a task can continue across sessions. Before merge, move each kind of content to its permanent
home and empty the directory. `just ci` enforces this.

| Content | Destination |
|---|---|
| Plan and progress log | Deleted; the PR description summarizes the outcome |
| Out-of-scope bug or improvement | New `needs-triage` issue |
| Question needing a decision | The PR, or a `kind/question` issue if it outlives the PR |
| Dead end or rejected approach | Alternatives considered in the design doc |
| Context about specific code | A comment at that code |
| Idea for later | An issue, or a draft item in the Project |
| How the system works | Architecture overview or crate docs |

If content keeps turning up that fits none of these, propose a home for it.

## Pull requests

PRs are squash-merged, and the PR title and description become the commit on `main`, so write
them for someone reading the history later.

- **Title:** [Conventional Commits](https://www.conventionalcommits.org/), scoped by crate where
  one applies, as in `feat(mw-graph): validate wire types on connect`. Common types are `feat`,
  `fix`, `docs`, `test`, `refactor`, `perf`, `ci`, and `chore`. Mark breaking changes with `!`.
- **Description:** follow the [PR template](../../.github/pull_request_template.md): what changed
  and why with `Closes #N`, what a reviewer should look at first, how the tests cover the
  acceptance criteria and which living docs changed, and any new dependency or `unsafe` code with
  its justification. Leave out the last two sections when they don't apply.
- **Draft until done.** Drafts skip CI. Mark the PR ready once the definition of done is met, and
  CI runs.

Branch protection on `main` requires CI to pass on a branch that's up to date with `main`, and
keeps history linear.

Workflows can use only actions from GitHub itself and an allowlist in the repository's Actions
settings (currently `taiki-e/install-action` and `Swatinem/rust-cache`), each pinned to a full
commit SHA with its version in a comment. Dependabot keeps the pins current. Treat a new
third-party action like a new dependency: justify it in the PR, and have a human developer add it
to the allowlist under Settings → Actions → General first, or GitHub rejects the workflow run.

Push protection rejects any push that contains a recognized credential. Don't bypass it: remove
the secret from the branch's commits, or ask a human developer if it looks like a false positive.

## Definition of done

- `just ci` passes.
- Public items are documented.
- Tests cover the acceptance criteria.
- Living docs are updated: the architecture overview, glossary, crate docs, and guides, as the
  change requires.
- `docs/wip/` is empty.
- The design doc status is set (major changes).
- Any new dependency or `unsafe` code is justified in the PR.

## Releases

Releases are tags on `main` (`vX.Y.Z`) with GitHub Releases, and release notes are generated from
the commit history. Run the extended workflow manually before tagging.
