# Modda

Automates the installation of a bunch of mods above an Infinity Engine based game.
(though so far, it was only tested with BG1 & BG2).

It depends on weidu to be accessible somewhere on the computer.

## Initial goal

Be able to generate (mostly) reproducible installations.

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

## Generating from weidu.log

It's possible to generate a skeleton YAML file from an existing `weidu.log` file.

From the gamedirectory (where weidu.log is located):

```
modda reverse --output my-install.yaml
```
with a weidu.log that looks like that
```
// Log of Currently Installed WeiDU Mods
// The top of the file is the 'oldest' mod
// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]
~FAITHS_AND_POWERS/FAITHS_AND_POWERS.TP2~ #0 #25 // Choosee a Sphere System -> nuFnP: a new sphere system (fewer spheres, more balanced, closer to PnP): 0.85sd19
~FAITHS_AND_POWERS/FAITHS_AND_POWERS.TP2~ #0 #31 // Install Cleric kits: 0.85sd19
~FAITHS_AND_POWERS/FAITHS_AND_POWERS.TP2~ #0 #33 // Install Druid kits: 0.85sd19
```

will generate
```yaml
version: '1'
global:
  lang_dir: fr_fr
  lang_preferences:
  - '#rx#^fran[cç]ais'
  - french
modules:
- name: faiths_and_powers
  components:
  - index: 25
    component_name: 'Choosee a Sphere System -> nuFnP: a new sphere system (fewer spheres, more balanced, closer to PnP): 0.85sd19'
  - index: 31
    component_name: 'Install Cleric kits: 0.85sd19'
  - index: 33
    component_name: 'Install Druid kits: 0.85sd19'
```

The `component_name` properties are actually just like comments (they would be ignored in an `install` operation).

The `lang_dir` property is taken from `weidu.conf` and `lang_preferences` is just guessed (for a limited set of languages, `en`, `fr` and `es` ATM).

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
It's possible to give a name to the resulting archive.

```yaml
    location:
      http: http://www.shsforums.net/files/download/710-xulaye/
      rename: Xulaye_v2.0.zip
```

### Example 2: Github fetch

You can specify a `release`/`asset` pair, a `tag`, a `commit` hash or (not really recommended) a `branch`.

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

- At this point, was only tested on linux and (a little less) windows.
- Mods that use `ACTION_READLN` are not handled well (installation is interrupted until the user makes some choice, and reproducibility is not guaranteed).To work around this, I usually `patch` those commands out.

## Modifying mode

There are some ways to changea mod after it's been copied in the game directory (unarchive+copy) and before executing installation (weidu phase).

WARNING: If you use these capabilities, please don't bother the mod authors/maintainers with questions (except maybe the `add_conf` method). Check with an unmodified mod first.

### Patching`

Apply a patch in "unified diff" format to the mod.

It assumes the patched files are text files UTF-8 encoded but can be forced to use (some) other encodings.

``` yaml
- name: my_mod
    location:
        http: https://whatever.org/path/my_mod.zip
        patch:
            relative: patches/my_mod-remove-action_readln.diff
- name: other_mod_in_win1252
    location:
        http: https://whatever.org/path/my_mod.zip
        patch:
            relative: patches/my_mod-remove-action_readln.diff
            encoding: WIN1252 # UTF8 / WIN1252 / WIN1251
```

With my_mod-remove-action_readln.diff

```diff
--- my_mod/lib/my_lib.tpa
+++ my_mod/lib/my_lib.tpa
@@ -70,13 +70,7 @@
 <some patch context line>
 <yet more patch context>
 
-PRINT ~Choose a portrait:~
-
-PRINT ~Please choose one of the following:
-[1] Default
-[2] Alt portrait~
-
-OUTER_SPRINT ~portrait~ ~placeholder_value~
+OUTER_SPRINT ~portrait~ 1
-OUTER_WHILE (!(IS_AN_INT ~portrait~) OR (~portrait~ > 0x2) OR (~portrait~ < 0x1)) BEGIN
-  ACTION_READLN ~portrait~
-END
```

