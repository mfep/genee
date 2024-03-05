# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.9.0] - Unreleased

### Added

- Added an interactive user interface to edit and visualize habit data. It is accessed via tha main `genee` command (i.e. no subcommand is needed)

### Removed

- Removed redundant non-interactive functionality.
  - The following subcommands are not available anymore: `export`, `fill`, `graph`, `insert`
  - The following arguments are not available anymore: `--graph-days`, `--list-previous-days`, `--max-displayed-cols`

## [v0.8.0] - 2024-01-28

### Added

- `list_most_frequent_days` command line option. When greater than 0, the specified number of most frequently occurring daily habit compositions are printed to the terminal. This option can also be saved to and recalled from the persistent configuration.

### Fixed

- Fixed data entry when there are hidden categories

### Removed

- CSV datafile support

## [v0.7.1] - 2024-01-23

### Fixed

- Fixed SQLITE database upgrade routine from v0 to v1
