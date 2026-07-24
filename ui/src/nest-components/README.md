# @nest/components (vendored)

Selected pieces of the Nest framework's `@nest/components` React library
(`core/crates/nest-react-components` in the main `nest` repo), copied in as
source rather than depended on.

## Why vendored instead of a path dependency

`sparrow`/`finch` depend on it directly (`"@nest/components": "../../../../core/crates/nest-react-components"`).
Airtable Sync doesn't, for two reasons:

1. **Portability.** Airtable Sync is its own git repository (like Kiwi), not a
   folder inside the `nest` monorepo. A relative path dependency only resolves
   if `core/` happens to be checked out at that exact relative location next
   to this repo — true today on this machine, not guaranteed for anyone else
   cloning this repo standalone.
2. **A real `tsc -b` bug**, independent of (1). `@nest/components` ships raw
   `.tsx` source (no compiled `dist`) and its own separately-managed
   `node_modules` (with its own `@types/react`). When a consumer both has its
   own `@types/react` *and* pulls in this package as a linked dependency, `tsc`
   sees two physically distinct copies of `@types/react` and fails with
   `"Two different types with this name exist, but they are unrelated"` on
   `Popover.tsx`/`Select.tsx`'s ref-callback types — reproduced identically in
   `apps/sparrow/desktop/ui` too, so it's pre-existing there, just unnoticed
   (`vite build` alone doesn't hit it; only a full `tsc -b` does). Vendoring
   avoids this structurally: there's only ever one `@types/react` in play
   because these files are just part of `src/`, checked by the same
   `tsconfig.json` as everything else.

Kiwi vendors the **entire** library (it has a Components browser feature that
needs all of it). We only vendor what we actually use — currently just `Chip`.

## What's vendored

| Component | Vendored from | Source commit |
|-----------|----------------|----------------|
| `Chip` | `core/crates/nest-react-components/src/components/data-display/Chip.tsx` | `ffb8f6d` (2026-07-12) |

Plus `lib/cn.ts` (the `clsx` + `tailwind-merge` className helper every
component uses), same commit.

## Adding another vendored component

Copy the component's `.tsx` file into `components/<category>/`, copy any
`lib/*.ts` helper it imports that isn't already here, add the entry to the
table above with the commit hash (`git log -1 --format=%H -- <path>` from the
`nest` repo root), and export it from `index.ts`. Install any new runtime
dependency it needs (check its imports) the normal way in `ui/package.json` —
`clsx`, `tailwind-merge`, and `lucide-react` are already here for `Chip`;
components using `@floating-ui/react` or `@react-aria/*` will need those too.

## Syncing upstream changes

Diff the vendored file(s) against the current upstream source before
re-copying — these are hand-copied, not rsync'd wholesale like Kiwi's (we
only have one component, not thirty):

```sh
# from the nest repo root
diff core/crates/nest-react-components/src/components/data-display/Chip.tsx \
     apps/airtable-sync/ui/src/nest-components/components/data-display/Chip.tsx
```

If it changed upstream, copy the new version in, keep the header comment
(source path + commit hash) updated, and bump the commit hash in the table
above.
