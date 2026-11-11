# Glint Launcher

**Glint Launcher** is a program for managing Geometry Dash (GD) versions and instances. Instances are separate copies of the game, which can have their own configs, save files, and mods. Downloads are done through [DepotDownloader](https://github.com/steamre/depotdownloader), after authenticating with Steam Guard.

This solves the problem of struggling to downgrade after a GD update happens. Glint also integrates with Geode, allowing you to install and manage mods for versions 2.2 and above. For older versions, Glint Loader is used as a free dll loader.

This is not an alternative/replacement for Geode or Mega Hack. This is a separate tool entirely.
## Status

Early in development. Usable on Windows, MacOS, and Linux. More features and fixes are planned.
## Features

- Download any GD version from 1.9-latest
- Add existing GD instances (not downloaded by Glint)
- Play any GD instance from Glint
- Rename and favourite instances
- Install Geode into any GD instance (2.2-latest)
- Install Glint Loader for GD versions 1.9-2.113
- Browse and install Geode mods in the launcher
- Toggle and manage mods in the launcher
- Pin mods (saves between Geode and Glint)
- Add local mods directly
- View logs
## Planned

- More advanced linking to main save file (on instance creation, setting, when selecting instance)
- Geode install and Geode mod management on MacOS
- Instance management (copying, exporting/importing instance)
- Mod packs (exporting lists of geode links to download or the mods themselves)
- GDPS integration (list of available gdps servers to download)
- Levels list (like prism launcher, export/import in various formats)
- Texture pack tab
- Screenshots/clips tab
## Discord
If you have any questions, need help, or just want to chat, please join our [Discord server](https://discord.gg/7pseDg9Wse)!

## Developers
### Building From Source
Unfortunately, this project utilizes a handy and easy to use UI library that was largely developed using artificial intelligence as a test case for recent developments in LLMs. Due to the scope of that project, and facts regarding its creation and development, I have chosen to keep that repository private. This means that, although glint is open source, the UI library (weft) will remain, at least for the time being, closed source. This means that building this project from source is currently not possible, and the open repository stands to simply view the code of the project.

We understand the side effects of this, particularly residing in trust, and if it gets to the point where people would much rather I publicize the weft source, I will do so.
