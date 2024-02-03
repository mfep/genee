use anyhow::{bail, Context, Result};
use chrono::{Duration, Local, NaiveDate};
use dialoguer::MultiSelect;
use genee::{configuration, datafile, graphing};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod ui;

#[derive(StructOpt)]
#[structopt(about)]
struct CliOptions {
    /// Path to the diary file.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long, parse(from_os_str))]
    datafile: Option<PathBuf>,

    /// How many days each period should contain.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long)]
    graph_days: Option<usize>,

    /// Specifies the number of displayed periods when graphing the diary data.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long)]
    past_periods: Option<usize>,

    /// Specifies the number of days from the diary that should be printed in a tabular format.
    #[structopt(short, long)]
    list_previous_days: Option<usize>,

    /// Specifies the number of most frequent daily habit compositions over the specified period.
    #[structopt(short = "f", long)]
    list_most_frequent_days: Option<usize>,

    /// Specifies the maximum allowed width of the terminal output.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(long)]
    max_displayed_cols: Option<usize>,

    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    /// If set, habit information for all the missing days is queried between --from-date
    /// and yesterday. If --from-date is not set, all the missing days are queried between the
    /// first entry in the diary and yesterday.
    /// If there is no entry in the diary, only yesterday is queried.
    Fill {
        /// Start the querying from this date. Must be in format YYYY-MM-DD.
        #[structopt(long)]
        from_date: Option<String>,

        /// Controls whether to graph the diary after the fill.
        #[structopt(long)]
        no_graph: bool,
    },

    /// Displays the habit data according to the specified options to the terminal.
    Graph {
        /// Start the graphing from this date. Must be in format YYYY-MM-DD.
        #[structopt(long)]
        from_date: Option<String>,
    },

    /// Queries for habit information on the specified date.
    Insert {
        /// Date to set habit data on.  Must be in format YYYY-MM-DD.
        date: String,

        /// Controls whether to graph the diary after the fill.
        #[structopt(long)]
        no_graph: bool,
    },

    /// Prints the persistent configuration.
    ListConfig,

    /// Provide a comma separated list of habit categories. A new diary file is created at the specified
    /// --datafile path.
    New { category_list: String },

    /// Saves the specified options to persistent configuration.
    SaveConfig,

    /// Writes the contents of the datafile into a new datafile. Useful to convert between formats.
    Export { exported_path: PathBuf },

    /// Adds or unhides a category.
    AddCategory { name: String },

    /// Hides a category.
    HideCategory { name: String },
}

fn main() -> Result<()> {
    let opt = handle_config()?;
    let datafile_path = opt.datafile.as_ref().unwrap();
    match opt.command {
        Some(Command::Fill {
            ref from_date,
            no_graph,
        }) => {
            let from_date = parse_from_date(from_date)?;
            let mut data = datafile::open_datafile(datafile_path)?;
            let to_date = get_graph_date(&*data)?;
            fill_datafile(&from_date, &to_date, &mut *data)?;
            if !no_graph {
                plot_datafile(&opt, &to_date, &*data)?;
            }
        }
        Some(Command::Graph { ref from_date }) => {
            let data = datafile::open_datafile(datafile_path)?;
            let from_date = parse_from_date(from_date)?;
            let from_date = from_date.unwrap_or_else(|| Local::now().naive_local().date());
            plot_datafile(&opt, &from_date, &*data)?;
        }
        Some(Command::Insert { ref date, no_graph }) => {
            let date = parse_from_date(&Some(date.clone()))?.unwrap();
            let mut data = datafile::open_datafile(datafile_path)?;
            insert_to_datafile(&date, &mut *data)?;
            if !no_graph {
                plot_datafile(&opt, &date, &*data)?;
            }
        }
        Some(Command::ListConfig) => {
            let persistent_config = configuration::load_config()?;
            println!(
                "Listing persistent configuration loaded from '{}'\n{}",
                configuration::get_config_path().to_string_lossy(),
                &persistent_config
            );
        }
        Some(Command::New { ref category_list }) => {
            create_new(datafile_path, category_list)?;
        }
        Some(Command::SaveConfig) => {
            save_config(&opt)?;
        }
        Some(Command::Export { ref exported_path }) => {
            export_datafile(datafile_path, exported_path)?;
        }
        Some(Command::AddCategory { ref name }) => {
            add_category(datafile_path, name)?;
        }
        Some(Command::HideCategory { ref name }) => {
            hide_category(datafile_path, name)?;
        }
        None => {
            ui::run_app(&opt)?;
        }
    }

    Ok(())
}

