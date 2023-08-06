// use futures::stream::*;
// use heck::ToTitleCase;
// use indicatif::ProgressBar;
use crate::class::*;
use reqwest::Client;
use scraper::ElementRef;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
// use serde_json;
// use std::collections::BTreeSet;
// use std::collections::HashMap;
use thiserror::Error;
// use tokio::fs::File;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};

// const API_LINK: &str = "https://stevens.smartcatalogiq.com/Institutions/Stevens-Institution-of-Technology/json/2023-2024/Academic-Catalog.json";
const CLASSES_PAGE: &str =
    "https://stevens.smartcatalogiq.com/en/2023-2024/academic-catalog/courses/";

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

#[derive(Serialize, Deserialize, Clone)]
pub struct ClassPage {
    pub link: String,
    pub text: String,
}

/// Queries classes from the provided api link
/// and returns a vec of the response texts,
/// returning early if an error is added to
/// the vec.
pub async fn query_classes(cache: &Vec<ClassPage>) -> Vec<Result<ClassPage, ClassQueryError>> {
    // let mut links = match query_class_links().await {
    //     Ok(value) => value,
    //     Err(why) => return vec![Err(why)],
    // };
    let classes_page = Html::parse_document(
        reqwest::get(CLASSES_PAGE)
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
            .as_str(),
    );
    let class_nodes = classes_page
        .select(&Selector::parse("#main > ul:nth-child(3) > li").unwrap())
        .collect::<Vec<_>>();
    let mut links = class_nodes
        .iter()
        .map(|t| {
            format!(
                "https://stevens.smartcatalogiq.com{}/",
                t.inner_html().split("\"").nth(1).unwrap().to_lowercase()
            )
        })
        .collect::<Vec<_>>();

    let mut responses = Vec::with_capacity(links.len());
    for response in cache {
        if let Some(index) = links.iter().position(|link| *link == response.link) {
            links.remove(index);
            responses.push(Ok((*response).clone()));
        } else {
            responses.push(Err(ClassQueryError::CachedLinkNotFound {
                cached_link: response.link.clone(),
            }));
            return responses;
        }
    }
    let client = Client::new();
    let mut counter = 1;
    let length = links.len();
    for link in links {
        println!("Querying {}: {}", length, counter);
        match client.get(&link).send().await {
            Ok(response) => match response.status() {
                reqwest::StatusCode::OK => responses.push(Ok(ClassPage {
                    link,
                    text: response.text().await.unwrap(),
                })),
                _ => {}
            },
            Err(why) => {
                responses.push(Err(ClassQueryError::Reqwest { source: why }));
                return responses;
            }
        };
        counter += 1;
    }
    responses
}

// async fn query_class_links() -> Result<Vec<String>, ClassQueryError> {
//     let mut links = vec![];
//     let response = reqwest::get(API_LINK).await?;
//     let l1 = &response.json::<serde_json::Value>().await?["Children"][23];
//     // TODO Rewrite
//     for c1 in 0..65536 {
//         let l2 = &l1["Children"][c1];
//         if l2.is_null() {
//             break;
//         }
//         for c2 in 0..65536 {
//             let l3 = &l2["Children"][c2];
//             if l3.is_null() {
//                 break;
//             }
//             for c3 in 0..65536 {
//                 let course = match &l3["Children"][c3] {
//                     value => match value.is_null() {
//                         true => break,
//                         _ => value,
//                     },
//                 };
//                 links.push(
//                     "https://stevens.smartcatalogiq.com/en".to_string()
//                         + &*course["Path"]
//                             .as_str()
//                             .unwrap_or_else(|| todo!())
//                             .to_lowercase(),
//                 );
//             }
//         }
//     }
//     Ok(links)
// }

pub fn parse_class(page: ClassPage) -> Option<Class> {
    let html = Html::parse_document(page.text.as_str());
    let main = match html
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("main"))
    {
        Some(value) => value,
        None => todo!(),
    };

    let id = match &*parse_id(&main) {
        "HSS HSS 317" => "HSS 317",
        "EM 347 ZZZDNU" => "EM 347",
        rest => rest,
    }
    .to_owned();
    let name = parse_name(&main);
    let description = parse_description(&main);
    let credits = parse_credits(&main);
    let cross_listed = parse_cross_listed(&main);
    let prerequisites = parse_prerequisites(&main);
    let offered = parse_offered(&main);
    let distribution = parse_distribution(&main);
    let link = page.link;

    if link.contains("narrative-courses")
        || link.contains("te-technical-elective")
        || link.contains("hum-humanities-general")
    {
        return None;
    }

    Some(Class::new(
        id.chars().filter(|c| c.is_alphabetic()).collect::<String>(),
        link.split_once("/courses/")
            .unwrap()
            .1
            .split_once("/")
            .unwrap()
            .0
            .split_once("-")
            .unwrap()
            .1
            .split("-")
            .map(std::primitive::str::trim)
            .map(|s| if s == "humanities" { "hplaceholder" } else { s })
            .map(|s| match s.strip_prefix("humanities") {
                Some(rest) => format!("humanities {}", rest),
                None => String::from(s),
            })
            .map(|s| {
                if s == "hplaceholder" {
                    String::from("humanities")
                } else {
                    s
                }
            })
            .map(|s| {
                if s == "languageitalian" {
                    String::from("language italian")
                } else {
                    s
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split(" ")
            .map(|s| match s {
                "or" | "and" | "of" | "for" => String::from(s),
                _ => {
                    format!("{}{}", (&s[..1].to_string()).to_uppercase(), &s[1..])
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        id.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>(),
        name,
        description,
        credits,
        prerequisites,
        offered,
        vec![cross_listed],
        distribution,
        link,
    ))
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
    let element = main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_credits"));
    match element {
        None => String::from("0"),
        Some(element) => element
            .select(&Selector::parse("div").unwrap())
            .find(|element| element.value().attr("class") == Some("credits"))
            .unwrap_or_else(|| todo!())
            .text()
            .collect::<String>()
            .trim()
            .to_owned(),
    }
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
                _ => String::from(""),
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
fn parse_offered(main: &ElementRef) -> Vec<String> {
    match main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("offered"))
    {
        Some(e) => e
            .text()
            .last()
            .unwrap_or_else(|| unreachable!())
            .split("\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect::<Vec<_>>(),
        _ => vec![],
    }
}
fn parse_distribution(main: &ElementRef) -> Vec<String> {
    match main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("distribution"))
    {
        Some(value) => value
            .text()
            .last()
            .unwrap_or_else(|| unreachable!())
            .split("\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect::<Vec<_>>(),
        _ => vec![],
    }
}
