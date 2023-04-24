use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use url::{ParseError, Url};

#[tokio::main]
async fn main() -> Result<()> {
    //let courses = get_course_links().await?;
    //println!("{}", serde_json::to_string_pretty(&courses).unwrap());

    Ok(())
}

async fn get_course_links() -> Result<BTreeMap<String, String>> {
    let mut courses = BTreeMap::new();
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
                    course["Name"]
                        .as_str()
                        .context("\"Name\" field missing from course")?
                        .to_string(),
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

async fn get_course(id: String, link: String) -> Result<Course> {
    let response = reqwest::get(link.clone()).await?.text().await?;
    let html = Html::parse_document(&response);
    let element = match html
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("main"))
    {
        Some(value) => value,
        _ => return Err(anyhow::anyhow!("IDK")),
    };

    let name = element
        .select(&Selector::parse("h1").unwrap())
        .next()
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .to_string();
    let description = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("desc"))
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .to_string();
    let credits = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_credits"))
        .unwrap()
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("credits"))
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .parse::<u8>()
        .unwrap();
    let prerequisites = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_prereqs"))
        .unwrap()
        .select(&Selector::parse("a").unwrap())
        .find(|element| element.value().attr("class") == Some("sc-courselink"))
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .to_string();
    let distribution = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("distribution"))
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .to_string();
    let offered = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("offered"))
        .unwrap()
        .text()
        .last()
        .unwrap()
        .trim()
        .to_string();

    Ok(Course {
        id,
        name,
        description,
        credits,
        prerequisites,
        offered,
        distribution,
        link,
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

#[derive(Serialize, Deserialize, Debug)]
struct Course {
    pub id: String,
    pub name: String,
    pub description: String,
    pub credits: u8,
    pub prerequisites: String,
    pub offered: String,
    pub distribution: String,
    pub link: String,
}