"Relative" patches are searched
- in the same directory as the YAML manifest file if `global.local_patches` is not defined
- in `${manifest_directory}/${local_patches}` if `local_patches` is defined

### Regex replace

instead of the `patch` property of `location`, this uses a ` replace` property which is _a list_  of "replace operations"

```yaml
    location:
      http: https://somewhere.under/the-rainbow.zip
      replace:
        # this timer is far too long!
        - file_globs: ["the-rainbow/dialogue/who_s_this.d"]
          replace: 'RealSetGlobalTimer("MyLongTimer","GLOBAL",7200)'
          with: 'RealSetGlobalTimer("MyLongTimer","GLOBAL",3600)'
        - file_globs: ["the-rainbow/scripts/script.baf", "the-rainbow/scripts/script2.baf"]
          replace: "something else"
          with: "something better"
```

- `file_globs` is a list of "globs" (the best description I could find is the one in the 
- [gitignore documentaition](https://git-scm.com/docs/gitignore#_pattern_format))
- `replace` is a regexp in the [Rust regex crate format](https://docs.rs/regex/latest/regex/#syntax) (**Not the Weidu regex format**), which tells _what_ will be replaced
- `with` is a replacement string which tell _with what_ it will be replaced (maybe including capture groups).

## Adding a single file
Use the mod `add_conf` property to add a single file in the mod directory.

```yaml
- name: EET
  components: [0]
  location:
    http: ...
  add_conf:
    file_name: bgee_dir.txt
    content: /path/>to/>my_bg1ee/installation
- name: cdtweaks
  components: ask
  location:
    http: ...
  add_conf:
    file_name: cdtweaks.txt
    content: |
      OUTER_SET always_install_unique_icons = 1
      OUTER_SET romance_use_config_values = 1
      OUTER_SET romance_speed_factor = 67
```

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

This uses a configuration file with one single configuration property (at the moment).
The file is name `modda.yml` and will be taken from the current directory (first) then from the "OS conventional location for application configuration.

- on linux, it should be `~/.config/modda/modda.yml`
- on windows, should be around `C:\Users\<username>\AppData\Roaming\modda\config/modda.yml`
- on macos, something like `$HOME/Library/Application Support/modda/modda.yml`

Properties:
- `archive_cache` which tells the program where to store and search for downloaded mod archives.
- `extract_location` the temporary place where archive are extracted before being copied to the game directory (using a place on the same file system as the game directory can provide some performance advantage)
- `weidu_path` where weidu executable can be found
- `extractors` tells howto extract some archive format with an external program

All properties are optional.

```yaml
# can be an absolute path, or can use ~ exapansion on UNIX-like OSes
archive_cache: ~/path/to/my/cache
```
## Authenticated github downloads

It is possible to download from a private repository.

- create a _Personal Access Token_ , see https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-personal-access-token-classic
- create a `modda-credentials.yml` file in the configuration location (the same location as `modda.yml`` file)

```yaml
github:
    personal_tokens:
        my_repositories: XXXXX # copied from github
```

Then the `location` property of the mod needs to tell which token must be used.

```yaml
mods:
  - name: MysteriousMod
    components:
      - 0
    location:
      github_user: Myself
      repository: my_private_repo
      release: V1.0
      asset: my_private_asset.zip
      auth: PAT my_repositories
```

## Weidu

Installation uses
 - either a weidu executable that can be discovered on the `PATH`
 - Or one weidu executable in the same location modda is run from (game directory)
 - or, if present, any executable set in the config file, as `weidu_path`

Modda will not use the various setup-XXX.exe that clutter the game directory (and are just `weidu.exe` duplicates with different versions).

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

Tested on windows with

```yaml
extractors:
  rar:
    command: 'C:\Program Files\7-Zip\7z.exe'
    args: [ "x", "${input}", "-o${target}" ]
```

Obviously "rar" can be replaced by some other extension (7z, bz2, xz) but those are rare as weidu mods (I think?).

## Building
TODO Describe installing rustup, installing the compiler components and running cargo build
