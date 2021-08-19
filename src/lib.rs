use chrono::prelude::*;
use std::convert::From;
use lazy_static::lazy_static;
use scraper::{Html, Selector, ElementRef};
use tokio::sync::mpsc;
use anyhow::Result;
use tabled::Tabled;

lazy_static! {
    static ref NAME_SELECTOR: Selector = Selector::parse("span.package-snippet__name").unwrap();
    static ref VERSION_SELECTOR: Selector = Selector::parse("span.package-snippet__version").unwrap();
    static ref RELEASE_SELECTOR: Selector = Selector::parse("span.package-snippet__released").unwrap();
    static ref DESCRIPTION_SELECTOR: Selector = Selector::parse("p.package-snippet__description").unwrap();
    static ref DATETIME_SELECTOR: Selector = Selector::parse("time").unwrap();
}

fn unwrap_selector(input: &ElementRef, selector: &Selector) -> String {
    input.select(selector).next().map(|e| e.inner_html()).unwrap_or("".into())
}

fn unwrap_time_selector(input: &ElementRef) -> Option<DateTime<Utc>> {
    input.select(&RELEASE_SELECTOR).next()
        .and_then(|release| release.select(&DATETIME_SELECTOR).next())
        .and_then(|time| time.value().attr("datetime"))
        .and_then(|dt| dt.parse::<DateTime<Utc>>().ok())
}

fn format_date(release: &DateTime<Utc>) -> String {
    release.format("%Y-%m-%d").to_string()
}

#[derive(Debug,Tabled)]
pub struct Package {
    #[header("Name")]
    pub name: String,
    #[header("Version")]
    pub version: String,
    #[header("Released")]
    #[field(display_with="format_date")]
    pub release: DateTime<Utc>,
    #[header("Description")]
    pub description: String,
}

impl From<&ElementRef<'_>> for Package {
    fn from(input: &ElementRef) -> Self {
        let release = unwrap_time_selector(input);
        Package{
            name: unwrap_selector(input, &NAME_SELECTOR),
            version: unwrap_selector(input, &VERSION_SELECTOR),
            release: release.unwrap(),
            description: unwrap_selector(input, &DESCRIPTION_SELECTOR),
        }
    }
}


pub async fn query_pypi(name: String, pages: usize) -> Result<Vec<Package>>{
    let client = reqwest::Client::new();
    let (tx, mut rx) = mpsc::channel(32);

    let package_snippet = Selector::parse("a.package-snippet").unwrap();

    tokio::spawn(async move {
        for page_idx in (1..=pages).map(|i| i.to_string()) {
            let query_params = vec![("q", &name), ("page", &page_idx)];

            let page_body = client.get("https://pypi.org/search/")
                .query(&query_params)
                .send()
                .await;
            tx.send(page_body).await.expect("can send on package channel");
        }
    });

    let mut packages = vec![];

    while let Some(response) = rx.recv().await {
        let page_body = response?.text().await?;
        let page_result = Html::parse_document(&page_body);
        for element in page_result.select(&package_snippet) {
            let package = Package::from(&element);
            packages.push(package);
        }
    }

    Ok(packages)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_package_data_test() {
        let input = r#"
            <a class="package-snippet" href="/project/gitlab3/">
            <h3 class="package-snippet__title">
              <span class="package-snippet__name">gitlab3</span>
              <span class="package-snippet__version">0.5.8</span>
              <span class="package-snippet__released"><time datetime="2017-03-18T19:38:52+0000" data-controller="localized-time" data-localized-time-relative="true" data-localized-time-show-time="false" title="2017-03-18 20:38:52" aria-label="2017-03-18 20:38:52">Mar 18, 2017</time></span>
            </h3>
            <p class="package-snippet__description">GitLab API v3 Python Wrapper.</p>
          </a>"#;
        let page = Html::parse_fragment(input);
        let snippet = page.root_element();
        let package = Package::from(&snippet);

        assert_eq!(package.name, "gitlab3");
        assert_eq!(package.version, "0.5.8");
        assert_eq!(package.release, "2017-03-18T19:38:52+0000".parse::<DateTime<Utc>>().unwrap());
        assert_eq!(package.description, "GitLab API v3 Python Wrapper.");
    }
}
