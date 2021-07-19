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

## Limitations

- Needs weidu accessible on the path
- Only tested with weidu 247
- At this point, was only tested on linux
- Mods that use ACTION_READLN are not handled well (installation is interrupted until the user makes some choice, and reproductibility is not guaranteed)

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
- on windows, probably around `%USERPROFILE%\AppData\Roaming\modda\modda.yml` (untested)
- on macos, something like `$HOME/Library/Application Support/modda/modda.yml`

It currently contains one property: `archive_cache` which tells the program where to store and search for downloaded modf archives.

```yaml
# can be an absolute path, or can use ~ exapansion on UNIX-like OSes
archive_cache: ~/path/to/my/cache
```

## Todo

- Parallel HTTP fetching of mods using a pool
- Resume aborted HTTP downloads?
- Document YAML for archive fetching and unpacking (more)
- Warn if the version of a mod changed ; this may impact reproductibility (component number changing, new components etc.)
- Maybe use the `directories` projectdir cachevlue for default location of download cache?
