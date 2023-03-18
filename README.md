# Modda

Automates the installation of a bunch of mods above an Infinity Engine based game.
(though so far, it was only tested with BG1 & BG2).

## Initial goal

Be able to generate (mostly) reproducible installations

## Operation

You create a recipe that
 - decides which language(s) will be selected
 - lists all mods that should be installed, in order, and of the components for each mod

The recipe is a YAML file (This could probably be a json file, but this was not tested), for example this is for an installation on BG1 with a preference for french and fallback on english.

```yaml
global:
  lang_dir: "fr_FR"
  lang_preferences: ["#rx#^fran[cç]ais", french, english, "american english"]
modules:
  - name: DlcMerger
    components: [
      1 # Merge DLC into game -> Merge "Siege of Dragonspear" 
    ]
    location:
      source:
        github_user: Argent77
        repository: A7-DlcMerger
        release: v1.3
        asset: lin-A7-DlcMerger-v1.3.zip
  - name: bg1ub
    components: [
      0,  # Ice Island Level Two Restoration
      11, # Scar and the Sashenstar's Daughter
      12, # Quoningar, the Cleric
      13, # Shilo Chen and the Ogre-Magi
      14, # Edie, the Merchant League Applicant
      16, # Creature Corrections
      17, # Creature Restorations
      18, # Creature Name Restorations
      19, # Minor Dialogue Restorations
      21, # Store, Tavern and Inn Fixes and Restorations
      22, # Item Corrections and Restorations
      29, # Duke Eltan in the Harbor Master's Building
      30, # Nim Furlwing Encounter
      32, # Svlast, the Fallen Paladin Encounter
      33, # Mal-Kalen, the Ulcaster Ghost
      34  # Chapter 6 Dialogue Restorations
    ]
    location: # here, using a json-y representation for the source, also works with double quotes
      source: { github_user: Pocket-Plane-Group, repository: bg1ub, release: v16.4,
                asset: bg1-unfinished-business-v16.4.zip }
```

The comments are optional of course, they are only for the reader.

## Fetching mods

- You can specify a `location` for fetching a mod.
- If a module doesn't have a `location` field, it is expected to already be in the game directory.
- If a mod `.tp2` file is found in the game directory, the `location` is ignored.

### Example 1: HTTP fetch

```yaml
    - name: iwdcrossmodpack
      components: ask
      location:
        http: http://america.iegmc.net/g3//lin-IWDCrossmodPack-v1.4.tar.gz
```

### Example 2: Github fetch

To obtain a _release_
```yaml
  - name: iwdification
    components:
      - 30 # IWD Arcane Spell Pack: Release Candidate 2
      - 40 # IWD Divine Spell Pack: Release Candidate 2
    location:
      github_user: Gibberlings3
      repository: iwdification
      release: v5
      asset: lin-iwdification-v5.tar.gz
```
To obtain a tag
```yaml
  - name: iwdification
    components:
      - 30 # IWD Arcane Spell Pack: Release Candidate 2
      - 40 # IWD Divine Spell Pack: Release Candidate 2
    location:
      github_user: Gibberlings3
      repository: iwdification
      tag: v5
```
### Example 3: Local (file-system) location

```yaml
  - name: willowisp
    components: # 4 later
      - 0 # Will NPC, shaman stronghold and new shaman kit for BG2EE
      - 1 # Change shaman .tlk string to remove "Ineligible for any stronghold" line
      - 2 # New items for shamans and undead NPCs
      - 3 # Optional: Drider and Dark Treant Enemies
    location:
      path: /home/me/my_mods/static/Will of the Wisp v2.20.zip
```

## Limitations

- Only tested with weidu 247
- At this point, was only tested on linux
- Mods that use `ACTION_READLN` are not handled well (installation is interrupted until the user makes some choice, and reproducibility is not guaranteed)

## Errors and warnings

Mods that end in a weidu `ERROR` interrupt the installation.

_By default_, mods that emits `WARNINGS` are interrupted too but this can be disabled at the mod level with a ignore_warnings property

```yaml
  - name: rr # rogue rebalancing
    components: [ 0, 1, 2, 3, 4, 5, 7, 8, 11, 12]
    ignore_warnings: true # component 7: WARNING: no effects altered on MISC2P.ITM
```

I would advise that the components with warnings should be isolated:

```yaml
  - name: rr # rogue rebalancing
    description: components before number 7 which gives a warning
    components: [ 0, 1, 2, 3, 4, 5]
  - name: rr # rogue rebalancing
    description: "components 7 alone, warns with WARNING: no effects altered on MISC2P.ITM"
    components: [7]
    ignore_warnings: true
  - name: rr # rogue rebalancing
    description: components after number 7 which gives a warning
    components: [8, 11, 12]
```

If the components with warning has no order dependency or reverse-dependency with the other components in the mod, it can be made simpler by grouping all other components in a single set.

## Configuration

This uses a configuration file with one single config property (at the moment).
The file is name `modda.yml` and will be taken from the current directory (first) then from the "OS conventional location for application configuration.

- on linux, it should be `~/.config/modda/modda.yml`
- on windows, shoudld be around `C:\Users\<username>\AppData\Roaming\modda\config/modda.yml`
- on macos, something like `$HOME/Library/Application Support/modda/modda.yml`

It currently contains one property: `archive_cache` which tells the program where to store and search for downloaded modf archives.

```yaml
# can be an absolute path, or can use ~ exapansion on UNIX-like OSes
archive_cache: ~/path/to/my/cache
```

## Weidu

Installation uses
 - either a weidu executable that can be discovered on the `PATH`
 - Or one weidu executable in the same location modda is run from (game directory)
 - or, if present, any executable set in the config file, as `weidu_path`

## Logs

Each mod produces a `setup-<mod identifier>.log` log file.
Multiple run of the same mod (for different components at different places in the installation order) will append in the same file.

The log level can be increased

```
# unix-like
RUST_LOG=debug modda install -m ...

# windows
set RUST_LOG=debug
modda install -m ...
```

Other things are possible, like different log levels by crate, https://docs.rs/env_logger/latest/env_logger/ for the whole doc.

## RAR (or rare archive formats)

RAR is only supported with an external CLI/console executable.

The actual extractor used can be configured in modda.yml

For me (linux) it will be

```yaml
extractors:
    rar:
        command: unrar-nonfree 
        args: [ "x", "${input}", "${target}" ] # ${input} is replace by the actual archive name, ${target} is the destination directory
```
Obviously "rar" can be replaced by some other extension (7z, bz2, xz) but those are rare as weidu mods (?).
