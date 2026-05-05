# steamer

A CLI-tool to automatically fetch and download SteamGridDB for your non-steam games.
Use `--interactive` flag to pick and choose which steamgrid game to use, otherwise it will always pick the first one.
Set the `STEAMGRID_API_KEY` environmental variable with your own API key in order to use this tool.

Downloads icon, grid, hero and logo for each game. Skips the game if any are not available.

Tested on linux, other platforms untested.
