---
name: serve-a-branch
description: Have the Hub build and serve a branch of a Lore repository, wait until it serves after a push, and choose which branch serves you each server. Use when you push a server's code, ask why a branch does or does not serve, or switch yourself to another branch's instance.
---

# Serve a branch

The Hub builds and serves every branch of a repository it watches, as a factory
instance, and you choose per server which branch's instance serves you. Every call acts
as the person you work for: the Hub decides by their grants, and its refusal is the
tool's error, with the HTTP status and the reason.

## Once per repository: watch it

Call `console__watch` with the repository's ID. From then on every branch with an
executable `openshell/run` is built and served when someone pushes it. Watching needs
the person's grant on the repository; a repository you just created with
`console__create_repository` can be watched at once.

## After each push: wait for it to serve

1. Push the branch with Lore.
2. Call `console__status` with the repository, the branch and `wait` (at most 60
   seconds). It answers as soon as the revision the Hub applied serves or fails.
3. The branch serves when `servingRevision` equals `appliedRevision` and
   `sandbox.phase` is `ready`.
4. Otherwise read the reason in this order: the Hub's `failed`, its stage and message;
   then the sandbox's `conditions`, `exitCode` and `restartCount`; then any `warning`;
   and `ungrantedProviders`, the providers the branch asked for that its repository is
   not granted. For the program's own output, follow the skill `read-instance-logs`.

A status of 404 means the Hub has no instance of that branch: check that the repository
is watched and that the branch has an executable `openshell/run`.

## Linked runners

A branch whose program runs over another repository has a runner per link. Pass `link`
to `console__status` (and to `console__logs`) to read that runner instead of the branch.
The runner is ready when its `servingRevision` names its `appliedRevision` and its
`sandbox.phase` is `ready`; `runner`
names the linked repository and its tracked branch, where people push to change what the
runner serves, and `runnerFailure` says why the Hub could not run the link.

## Choose which branch serves you

Call `console__select` with the server's slug and a branch to be served that server from
that branch's instance in every chat, or with `branch: null` to return to the base
branch. Your tool list changes with it: the selected instance's tools replace the
server's. Select a branch only after `console__status` shows it serving.
