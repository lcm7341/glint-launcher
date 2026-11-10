# Architecture

Glint is one Rust binary crate. Its first architectural boundary is between
application policy, presentation, and host operating-system mechanisms.

## Ownership

| Module | Owns |
| --- | --- |
| `main` | Weft startup and wiring the application to its window |
| `ui` | Rendering, selection, dialogs, filters, and mod file-selection state |
| `app` | Actions, application state, feature coordination, notices, and polling |
| `platform` | Host paths, pickers, desktop opening, Steam discovery, native commands, and process queries |
| `launch` | Shared launch policy, process lifecycle, and compatibility runtimes |
| `launch/wine` | The managed Wine distribution, prefix, and Steam for Windows |
| `steam` | Shared Steam library/shortcut formats and host-client discovery coordination |
| `depot` | DepotDownloader installation, arguments, output, and manifest integration |
| `instances` | Instance files, discovery, and save-swap recovery |
| `gd` | Game versions, builds, catalog, and executable formats |
| `mods` | Loader/mod inspection, file operations, and Geode API integration |
| `paths` | Glint's directory layout, independent of settings I/O |
| `settings` | Application preferences, their text format, loading, and saving |
| `parse_error` | Shared error for the small text formats |
| `task` | Background jobs and the UI wake callback type, without depending on `app` |
| `format` | Small formatting helpers shared by notices and presentation |

`app` coordinates feature implementations. Those implementations do not import
UI modules. UI code reads application/domain state and submits actions; it does
not perform installation or save swapping. A selected mod file becomes an
`Action::Mod` containing its path, and the application rechecks import conditions
when that action executes.

## Application state

`App` holds feature owners and coordinates operations that involve more than one
of them. It does not own their individual task slots, process handles, or caches.

| Owner | Private state and operations |
| --- | --- |
| `InstanceLibrary` in `app/library.rs` | Instance store, loaded entries, discovered installs, CRUD, and Steam adoption |
| `Downloads` in `app/downloads.rs` | Tool location/acquisition, pending instance, active transfer, seeding, progress, and cancellation |
| `PlaySession` in `app/play.rs` | Launch environment, Steam shortcuts, active game, save routing, and external-game monitor |
| `WineSupport` in `app/wine.rs` | Wine locations, setup task, setup progress, and Wine Steam launch preparation |
| `ModManager` in `app/mods.rs` | Installed-mod snapshots and the current loader/index installation task |
| `Activity` in `app/activity.rs` | Pending notices and the bounded application log |

Instance mutations refresh the library before returning. Transfer polling returns
an update for the coordinator to apply to settings and instance completion
markers. Game exit polling tells the coordinator whether save restoration is
required. Feature owners report notices and log entries through `Activity`; they
do not need the rest of `App` to report progress.

The `coordinator.rs` files contain the remaining `impl App` methods that bridge
features. For example, starting a download combines the catalog, instance library,
and saved Steam account; its transfer lifecycle then belongs to `Downloads`.
Mod commands consult the game session before changing Geode settings. This is
application policy rather than a dependency from the mod cache onto `App`.

The external-game monitor receives a shutdown signal when its session is dropped.
Mod snapshot inputs are evaluated only when the cache needs refreshing.

Instance names and instance configuration live in `instances`, rather than `gd`.
The existing settings and instance file formats and directory names are retained.

## Host, game build, and launch route

These are separate concepts:

- `HostOs`: Windows, macOS, or Linux, describing the machine running Glint.
- `GdPlatform`: Windows or macOS, describing the game build.
- `LaunchRoute`: native, Wine, or Proton, describing execution.

The host implementation is selected in `platform.rs`. Each host implements the
same operation signatures under `platform/windows`, `platform/macos`, or
`platform/linux`. Normal callers use the facade rather than reaching into those
directories or choosing operating-system commands themselves.

The directory trees need not be identical. Linux has no native GD build, so it
has no native `launch.rs`; its entry point returns `ErrorKind::Unsupported`.
Unimplemented process queries also return `Unsupported`, distinct from a
successful query that found no processes. Existing application callers retain
their fallback behavior; consult the parity table before relying on a query.

Native Steam discovery belongs to the host layer. Steam for Windows inside a
managed Wine prefix belongs to `launch/wine`, including its window fix and DLL
overrides. Steam's shared file formats belong to `steam`.

Feature-specific release asset selection and Unix filesystem API adapters can
remain with their feature. A platform directory is not a reason to move every
conditional-compilation expression away from its owner.

## Launch decisions

`launch/plan.rs` contains a pure policy function over `LaunchFacts`. It returns
one of:

- a ready launch route;
- Wine setup required;
- Wine setup in progress;
- a specific blocking condition.

The Play control and execution both consume this decision. Steam DRM uses
Steam-mediated launching under Wine and blocks standalone Proton launching.
An incompatible macOS build is rejected before execution.

Readiness describes route selection and configured prerequisites. It is not a
guarantee that an external process will start: Steam sign-in, process races,
Rosetta, and filesystem errors are checked during execution as appropriate.

## Verification and parity

See [platform-parity.md](platform-parity.md) for implementation and verification
status. `bash scripts/check.sh` runs formatting, file-size checks, build, unit
tests, and Clippy.

Tests compile all three host contracts on the current test host and inspect
command arguments without launching native dialogs. This catches interface
drift and argument construction errors, but does not establish native runtime
parity or replace builds on Windows and Linux.

Regression tests exercise library mutations, Steam adoption/deletion, source
selection for download seeding, task completion/failure, mod-cache invalidation,
and save swapping. Save tests use temporary directories, including interrupted
and ambiguous recovery; they do not touch the user's actual saves. Background
installation tests provide completed jobs without downloading or running tools.

File boundaries should continue to follow ownership and what changes together.
An identical platform file tree, a line limit, or the earlier proposed directory
tree is not a reason to split a cohesive module or combine unrelated behavior.
