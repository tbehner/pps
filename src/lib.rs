use std::fmt;
use chrono::prelude::*;
use std::convert::From;
use lazy_static::lazy_static;
use scraper::{Html, Selector, ElementRef};
use tokio::sync::mpsc;
use tokio::process::Command;
use anyhow::Result;
use tabled::Tabled;
use serde::{Serialize, Deserialize};
use thousands::Separable;
use backoff::ExponentialBackoff;
use backoff::future::retry;
use std::cmp::Ordering;

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

#[derive(Serialize,Deserialize,Debug,PartialEq,Eq)]
pub struct Downloads {
    last_day: u64,
    last_week: u64,
    last_month: u64,
}

impl fmt::Display for Downloads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} / {} / {}", 
            self.last_day.separate_with_commas(), 
            self.last_week.separate_with_commas(), 
            self.last_month.separate_with_commas()
            )
    }
}

impl Ord for Downloads {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last_month.cmp(&other.last_month)
    }
}

impl PartialOrd for Downloads {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


#[derive(Serialize,Deserialize)]
struct DownloadsResponse {
    data: Downloads,
    package: String,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    response_type: String,
}


#[derive(Debug, PartialEq)]
pub struct LocalPackage {
    pub name: String,
    pub version: String,
}

impl LocalPackage {
    fn from_str(input: &str) -> Result<LocalPackage> {
        let fields: Vec<&str> = input.split_whitespace().collect();
        Ok(LocalPackage{
            name: fields[0].into(), 
            version: fields[1].into()
        })
    }
}

fn display_installed(possible_version: &Option<String>) -> String {
    possible_version
        .as_ref()
        .unwrap_or(&String::from(""))
        .into()
}

fn display_downloads(possible_downloads: &Option<Downloads>) -> String {
    match possible_downloads {
        Some(d) => format!("{}", d),
        None => "".into()
    }
}

#[derive(Debug,Tabled)]
pub struct Package {
    #[header("Name")]
    pub name: String,
    #[header("Installed")]
    #[field(display_with="display_installed")]
    pub installed: Option<String>,
    #[header("Version")]
    pub version: String,
    #[header("Released")]
    #[field(display_with="format_date")]
    pub release: DateTime<Utc>,
    #[header("Description")]
    pub description: String,
    #[header("Downloads")]
    #[field(display_with="display_downloads")]
    pub downloads: Option<Downloads>,
}

impl From<&ElementRef<'_>> for Package {
    fn from(input: &ElementRef) -> Self {
        let release = unwrap_time_selector(input);
        Package{
            name: unwrap_selector(input, &NAME_SELECTOR),
            installed: None,
            version: unwrap_selector(input, &VERSION_SELECTOR),
            release: release.unwrap(),
            description: unwrap_selector(input, &DESCRIPTION_SELECTOR),
            downloads: None,
        }
    }

}

async fn get_with_retry(url: &str) -> Result<String> {
     Ok(retry(ExponentialBackoff::default(), || async {
            let body = reqwest::get(url)
                .await?
                .error_for_status()?
                .text()
                .await?;
            Ok(body)
        }).await?)
}

impl Package {
    pub fn local(&mut self, version: &str) {
        self.installed = Some(version.into())
    }

    pub async fn update_downloads(&mut self) {

        let url = format!("https://pypistats.org/api/packages/{}/recent", self.name);
   
        let body: String = get_with_retry(&url).await.unwrap();
        let data: DownloadsResponse = serde_json::from_str(&body).unwrap();
        self.downloads = Some(data.data);
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

pub async fn get_installed_packages() -> Result<Vec<LocalPackage>> {
    let output = Command::new("pip")
        .arg("list")
        .arg("installed")
        .output()
        .await?;

    let stdout: String = String::from_utf8(output.stdout)?;

    let mut out = vec![];
    for line in stdout.lines().skip(2) {
        out.push(LocalPackage::from_str(line)?)
    }
    Ok(out)
}

pub async fn get_downloads(package_name: &str) -> Result<Downloads> {
    let url = format!("https://pypistats.org/api/packages/{}/recent", package_name);
    let data: DownloadsResponse = reqwest::get(&url)
        .await?
        .json()
        .await?;
    Ok(data.data)
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

    #[tokio::test]
    async fn get_installed_packages_test() {
       let installed_packages = get_installed_packages().await.unwrap();
       let pip_package = LocalPackage{ name: "pip".into(), version: "21.1.2".into() };
       assert!(installed_packages.contains(&pip_package));
    }

    #[tokio::test]
    async fn get_downloads_test() {
        let package_name = "python-gitlab";
        let downloads = get_downloads(package_name).await.unwrap();
        assert!(downloads.last_month > 0);
    }
}
