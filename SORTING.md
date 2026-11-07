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
selected order. The last selected order is saved automatically in Delfin’s
existing `config.toml` as `general.collection_sort`, and restored for newly opened
movie and TV collections, including after restarting the application. The setting
is shared across collections. Existing configurations without this setting retain
the A–Z default. Saving does not change accounts or other preferences; a toast
reports a failed save without preventing sorting in the current view.

No Jellyfin server changes are required. English strings are provided through
the existing Fluent localization mechanism; other locales use the English fallback.

## Validation

```sh
MESON_BUILD_ROOT="$PWD/build-local" FLATPAK=false \
CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_DEBUG=0 \
  cargo test --locked --release -p delfin --lib \
    --target-dir "$PWD/build-local/delfin"
git diff --check
```

The Linux unit suite passes, including all four dropdown/API mappings, config
round-trips for every sort choice without changes to other settings, and V1/V2
backward compatibility. The serde-enabled sort model is tested through Cargo,
not as a standalone `rustc` file. Live Jellyfin behavior is unchanged.

## Manual acceptance check

Build with the upstream Meson/Flatpak workflow documented in README.md. Open a
movie collection with more than 24 items, then:

1. Verify the default is A–Z and change to each of the other three choices.
2. Verify date ordering against the dates in Jellyfin.
3. Visit page two and confirm the selected order remains applied.
4. Change sort while on page two; verify pagination resets to page one.
5. Refresh, reopen the collection, and restart Delfin; verify both the dropdown
   and the loaded items retain the selection.
6. Switch choices quickly while loading; verify old results never replace the new order.
7. Verify keyboard navigation of the dropdown, and test a narrow window and an empty library.
8. Repeat in a TV collection.

This change does not update an existing installed Delfin client automatically.