fn handle_config() -> Result<CliOptions> {
    let opt = CliOptions::from_args();
    let persistent_config = configuration::load_config()?;
    let opt = merge_cli_and_persistent_options(opt, &persistent_config);
    Ok(opt)
}

fn parse_from_date(input_date: &Option<String>) -> Result<Option<NaiveDate>> {
    match input_date {
        Some(date_string) => Ok(Some(
            NaiveDate::parse_from_str(date_string, datafile::DATE_FORMAT).context(format!(
                "Could not parse input date \"{}\". Use the format {}",
                date_string,
                datafile::DATE_FORMAT
            ))?,
        )),
        None => Ok(None),
    }
}

fn get_graph_date(data: &dyn datafile::DiaryDataConnection) -> Result<NaiveDate> {
    let today = Local::now().naive_local().date();
    if data.get_row(&today)?.is_some() {
        Ok(today)
    } else {
        Local::now()
            .naive_local()
            .date()
            .checked_sub_signed(chrono::Duration::days(1))
            .ok_or_else(|| anyhow::Error::msg("Could not get yesterday"))
    }
}

fn merge_cli_and_persistent_options(
    options_from_cli: CliOptions,
    persistent_config: &configuration::Config,
) -> CliOptions {
    CliOptions {
        datafile: options_from_cli
            .datafile
            .or_else(|| Some(persistent_config.datafile_path.clone())),
        graph_days: options_from_cli
            .graph_days
            .or(Some(persistent_config.graph_days)),
        past_periods: options_from_cli
            .past_periods
            .or(Some(persistent_config.past_periods)),
        max_displayed_cols: options_from_cli
            .max_displayed_cols
            .or(Some(persistent_config.max_displayed_cols)),
        list_previous_days: options_from_cli
            .list_previous_days
            .or(Some(persistent_config.list_previous_days)),
        list_most_frequent_days: options_from_cli
            .list_most_frequent_days
            .or(Some(persistent_config.list_most_frequent_days)),
        ..options_from_cli
    }
}

fn save_config(opt: &CliOptions) -> Result<()> {
    let provided_datafile_path = opt
        .datafile
        .clone()
        .unwrap_or_else(configuration::get_default_datafile_path);
    let full_datafile_path = std::fs::canonicalize(provided_datafile_path.clone());
    if full_datafile_path.is_err() {
        println!("Cannot canonicalize provided datafile path, saving the uncanonicalized path to configuration");
    }
    let updated_config = configuration::Config {
        datafile_path: full_datafile_path.unwrap_or(provided_datafile_path),
        graph_days: opt.graph_days.unwrap_or(configuration::DEFAULT_GRAPH_DAYS),
        past_periods: opt
            .past_periods
            .unwrap_or(configuration::DEFAULT_PAST_PERIODS),
        max_displayed_cols: opt
            .max_displayed_cols
            .unwrap_or(configuration::DEFAULT_MAX_DISPLAYED_COLS),
        list_previous_days: opt
            .list_previous_days
            .unwrap_or(configuration::DEFAULT_LIST_PREVIOUS_DAYS),
        list_most_frequent_days: opt
            .list_most_frequent_days
            .unwrap_or(configuration::DEFAULT_LIST_MOST_FREQUENT_DAYS),
    };
    configuration::save_config(&updated_config)?;
    println!("Successfully updated persistent configuration");
    Ok(())
}

fn input_day_interactively(
    data: &mut dyn datafile::DiaryDataConnection,
    date: &NaiveDate,
) -> Result<()> {
    let prompt = format!(
        "Enter habit data for date {}",
        date.format(datafile::DATE_FORMAT)
    );
    let header = data.get_header()?;
    let header_names: Vec<String> = header.iter().map(|(name, _id)| name.clone()).collect();
    let selected_indices = MultiSelect::new()
        .with_prompt(prompt)
        .items(&header_names)
        .interact()?;

    let selected_ids: Vec<usize> = selected_indices
        .into_iter()
        .map(|idx| header.get(idx).unwrap().1)
        .collect();

    data.update_data(date, &selected_ids)?;
    Ok(())
}

