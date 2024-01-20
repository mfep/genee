# genee

[![Crates.io](https://img.shields.io/crates/v/genee.svg)](https://crates.io/crates/genee)
[![Docs.rs](https://docs.rs/genee/badge.svg)](https://docs.rs/genee)
[![CI](https://github.com/mfep/genee/workflows/CI/badge.svg)](https://github.com/mfep/genee/actions)

genee is a simple habit tracker program for the command line

## Features
- Daily tracking of habits 📅
- Open storage format: SQLite database or simple CSV files 📄
- Pretty diagrams to compare successive periods of habit data 📊
- Store default settings persistently 💾

## Workflow

1. Figure out the list of habits to track. In this example, we would like to restrict
our gaming binges and increase the frequency of our piano exercise sessions.
`GAM` stands for gaming, whereas `PNO` stands for the instrumental practice.
2. Download the [latest release](https://github.com/mfep/genee/releases/latest) from this repository.
3. Using the downloaded executable, create a new data file to store the diary data.
Specify the list of habit abbreviations to use in this file: ```genee new GAM,PNO```
4. Each day, fill in whether you practiced the particular habits the previous day or not.
This can be done by invoking ```genee fill```. This is followed by a prompt for each habit.
5. After the fill command, the program displays the habit data of the current period
(e.g. the number of habit occurences in the last 30 days) compared to the last period
(the number of habit occurences between 30 and 60 days before now).
This can be used to check whether our change of habits (picking up new habits,
dropping bad ones) are on track or not.

### Example output diagram
![](https://user-images.githubusercontent.com/12499658/121962015-72212600-cd68-11eb-82fb-30279566b220.png)

## Full helptext

```
genee X.Y.Z
A habit tracker app with command-line interface

USAGE:
    genee [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --datafile <datafile>
            Path to the diary file. If the file extension is csv, then the file is assumed to be a CSV text file.
            Otherwise it is assumed to be an SQLite database. When not provided, its value is loaded from persistent
            configuration file
    -g, --graph-days <graph-days>
            How many days each period should contain. When not provided, its value is loaded from persistent
            configuration file
    -l, --list-previous-days <list-previous-days>
            Specifies the number of days from the diary that should be printed in a tabular format

        --max-displayed-cols <max-displayed-cols>
            Specifies the maximum allowed width of the terminal output. When not provided, its value is loaded from
            persistent configuration file
    -p, --past-periods <past-periods>
            Specifies the number of displayed periods when graphing the diary data. When not provided, its value is
            loaded from persistent configuration file

SUBCOMMANDS:
    add-category     Adds or unhides a category. Only supported for SQLite datafiles
    export           Writes the contents of the datafile into a new datafile. Useful to convert between formats
    fill             If set, habit information for all the missing days is queried between --from-date and
                     yesterday. If --from-date is not set, all the missing days are queried between the first entry
                     in the diary and yesterday. If there is no entry in the diary, only yesterday is queried
    graph            Displays the habit data according to the specified options to the terminal
    help             Prints this message or the help of the given subcommand(s)
    hide-category    Hides a category. Only supported for SQLite datafiles
    insert           Queries for habit information on the specified date
    list-config      Prints the persistent configuration
    new              Provide a comma separated list of habit categories. A new diary file is created at the
                     specified --datafile path
    save-config      Saves the specified options to persistent configuration
```

## Building

genee builds with the standard Rust toolchain:

```
git clone https://github.com/mfep/genee.git
cd genee
cargo build --release
```

## Contribution

See [CONTRIBUTING.md](CONTRIBUTING.md).
