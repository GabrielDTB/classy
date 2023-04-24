use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeSet;

#[tokio::main]
async fn main() -> Result<()> {
    let mut courses = Vec::new();
    let links = get_course_links().await?;
    let client = Client::new();
    let bar = ProgressBar::new(links.len().try_into().unwrap());
    for link in links {
        courses.push(
            get_course(&link.to_lowercase(), &client)
                .await
                .context(format!("in parsing of {}", link))?,
        );
        bar.inc(1);
        //println!("{:?}", courses.last().unwrap());
    }
    bar.finish();
    tokio::fs::write(
        "courses.json",
        serde_json::to_string_pretty(&courses).unwrap(),
    )
    .await?;

    Ok(())
}

async fn get_course_links() -> Result<BTreeSet<String>> {
    let mut courses = BTreeSet::new();
    let url = "https://stevens.smartcatalogiq.com/Institutions/Stevens-Institution-of-Technology/json/2022-2023/Academic-Catalog.json";
    let response = reqwest::get(url).await?;
    let l1 = &response.json::<serde_json::Value>().await?["Children"][23];
    for c1 in Counter::new(0) {
        let l2 = &l1["Children"][c1];
        if l2.is_null() {
            break;
        }
        for c2 in Counter::new(0) {
            let l3 = &l2["Children"][c2];
            if l3.is_null() {
                break;
            }
            for c3 in Counter::new(0) {
                let course = match &l3["Children"][c3] {
                    value => match value.is_null() {
                        true => break,
                        _ => value,
                    },
                };
                courses.insert(
                    "https://stevens.smartcatalogiq.com/en".to_string()
                        + course["Path"]
                            .as_str()
                            .context("\"Path\" field missing from course")?,
                );
            }
        }
    }
    Ok(courses)
}

async fn get_course(link: &str, client: &Client) -> Result<Course> {
    let response = client.get(link).send().await?.text().await?;
    let html = Html::parse_document(&response);
    let element = match html
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("main"))
    {
        Some(value) => value,
        _ => bail!(
            "page did not have an element of the required selector\n{:#?}",
            response
        ),
    };
    let flatten = regex::Regex::new(r"\s+").unwrap();
    let id = element
        .select(&Selector::parse("h1").unwrap())
        .next()
        .context("no elementt matched the first selector in name parsing")?
        .text()
        .nth(1)
        .context("last element not found in name parsing")?
        .trim()
        .to_string();
    let name = element
        .select(&Selector::parse("h1").unwrap())
        .next()
        .context("no elementt matched the first selector in name parsing")?
        .text()
        .last()
        .context("last element not found in name parsing")?
        .trim()
        .to_string();
    let description = element
        .select(&Selector::parse("div").unwrap())
        //println!("{}", serde_json::to_string_pretty(&courses).unwrap());
        .find(|element| element.value().attr("class") == Some("desc"))
        .context("no elementt matched the first attribute in description parsing")?
        .text()
        .last()
        .context("last element not found in description parsing")?
        .replace("\n", " ")
        .replace("\t", " ");
    let description = flatten.replace_all(&*description, " ").trim().to_string();
    let credits = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_credits"))
        .context("no elementt matched the first attribute in credits parsing")?
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("credits"))
        .context("no element matched the second attribute in credits parsing")?
        .text()
        .last()
        .context("last element not found in credits parsing")?
        .trim()
        .to_string();
    let prerequisites = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_prereqs"))
        .context("no element matched the first attribute in prerequisites parsing")?
        .text()
        .last()
        .context("last element not found in prerequisites parsing")?
        .trim()
        .to_string();
    let distribution = match element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("distribution"))
    {
        Some(value) => value
            .text()
            .last()
            .context("last element not found in distribution parsing")?
            .split(|c| c == '\n' || c == '\t')
            .map(|s| s.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect::<BTreeSet<String>>(),
        _ => BTreeSet::new(),
    };
    let offered = match element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("offered"))
    {
        Some(e) => e
            .text()
            .last()
            .context("last element not found in offered parsing")?
            .split(|c| c == '\n' || c == '\t')
            .map(|s| s.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect::<BTreeSet<String>>(),
        _ => BTreeSet::new(),
    };

    Ok(Course {
        id,
        name,
        description,
        credits,
        prerequisites,
        offered,
        distribution,
        link: link.to_owned(),
    })
}

struct Counter {
    current: usize,
}

impl Counter {
    fn new(start: usize) -> Counter {
        Counter { current: start }
    }
}

impl Iterator for Counter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current += 1;
        Some(current)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Course {
    pub id: String,
    pub name: String,
    pub description: String,
    pub credits: String,
    pub prerequisites: String,
    pub offered: BTreeSet<String>,
    pub distribution: BTreeSet<String>,
    pub link: String,
}
