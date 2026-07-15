# Design

The design system is itself negentropic. This site is judged twice: once in
its source, by the checker, and once on its face, by anyone who opens it. The
face answers to the same constitution — a closed vocabulary, declared
territories, and violations a freshman can point at.

## The four organs

All four live in `packages/components/src`, the style territory.

- `tokens.scss` — **defines** the design vocabulary. One typed `@property`
  registration per token; a var not registered here does not exist. Typed
  registrations carry canary initials (`magenta`, `0px`): a theme that forgets
  a binding confesses on screen.
- `themes/<theme>.scss` — **declares values**. One binding table per theme;
  every registered token bound exactly once. This is the only file kind in
  the repository where a design literal (hex, rem, ms) may appear. Compile-time
  scss locals may dedupe values inside a theme; they never leave the file.
- `media.scss` — **defines the seams**. The only home of `@media`, spelled
  once per seam as a mixin (`wide` / `narrow`, one seam at 48rem); it also
  flips the `--seam` marker so the runtime can ask which world it is in
  without ever learning the numbers. Media states are enumerated worlds,
  like themes on the viewport axis — not a breakpoint ladder.
- `<Atom>.scss` — **consumes**. Component sheets speak `var()` plus the
  enum whitelist below, and reference seams only via the media mixins.
  No literals, no raw `@media`, ever.

## The token table

28 seats. Every seat is used by a living component; an unused token is a
squatter and gets evicted.

| dimension | tokens | rationale |
| --- | --- | --- |
| color | `ground` `panel` `well` `rule` `ink` `bright` `muted` `accent` | the page, its raised and sunken surfaces, one line color, three text voices, one accent |
| type | `fine` `body` `hero` | small print, prose, inscription — three sizes, no ladder |
| space | `space-1` … `space-6` | 0.25 / 0.5 / 1 / 1.5 / 2 / 4 rem — a countable gamut |
| radius | `radius` `bead` | the corner and the pill |
| line | `line` | one stroke width |
| leading | `leading` | one rhythm |
| face | `sans` `mono` | the voice and the ledger |
| measure | `page` `prose` `cell` | page width, reading width, grid cell |
| track | `track` | the inscription's letter squeeze |
| seam | `seam` | the runtime-readable media marker |

## The whitelist

Atom sheets may use, beyond `var()`: CSS keyword enums (`flex`, `grid`,
`inline-flex`, `inline-block`, `wrap`, `center`, `none`, `auto`, `pointer`,
`border-box`, `hidden`, and their peers) and the identity values `0`, `100%`,
`1fr`. **No unit literal is ever whitelisted** — the moment a rule wants a
number with a unit, it wants a token.

## The consumption ladder

Media differences resolve at the cheapest rung that holds; each rung down
needs a harder reason.

1. **Fluid** — layout absorbs the continuum; most differences die here.
2. **Rebind** — a theme rebinds a var under a seam; the var stays, the value
   breathes, atoms remain ignorant.
3. **Shift** — an atom restructures under a media mixin (nav folds, grid
   changes column count). Rare; each instance reviewable on its own.
4. **Fork** — the atom itself splits into media-exclusive incarnations.

## The fork law

When an interaction contract cannot be bridged by CSS alone — DOM structure
differs (sheet vs popover), input modality differs (touch picker vs keyboard
navigation), the accessibility contract differs — the atom forks: one public
name in `lib.ts`, one incarnation per media world under a media-named
directory (`wide/Select`, `narrow/Select`). The fork never leaks upward:
apps compose one name; a single dispatcher inside the components territory
reads the `--seam` marker and mounts exactly one incarnation. Size, density,
and spacing never justify a fork — they are rung 2. Forks are expected to be
rare; a crowd at rung 4 means the seam is wrong or the design is fighting
the carrier.

## Naming

A token name is one vocabulary atom, optionally followed by an ordinal
(`--ink`, `--space-2`). If a token cannot be named in one word, its seat is
suspect.

## Doctrine

- Fluid first; the face has a few named seams, not a responsive continuum.
- A design literal has exactly one legal residence: the theme file.
- `@media` is spelled in exactly one file: `media.scss`.
- Interactive incarnations mount singly; `display: none` is not a dispatcher.

## Amendment

Adding a token, a seam, a whitelist entry, or a fork is a reviewed contract
change: name the seat, state the rationale, land it with the change that
needs it. These rules are not yet negentropy law — precedent first, law
after; when the violations have been pointed at often enough, the grant-law
shape is already visible.
