use anyhow::Result;
use pypi_search;
use tabled::Table;
use structopt::StructOpt;
use clap::arg_enum;

arg_enum! {
    #[derive(Debug)]
    enum SortBy {
        PyPI,
        Date,
        Name
    }
}

/// Search PyPI for packages by name.
#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(short, long)]
    debug: bool,

    #[structopt(name = "NAME")]
    name: String,

    #[structopt(short, long, default_value = "1")]
    pages: usize,

    #[structopt(long)]
    header: bool,

    #[structopt(long, possible_values = &SortBy::variants(), case_insensitive = true, default_value = "PyPI")]
    sort_by: SortBy,
}

#[tokio::main]
async fn main() -> Result<()>{
    let opt = Opt::from_args();
    let mut packages = pypi_search::query_pypi(opt.name.into(), opt.pages).await?;
    match opt.sort_by {
        SortBy::Date => {packages.sort_by(|a, b| b.release.cmp(&a.release))},
        SortBy::Name => {packages.sort_by(|a, b| a.name.cmp(&b.name))},
        _ => {},
    }


    let table = Table::new(&packages)
        .with(tabled::Style::noborder())
        .with(
            tabled::Modify::new(tabled::Full)
                .with(tabled::Alignment::left())
        );
    print!("{}", table);
    Ok(())
}

