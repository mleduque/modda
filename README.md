# Modda

Automates the installation of a bunch of mods above an Infinity Engine based game.
(though so far, it was only tested with BG1 & BG2).

## Initial goal

Be able to generate (mostly) reproducible installations

## Operation

You create a recipe that 
 - decides which language(s) will be selected
 - lists all mods that should be installed, in order, and of the components for each mod

## Limitations

- Needs weidu accessible on the path
- Only tested with weidu 247
- At this point, was only tested on linux

## Todo

- Implement HTTP fetching of mods
- Warn if the version of a mod changed ; this may impact reproductibility (component number changing, new components etc.)
