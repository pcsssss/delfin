# Collection sorting

Local feature branch: `feat/library-sorting`
Upstream: https://codeberg.org/avery42/delfin (base commit `820b466`).
GitHub fork: https://github.com/pcsssss/delfin (default branch: `main`).

The collection header now offers Title: A–Z, Title: Z–A, Date added: newest first,
and Date added: oldest first. These apply to movie and TV collections.

Ordering is sent to Jellyfin using `SortBy` and `SortOrder`, before the server
applies pagination. Date added uses Jellyfin's `DateCreated`, not release date or
filesystem modification time. Title/year are secondary sort keys for equal dates.
Changing sort recreates the media page, starting at page one. Refresh keeps the
selected order. A newly opened collection defaults to A–Z; the choice is not
persisted across reopening the collection or restarting the application.

No Jellyfin server changes are required. English strings are provided through
the existing Fluent localization mechanism; other locales use the English fallback.

## Validation

```sh
rustc --edition=2021 --test delfin/src/jellyfin_api/models/collection_sort.rs \
  -o /tmp/delfin-collection-sort-tests
/tmp/delfin-collection-sort-tests
git diff --check
cargo check --locked -p delfin
```

The four standalone sort tests and whitespace check passed on the development Mac.
Full Cargo check is blocked by missing GTK4/Graphene native dependencies on this
Mac (GTK >= 4.12 is required). No GUI build or playback test has been completed.
Live API validation was skipped because no active Jellyfin session was available.

## Linux acceptance test (still required)

Build with the upstream Meson/Flatpak workflow documented in README.md. Open a
movie collection with more than 24 items, then:

1. Verify the default is A–Z and change to each of the other three choices.
2. Verify date ordering against the dates in Jellyfin.
3. Visit page two and confirm the selected order remains applied.
4. Change sort while on page two; verify pagination resets to page one.
5. Refresh; verify the selection is unchanged.
6. Switch choices quickly while loading; verify old results never replace the new order.
7. Verify keyboard navigation of the dropdown, and test a narrow window and an empty library.
8. Repeat in a TV collection.

This change does not update an existing installed Delfin client automatically.
