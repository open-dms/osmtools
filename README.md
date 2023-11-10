# osmtools

CLI for filtering and extracting locality data from OSM files.

```plain
Usage: osmtools [OPTIONS] --in-file <IN_FILE> [COMMAND]

Commands:
  stats  Output statistics about the PBF file
  help   Print this message or the help of the given subcommand(s)

Options:
  -i, --in-file <IN_FILE>    PBF file to read
  -o, --out-file <OUT_FILE>  Path to output file. If unspecified output is written to stdout
  -f, --format <FORMAT>      Output format [default: geojson] [possible values: geojson, raw]
  -q, --query <QUERY>        Query for relations with matching name. (Sub)string or pattern allowed
  -h, --help                 Print help

Output statistics about the PBF file

Usage: osmtools --in-file <IN_FILE> stats [OPTIONS]

Options:
  -a, --all   Show stats for all relations, using minimal filters
  -h, --help  Print help
```
