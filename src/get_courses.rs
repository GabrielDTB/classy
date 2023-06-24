// use futures::stream::*;
// use heck::ToTitleCase;
// use indicatif::ProgressBar;
use reqwest::Client;
use scraper::ElementRef;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
// use std::collections::BTreeSet;
// use std::collections::HashMap;
use thiserror::Error;
// use tokio::fs::File;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};

const API_LINK: &str = "https://stevens.smartcatalogiq.com/Institutions/Stevens-Institution-of-Technology/json/2022-2023/Academic-Catalog.json";

#[derive(Error, Debug)]
pub enum ClassQueryError {
    #[error("")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    #[error("")]
    CachedLinkNotFound { cached_link: String },
}

pub struct ClassPage {
    link: String,
    text: String,
}

/// Queries classes from the provided api link
/// and returns a vec of the response texts,
/// returning early if an error is added to
/// the vec.
pub async fn query_classes(cache: Vec<ClassPage>) -> Vec<Result<ClassPage, ClassQueryError>> {
    let mut links = match query_class_links().await {
        Ok(value) => value,
        Err(why) => return vec![Err(why)],
    };
    let mut responses = Vec::with_capacity(links.len());
    for response in cache {
        if let Some(index) = links.iter().position(|link| *link == response.link) {
            links.remove(index);
            responses.push(Ok(response));
        } else {
            responses.push(Err(ClassQueryError::CachedLinkNotFound {
                cached_link: response.link,
            }));
            return responses;
        }
    }
    let client = Client::new();
    for link in links {
        match client.get(&link).send().await {
            Ok(response) => responses.push(Ok(ClassPage {
                link,
                text: response.text().await.unwrap(),
            })),
            Err(why) => {
                responses.push(Err(ClassQueryError::Reqwest { source: why }));
                return responses;
            }
        };
    }
    responses
}

async fn query_class_links() -> Result<Vec<String>, ClassQueryError> {
    let mut links = vec![];
    let response = reqwest::get(API_LINK).await?;
    let l1 = &response.json::<serde_json::Value>().await?["Children"][23];
    // TODO Rewrite
    for c1 in 0..65536 {
        let l2 = &l1["Children"][c1];
        if l2.is_null() {
            break;
        }
        for c2 in 0..65536 {
            let l3 = &l2["Children"][c2];
            if l3.is_null() {
                break;
            }
            for c3 in 0..65536 {
                let course = match &l3["Children"][c3] {
                    value => match value.is_null() {
                        true => break,
                        _ => value,
                    },
                };
                links.push(
                    "https://stevens.smartcatalogiq.com/en".to_string()
                        + &*course["Path"]
                            .as_str()
                            .unwrap_or_else(|| todo!())
                            .to_lowercase(),
                );
            }
        }
    }
    Ok(links)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub description: String,
    pub credits: String,
    pub cross_listed: String,
    pub prerequisites: String,
    pub offered: String,
    pub distribution: String,
    pub link: String,
}

pub fn parse_class(page: ClassPage) -> Class {
    let html = Html::parse_document(page.text.as_str());
    let main = match html
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("main"))
    {
        Some(value) => value,
        None => todo!(),
    };
    Class {
        id: parse_id(&main),
        name: parse_name(&main),
        description: parse_description(&main),
        credits: parse_credits(&main),
        cross_listed: parse_cross_listed(&main),
        prerequisites: parse_prerequisites(&main),
        offered: parse_offered(&main),
        distribution: parse_distribution(&main),
        link: page.link,
    }
}

fn parse_id(main: &ElementRef) -> String {
    main.select(&Selector::parse("h1").unwrap())
        .next()
        .unwrap_or_else(|| todo!())
        .text()
        .nth(1)
        .unwrap_or_else(|| todo!())
        .trim()
        .to_string()
}
fn parse_name(main: &ElementRef) -> String {
    main.select(&Selector::parse("h1").unwrap())
        .next()
        .unwrap_or_else(|| todo!())
        .text()
        .last()
        .unwrap_or_else(|| todo!())
        .trim()
        .to_string()
}
fn parse_description(main: &ElementRef) -> String {
    let flatten = regex::Regex::new(r"\s+").unwrap();
    let description = main
        .select(&Selector::parse("div").unwrap())
        //println!("{}", serde_json::to_string_pretty(&courses).unwrap());
        .find(|element| element.value().attr("class") == Some("desc"))
        .unwrap_or_else(|| todo!())
        .text()
        .collect::<String>()
        //.context("last element not found in description parsing")?
        .replace("\n", " ")
        .replace("\t", " ");
    flatten.replace_all(&*description, " ").trim().to_string()
}
fn parse_credits(main: &ElementRef) -> String {
    main.select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_credits"))
        .unwrap_or_else(|| todo!())
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("credits"))
        .unwrap_or_else(|| todo!())
        .text()
        .collect::<String>()
        .trim()
        .to_owned()
}
fn parse_cross_listed(main: &ElementRef) -> String {
    // let mut out = vec![];
    match main
        .select(&Selector::parse("div.sc_credits + h3 + a.sc-courselink").unwrap())
        .next()
    {
        Some(a) => a.text().collect::<String>(),
        None => {
            match main
                .select(&Selector::parse("div.sc_credits + h3").unwrap())
                .next()
            {
                Some(h3) => h3
                    .next_sibling()
                    .unwrap()
                    .value()
                    .as_text()
                    .unwrap_or_else(|| todo!())
                    .chars()
                    .collect::<String>(),
                _ => todo!(),
            }
        }
    }
}
fn parse_prerequisites(main: &ElementRef) -> String {
    main.select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_prereqs"))
        .unwrap_or_else(|| todo!())
        .text()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "Prerequisite")
        .collect::<Vec<&str>>()
        .join(" ")
}
fn parse_distribution(main: &ElementRef) -> String {
    match main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("distribution"))
    {
        Some(value) => value
            .text()
            .last()
            .unwrap_or_else(|| todo!())
            .trim()
            .to_owned(),
        _ => String::from(""),
    }
}
fn parse_offered(main: &ElementRef) -> String {
    match main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("offered"))
    {
        Some(e) => e.text().last().unwrap_or_else(|| todo!()).trim().to_owned(),
        _ => String::from(""),
    }
}
