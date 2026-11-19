# 0015. Use a pinned version of opensocial.group

Status: Provisional (the proposal is a draft and will change)

## Context

The backend plan calls for Worlds to be groups that other apps understand.
Use opensocial.group records where they fit, and Motet records under
`at.motet` where they do not.

The [proposal](https://tangled.org/opensocial.group/proposal) defines two
well-known Spaces for each group: `group.opensocial.meta` and
`group.opensocial.members`. They hold the profile, rules, labeler, roles,
permissions, membership, acceptance, access and Space index records. It also
defines a per-user `invites` Space, permission sets and group-host methods.
The host is the OAuth authorization server for the group's DID. Roles grant
OAuth Space scopes rather than named action verbs.

## Decision

Vendor the proposal's lexicons unchanged at revision
`2b41a9bf9171c03a1aa5ee26c6e39823ba27c79d` (2026-10-19) in
`lexicons/group/opensocial/`. Include `com.atproto.label.label`, the proposal's
strawman label record and the `com.atproto` definitions they reference.

- Leave the missing licence for the maintainer to raise with upstream.
  The authors [intend the proposal to be public](https://atproto.com/blog/atmospheric-groups-intro),
  but it has no licence file. Record that gap in `lexicons/README.md`.
- Follow the proposal when creating a World. Create `meta` and `members`
  Spaces with an `access` record in each, a `profile`, a `permissions` record
  and the owner's `membership`. Give the owner's role every scope and the
  ability to assign every role. Publish a `declaration` in the public repo
  when `meta` is public.
- Treat Motet-specific Spaces for GMs, private scenes and applications as
  modality Spaces: Spaces used by a particular app feature. Name them under
  `at.motet.pilot.space.*` and list them in the Space index unless they must
  stay secret.
- Do not add fields to opensocial schemas. Store extra Motet fields in
  separate records under `at.motet`, following the plan's modelling rules.

## Consequences

`motet-lexgen` must support Space, permission-set, query, procedure, token and
union definitions. Updating the proposal requires an explicit revision bump
of the vendored files and corresponding changes to code that writes them.

## Revisit when

The proposal gains a licence, tags a release or changes its Space or record
model.
