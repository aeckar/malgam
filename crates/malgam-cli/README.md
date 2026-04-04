# `malgam-cli`: Malgam Command-Line Utility

This crate contains the Malgam command-line application, `mal`. The program offers various subcommands to perform operations on Malgam documents or object notation.

- `mal build <PATH>?` build to html, path or file
  - `-o`
  - if served, and uptodate, take from cache
- `mal fmt <PATH>?` format all, path or file

- `mal serve <PATH>?` build to html & host live local server, path or file

- `mal get <NAME>` get and update dependencies

- `mal bin`
- `mal init`
- `mal docs <NAME>`
- `mal remove|rm <NAME>`
- `mal list|ls`

- `mal conv <TO> <PATH>`
    - TO = [md, mal, malo, json, yaml, yml]

## Malgam Document (`.mal`)

conf.malo
