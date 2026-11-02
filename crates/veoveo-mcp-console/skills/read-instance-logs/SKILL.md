---
name: read-instance-logs
description: Read what a factory instance wrote, the Lore sync, the build and the server, and the network requests OpenShell denied it. Use when a branch fails to serve, a server misbehaves, or a request from its sandbox is refused.
---

# Read an instance's logs

`console__logs` returns an instance's latest lines across restarts, oldest first: the
Lore sync, the build and the server on `stdout` and `stderr`, and OpenShell's network
denials as `denied`. It first reads the instance's status as the person you work for, so
it shows only instances they may see.

## Read them well

- Pass `since`, such as the time of your push, to start at the run you care about.
- `limit` is 200 lines by default and at most 1000; `truncated` says older lines were
  left out, so narrow `since` rather than raising `limit`.
- With `link`, read the runner of that link of the branch instead.
- No lines and no sandbox means the instance has not started: read `console__status`
  first, which names the sandbox and the Hub's `failed`.

## What to look for

1. The last lines before an exit: the build's error or the server's panic.
2. `denied` lines: a host the sandbox may not reach. The branch's network policy, in its
   `openshell/` directory, admits hosts; a provider the repository is not granted shows
   in `console__status` as `ungrantedProviders`, which only an administrator grants.
3. A Lore sync failure at the start: the revision could not be fetched.

Fix the cause, push again, and wait with `console__status` and `wait`.
