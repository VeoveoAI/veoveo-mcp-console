---
name: see-the-factory
description: See what the factory serves, in one read - every server and branch, the Lore repository and links behind each, its data environment, which branch serves the person, and what should serve and does not.
---

# See the factory

Call `console__factory` when the person asks what is running, which version of a server
they have, which branch or data environment they are on, or what is wrong in the factory.
It answers everything their grants reach. The `Factory` App shows the same answer as a
page, on the Hub's home page.

## Reading the answer

- `routes` says which branch serves the person each server. `selected` says they chose it;
  otherwise the base branch serves them. `unavailable` says why calls to that server fail.
- `repositories` holds one entry per Lore repository. `server` is the name its tools carry,
  which often differs from `title`, the repository's name.
- An instance's `detail.status` is what `console__status` reports: `appliedRevision`,
  `servingRevision`, the sandbox's phase and `failed`. `detail.database` is its data
  environment.
- `pusherHoldsGrant` false means Lore refuses that instance's token, so it serves until its
  next start and then cannot fetch its branch. A push by someone with a grant fixes it.
- A branch's `latest` later than its instance's `appliedRevision` means the branch is ahead
  of what serves.
- A branch's `state` says what became of it:
  - `served`: it has an instance or runners.
  - `no_run`: its latest revision has no executable `openshell/run`, so it does not ask to
    serve. This is not a failure.
  - `pending`: it asks to serve and nothing has failed yet.
  - `prepare_failed`, `name_refused`, `links_failed`, `unreadable`: it should serve and does
    not, and `message` or `linkFailures` says why.

## Links

An instance with `link` is a runner: the parent repository's program, serving the linked
repository's tracked branch. `status.runner` names that repository and branch, which is
where people push to change what the link serves. People reach a link by their grant on
the linked repository, and call it as `{server}__{link}__{tool}`. Each link has its own
database. A link that could not run is in its branch's `linkFailures`; usually nobody
watches its repository, which `console__watch` fixes for someone with a grant on it.

## Going further

- Read an instance's output with `console__logs`, and why it fails with `console__status`.
- Change what serves the person with `console__select`. It changes every chat of theirs and
  which data environment their calls use, so say so before you do it.
- `scope: "tenant"` reads the whole factory, for an administrator. It names people,
  repositories, branches and revisions, and holds no file, log or row.
