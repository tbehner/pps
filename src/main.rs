use anyhow::Result;
use scraper::{Html, Selector};
use chrono::prelude::*;
use pipsearch;
use tokio::sync::mpsc;


#[tokio::main]
async fn main() -> Result<()>{
    let packages = pipsearch::query_pypi(3).await?;
    for package in packages {
        println!("{:?}", package);
    }

    Ok(())
}