fn fill_datafile(
    from_date: &Option<NaiveDate>,
    to_date: &NaiveDate,
    data: &mut dyn datafile::DiaryDataConnection,
) -> Result<()> {
    let mut missing_dates = data.get_missing_dates(from_date, to_date)?;
    if data.is_empty()? {
        missing_dates.push(
            Local::now()
                .naive_local()
                .date()
                .checked_sub_signed(Duration::days(1))
                .unwrap(),
        );
    }
    for date in missing_dates {
        input_day_interactively(data, &date)?;
    }
    Ok(())
}

fn insert_to_datafile(
    date: &NaiveDate,
    data: &mut dyn datafile::DiaryDataConnection,
) -> Result<()> {
    input_day_interactively(data, date)?;
    Ok(())
}

fn plot_datafile(
    opt: &CliOptions,
    last_date: &NaiveDate,
    data: &dyn datafile::DiaryDataConnection,
) -> Result<()> {
    if opt.list_previous_days.unwrap() > 0 {
        let start_day =
            *last_date - chrono::Duration::days(opt.list_previous_days.unwrap() as i64 - 1i64);
        print!(
            "{}",
            graphing::pretty_print_diary_rows(data, &start_day, last_date)?,
        );
    }
    if opt.list_most_frequent_days.unwrap() > 0 {
        print!(
            "{}",
            graphing::pretty_print_most_frequent_day_types(
                data,
                opt.graph_days.unwrap() * opt.past_periods.unwrap(),
                last_date,
                opt.list_most_frequent_days.unwrap()
            )?
        );
    }
    graphing::graph_last_n_days(
        data,
        last_date,
        opt.graph_days.unwrap(),
        opt.past_periods.unwrap(),
        opt.max_displayed_cols.unwrap(),
    )?;
    Ok(())
}

fn create_new(path: &Path, headers_string: &str) -> Result<()> {
    let mut headers_vector = vec![];
    for title in headers_string.split(',') {
        if title.is_empty() {
            bail!(format!("Invalid header specification: {}", headers_string));
        }
        headers_vector.push(String::from(title));
    }
    datafile::create_new_datafile(path, &headers_vector)?;
    println!("New datafile successfully created at {}", path.display());
    Ok(())
}

fn export_datafile(path: &Path, exported_path: &Path) -> Result<()> {
    if path == exported_path {
        bail!("Cannot export datafile to the source path")
    }

    // Read all rows from the current datafile to memory
    let data = datafile::open_datafile(path)?;
    let (min_date, max_date) = data.get_date_range()?;
    let mut rows = vec![];
    let mut current_date = min_date;
    while current_date <= max_date {
        let current_row_opt = data.get_row(&current_date)?;
        if let Some(current_row) = current_row_opt {
            rows.push((current_date, current_row));
        }
        current_date += chrono::Duration::days(1);
    }

    // Create and update new datafile
    let headers: Vec<String> = data
        .get_header()?
        .into_iter()
        .map(|(name, _id)| name)
        .collect();
    datafile::create_new_datafile(exported_path, &headers)?;
    let mut new_data = datafile::open_datafile(exported_path)?;
    new_data.update_data_batch(&rows)?;

    Ok(())
}

fn add_category(datafile_path: &Path, name: &str) -> Result<()> {
    let datafile = datafile::open_datafile(datafile_path)?;
    match datafile.add_category(name)? {
        datafile::AddCategoryResult::AddedNew => {
            println!("Added new category \"{}\"", name);
        }
        datafile::AddCategoryResult::AlreadyPresent => {
            bail!(
                "Category \"{}\" was already present and shown in the datafile",
                name
            );
        }
        datafile::AddCategoryResult::Unhide => {
            println!("Previously hidden category is showed again: \"{}\"", name)
        }
    }
    Ok(())
}

fn hide_category(datafile_path: &Path, name: &str) -> Result<()> {
    let datafile = datafile::open_datafile(datafile_path)?;
    match datafile.hide_category(name)? {
        datafile::HideCategoryResult::AlreadyHidden => {
            bail!("Category \"{}\" was already hidden", name)
        }
        datafile::HideCategoryResult::NonExistingCategory => {
            bail!("Category \"{}\" does not exist", name)
        }
        datafile::HideCategoryResult::Hidden => {
            println!("Category \"{}\" is hidden", name);
        }
    }
    Ok(())
}
