# steamer

A CLI-tool to automatically fetch and download SteamGridDB for your non-steam games.  
Downloads icon, grid, hero and logo for each game. Skips the game if any are not available.

Tested on linux, other platforms untested but should work.

## Installation

Install it via `cargo` the Rust package manager:

```sh
cargo install --git https://github.com/kaezrr/steamer.git
```

## Usage

```sh
Usage: steamer [OPTIONS] --api-key <API_KEY>

Options:
      --api-key <API_KEY>  Your SteamGridDB API key
  -i, --interactive        Interactively choose which SteamGridDB game to pick
  -o, --overwrite          Overwrite all existing assets and refetch them
  -h, --help               Print help
```

By default it always picks the first match for icons, heroes, grids and logos.

## Possible Improvements

- Extend it to work on normal steam games
- ~Add the option to preserve existing steamgrid assets instead of always overwriting~
- Add configuration file to save api key and other configuration options for covers, heroes, etc
- Integrate OS file events and add a `--watch` so that it runs automatically in the background efficiently
- Possible further parallelization improvements to make it work even faster
