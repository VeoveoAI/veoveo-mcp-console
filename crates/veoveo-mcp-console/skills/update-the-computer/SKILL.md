---
name: update-the-computer
description: Read which version the person's Computer runs, what the current version brings, and restart the Computer at it when the person asks. Use at the start of a session and whenever the person asks about updates.
---

# Update the Computer

`console__computer` reports the person's own Computer: the version it runs and the
current one, whether an update is available and what it brings, a restart that runs or
the last one's failure, a cause that blocks a restart, and an update installed since the
person was last told.

1. Call `console__computer` at the start of a session.
2. If an update is available or was just installed, tell the person what is new in a line
   or two.
3. Restart only when the person asks: call `console__computer` with `restart: true`. The
   Hub answers before the restart begins. `~` is kept; running programs and `/tmp` are
   not, so finish or save work first, and expect this session to end.
