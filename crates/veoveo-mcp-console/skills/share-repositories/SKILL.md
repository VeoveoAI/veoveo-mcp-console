---
name: share-repositories
description: Create Lore repositories that the person you work for owns, see who can reach a repository, and share it with people or with groups. Use when someone needs to clone, pull or push a repository, or when a project's people should share what you create.
---

# Share repositories

A project is a group: share a project's repositories with its group, and everyone who
joins the group reaches them. A person's own grants stay theirs.

## Create a repository

Call `console__create_repository` with a `name`. It answers the repository's `id` and
the `remote` to clone, and the person owns it at once, so the first push is admitted.
If the Hub answers that it created the repository but did not register it, call again
with the same `name` and the `id` it named. Then `console__watch` it to have its
branches served (skill `serve-a-branch`).

## See who can reach it

`console__repository_access` lists each person, by email and name, and each group, with
a level and the Lore actions it holds:

- `reader`: clone and pull, no push;
- `member`: also push to unprotected branches;
- `owner`: a member who also shares it, protects its branches and deletes it.

A protected branch takes `push-protected`, which only an administrator assigns.

## Share it

Call `console__share_repository` on a repository the person owns, with either `person`
(the email they sign in with) or `group` (its name), and `access` of `reader`, `member`,
`owner` or `none` to remove it. Their Computers reach it within five minutes. The Hub
refuses a change that would leave the person without `owner`, and a person who has never
signed in cannot be named yet.

## Groups

- `console__groups` lists the groups the person belongs to, with each group's `id`,
  members and roles.
- `console__create_group` creates a group they own; add people with
  `console__set_group_member` as `member` or `owner` (who also manages members), or
  remove them with `none`. A member reaches every repository shared with the group within
  five minutes.
- Other servers grant to the same group by its `id`, as Artifact does.
- `console__delete_group` deletes a group they own: its members lose what was shared with
  it. It is refused while the group is the one owner of a repository; make someone else
  an owner first.
