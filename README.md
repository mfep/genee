# genee
[![Continuous integration](https://github.com/mfep/genee/actions/workflows/continuous-integration.yml/badge.svg)](https://github.com/mfep/genee/actions/workflows/continuous-integration.yml)

genee is a simple habit tracker program for the command line

## Features
- Daily tracking of habits 📅
- Open storage format: simple CSV files 📄
- Pretty diagrams to compare successive periods of habit data 📊
- Store default settings persistently 💾

## Workflow

1. Figure out the list of habits to track. In this example, we would like to restrict our gaming binges and increase the frequency of our piano exercise sessions.
`GAM` stands for gaming, whereas `PNO` stands for the instrumental practice.
2. Download the [latest release](https://github.com/mfep/genee/releases/latest) from this repository.
3. Using the downloaded executable, create a new data file to store the diary data. Specify the list of habit abbreviations to use in this file:
```genee --new GAM,PNO```
4. Each day, fill in whether you practiced the particular habits the previous day or not. This can be done by invoking
```genee -f```. This is followed by a prompt for each habit.
5. After the fill command, the program displays the habit data of the current period (e.g. the number of habit occurences in the last 30 days)
compared to the last period (the number of habit occurences between 30 and 60 days before now). This can be used to check whether our change of habits
(picking up new habits, dropping bad ones) are on track or not.

### Example output diagram
![](https://user-images.githubusercontent.com/12499658/121962015-72212600-cd68-11eb-82fb-30279566b220.png)

## Full helptext

```
genee X.Y.Z
A habit tracker app with command-line interface

USAGE:
    genee.exe [FLAGS] [OPTIONS]

FLAGS:
    -f, --fill           If set, habit information for all the missing days is queried between --append-date and
                         yesterday. If --append-date is not set, all the missing days are queried between the first
                         entry in the diary and yesterday
    -h, --help           Prints help information
        --list-config    If set, the current persistent configuration is displayed to the terminal
        --save-config    If set, the provided values for --datafile --graph-days --past-periods --max-displayed-cols and
                         --list-previous-days options are written to the persistent configuration. Unspecified options
                         are reset to their default value
    -V, --version        Prints version information

OPTIONS:
    -a, --append-date <append-date>
            When provided, the habit data is queried and written to the diary at the specified date. The format of the
            date must be YYYY-MM-DD. If --fill is also set, this option serves a different purpose
    -d, --datafile <datafile>
            Path to the diary file. When not provided, its value is loaded from persistent configuration file

    -g, --graph-days <graph-days>
            How many days each period should contain. When not provided, its value is loaded from persistent
            configuration file
    -l, --list-previous-days <list-previous-days>
            Specifies the number of days from the diary that should be printed in a tabular format

        --max-displayed-cols <max-displayed-cols>
            Specifies the maximum allowed width of the terminal output. When not provided, its value is loaded from
            persistent configuration file
        --new <new>
            Provide a comma separated list of habit categories. A new diary file is created at the specified --datafile
            path. Be aware that this overwrites any existing diary file
    -p, --past-periods <past-periods>
            Specifies the number of displayed periods when graphing the diary data. When not provided, its value is
            loaded from persistent configuration file
```

## Building

genee builds with the standard Rust toolchain:

```
git clone https://github.com/mfep/genee.git
cd genee
cargo build --release
```

## Contribution

genee is a toy project for my own use and education. Feature request and bug reports are much appreciated though.
