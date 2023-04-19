use reqwest::Error;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashMap};
use url::{ParseError, Url};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let courses = get_courses().await?;
    println!("{}", serde_json::to_string_pretty(&courses).unwrap());
    Ok(())
}

async fn get_courses() -> Result<BTreeMap<String, String>, Error> {
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
                let l4 = &l3["Children"][c3];
                if l4.is_null() {
                    break;
                }
                let course: Course = serde_json::from_value(l4.clone()).unwrap();
                courses.insert(
                    course.name,
                    course
                        .link
                        .split_once("/2022-2023/Academic-Catalog/Courses/")
                        .unwrap()
                        .1
                        .to_string(),
                );
            }
        }
    }
    Ok(courses)
}

fn get_link(course_id: String, course_mappings: HashMap<String, String>) -> Option<String> {
    let shared =
        String::from("https://stevens.smartcatalogiq.com/en/2022-2023/Academic-Catalog/Courses/");
    let (prefix, id) = match course_id.split_once(' ') {
        Some(tuple) => (tuple.0, tuple.1),
        _ => return None,
    };
    let department = match course_mappings.get(prefix) {
        Some(value) => value,
        _ => return None,
    };
    let level = match id.len() {
        3 => format!("{}00", id.chars().next().unwrap()),
        1 | 2 => String::from('0'),
        _ => return None,
    };
    Some(format!(
        "{shared}{prefix}-{department}/{level}/{prefix}-{id}"
    ))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
struct Courses {
    pub courses: Vec<Course>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Course {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Path")]
    pub link: String,
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
