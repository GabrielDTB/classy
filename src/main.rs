use reqwest::Error;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use url::{ParseError, Url};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let courses = get_courses().await?;
    println!("{}", serde_json::to_string_pretty(&courses).unwrap());
    Ok(())
}

async fn get_courses() -> Result<Vec<Course>, Error> {
    let url = "https://stevens.smartcatalogiq.com/Institutions/Stevens-Institution-of-Technology/json/2022-2023/Academic-Catalog.json";
    let response = reqwest::get(url).await?;
    let l1 = &response.json::<serde_json::Value>().await?["Children"][23];
    let mut courses: Vec<Course> = vec![];
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
                let course = serde_json::from_value(l4.clone()).unwrap();
                courses.push(course);
            }
        }
    }
    Ok(courses)
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
