# Platform parity

“Implemented” describes code present in this repository, not a claim of native
runtime verification. The current refactor was built and linted on macOS ARM64.
Unit tests compile all three host contracts and inspect commands; native dialogs,
game launches, Wine setup, and Windows save swapping were not run during it.

| Capability | Windows host | macOS host | Linux host |
| --- | --- | --- | --- |
| Application data directory | `APPDATA` | Application Support | XDG data directory |
| Home directory | `USERPROFILE`, then `HOME` | `HOME` | `HOME` |
| Folder/DLL/Geode picker | PowerShell Windows Forms | AppleScript | kdialog, then zenity |
| Open a path or URL | Explorer | `open` | `xdg-open` |
| Host Steam discovery | Registry and Program Files | Application Support | Standard, legacy, and Flatpak paths |
| Native GD launch | Implemented | Implemented; encrypted old builds blocked | Not applicable: no native game build |
| Windows build compatibility runtime | Not applicable | Managed Wine and Steam for Windows | Proton and optional Steam Linux Runtime |
| Steam DRM Windows build | Native launch path | Launch through Steam for Windows | Standalone Proton launch blocked |
| Instance save isolation | Swap, backup, recovery journal | Native home override or Wine save link | Proton compatibility directory |
| Detect game by executable name | tasklist | Unimplemented host query | Derived from `/proc` game directories |
| Detect external game directories | Unimplemented | Unimplemented | `/proc` command line and working directory |
| Detect host Steam process | tasklist | Unimplemented | Unimplemented |

The Wine Steam client has separate process checks under `launch/wine`. Those do
not implement host Steam detection on macOS.

Unimplemented host queries return `ErrorKind::Unsupported`. Some existing
application workflows fall back on a managed session, assume Steam may be
running, or use an empty external-game list. In particular, externally launched
games are not comprehensively detected on Windows or macOS.

## Checks performed

- Every host implements the common paths, desktop, picker, process, Steam
  discovery, and native-launch signatures.
- Windows picker paths are passed through the environment rather than inserted
  into PowerShell source; STA is requested.
- macOS picker paths are passed as AppleScript arguments.
- kdialog and zenity receive the initial directory and correct file filters.
- Native commands retain the Steam application ID and macOS save-home override.
- Every supported host/game-build pair has an explicit launch decision.
- Wine setup, missing Proton/Steam, encrypted macOS builds, and Steam DRM routes
  are covered by policy tests.
- Save-swap round trips, interrupted recovery, and ambiguous recovery that must
  preserve all copies are tested with isolated temporary directories.
- Application task tests cover completion/failure and state release without
  launching native installers or downloading packages.

## Native verification still required

On each host, build the actual target and check picker selection/cancellation,
opening files and URLs, Steam discovery, game startup/exit, and save isolation.
On Windows, include interrupted save-swap recovery. On macOS, include Wine setup,
Steam sign-in, and a Steam DRM build. On Linux, include both supported picker
programs and external-game detection under Proton.
