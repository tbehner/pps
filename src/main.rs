use anyhow::Result;
use pps;
use tabled::Table;
use structopt::StructOpt;
use clap::arg_enum;
use futures::future::join_all;

arg_enum! {
    #[derive(Debug)]
    enum SortBy {
        PyPI,
        Date,
        Name,
        Downloads
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

    #[structopt(long)]
    downloads: bool,

    #[structopt(long, possible_values = &SortBy::variants(), case_insensitive = true, default_value = "PyPI")]
    sort_by: SortBy,
}

#[tokio::main]
async fn main() -> Result<()>{
    let opt = Opt::from_args();
    let mut packages = pps::query_pypi(opt.name.into(), opt.pages).await?;
    let local_packages = pps::get_installed_packages().await?;

    if opt.downloads || matches!(opt.sort_by, SortBy::Downloads) {
        join_all(packages.iter_mut().map(|pkg| pkg.update_downloads())).await;
    }

    for package in packages.iter_mut() {
        match local_packages.iter().find(|lp| lp.name == package.name) {
            Some(local_package) => package.local(&local_package.version),
            None => {}
        }
    }

    match opt.sort_by {
        SortBy::Date => {packages.sort_by(|a, b| b.release.cmp(&a.release))},
        SortBy::Name => {packages.sort_by(|a, b| a.name.cmp(&b.name))},
        SortBy::Downloads => {packages.sort_by(|a, b| b.downloads.cmp(&a.downloads))},
        _ => {},
    }


    let mut table = Table::new(&packages)
        .with(tabled::Style::noborder())
        .with(
            tabled::Modify::new(tabled::Full)
                .with(tabled::Alignment::left())
        );

    if !(opt.downloads || matches!(opt.sort_by, SortBy::Downloads)) {
        table = table.with(tabled::Disable::Column(5..));
    }

    print!("{}", table);
    Ok(())
}

