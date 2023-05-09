use anyhow::{bail, Context, Result};
use futures::stream::*;
use indicatif::ProgressBar;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeSet;

pub async fn do_stuff() -> Result<()> {
    let mut courses = Vec::new();
    let links = get_course_links().await?;
    let links_iter = links.iter();
    let client = Client::new();
    let bar = ProgressBar::new(links.len().try_into().unwrap());
    let mut errors = Vec::new();
    let mut futures = FuturesOrdered::new();
    let batch_size: usize = 1;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(batch_size as u64));

    for link in links_iter {
        futures.push_back(get_course(&link, &client));
        if futures.len() == batch_size {
            match futures.next().await.unwrap() {
                Ok(course) => courses.push(course),
                Err(e) => errors.push(e),
            }
        }
        bar.inc(1);
        interval.tick().await;
    }
    while let Some(result) = futures.next().await {
        match result {
            Ok(course) => courses.push(course),
            Err(e) => errors.push(e),
        }
    }
    bar.finish();
    println!("{:#?}", errors);
    println!("Total Errors: {}", errors.len());
    // tokio::fs::write(
    //     "courses.json",
    //     serde_json::to_string_pretty(&courses).unwrap(),
    // )
    // .await?;

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
                        + &*course["Path"]
                            .as_str()
                            .context("\"Path\" field missing from course")?
                            .to_lowercase(),
                );
            }
        }
    }
    Ok(courses)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Logic {
    Or,
    And,
    GroupStart,
    GroupEnd,
    Equivalence,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Seniority {
    pub freshman: bool,
    pub sophomore: bool,
    pub junior: bool,
    pub senior: bool,
    pub graduate: bool,
    pub doctorate: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Permission {
    DeanUndergraduate,
    DeanGraduate,
    Instructor,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Special {
    Major(String),
    Pinnacle(bool),
}
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Token {
    CoursePrereq(String),
    CourseCoreq(String),
    Logical(Logic),
    Seniority(Seniority),
    Permission(Permission),
    Special(Special),
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
        .context("no elementt matched the first selector in id parsing")?
        .text()
        .nth(1)
        .context("(1)th element not found in id parsing")?
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

    let prereq_tokens = element
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_prereqs"))
        .context("no element matched the first attribute in prerequisites parsing")?
        .text()
        .map(|s| {
            s.trim()
                .to_lowercase()
                .replace("\n", "") // Remove html characters
                .replace("\t", "")
                .replace("(", " ( ") // Add space around parenthesis so
                .replace(")", " ) ") // they can be parsed
        })
        .filter(|s| !s.is_empty() && s != "prerequisite")
        .collect::<Vec<String>>()
        .join(" ");
    let mut prereq_tokens = prereq_tokens
        .split(' ')
        .filter(|s| !s.is_empty())
        .peekable();

    fn get_course_id<'a>(
        iter: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
    ) -> Result<String> {
        let mut letters = "".into();
        let mut numbers = "".into();
        let token = iter.peek().context("HOW?!!")?;
        if !token.chars().all(|c| !c.is_digit(10)) {
            for char in token.chars() {
                if char.is_digit(10) {
                    let (l, r) = token.split_once(char).unwrap();
                    letters = l.to_owned();
                    numbers = (char.to_string() + r).to_owned();
                    break;
                }
            }
        } else {
            letters = (iter.next().unwrap()).to_owned();
            numbers = (*iter
                .peek()
                .context("tokens unexpectedly ended after course letter code")?)
            .to_owned();
        }
        if !numbers.chars().all(|c| c.is_digit(10)) {
            bail!("course id numbers not all numeric: {}", numbers)
        }
        if letters.len() < 2 || letters.len() > 4 {
            bail!(
                "couse id letters not of proper length: {} - {}",
                letters.len(),
                letters
            )
        }
        if numbers.len() < 1 || numbers.len() > 3 {
            bail!(
                "course id number not of proper length: {} - {}",
                numbers.len(),
                numbers
            )
        }
        iter.next();
        Ok(letters + " " + &numbers)
    }

    let mut prerequisites: Vec<Token> = vec![];
    let mut get_prereqs = || {
        while prereq_tokens.peek().is_some() {
            match *prereq_tokens.peek().unwrap() {
                "complete" => {
                    prereq_tokens.next(); // Skip
                }
                "or" => {
                    prerequisites.push(Token::Logical(Logic::Or));
                    prereq_tokens.next();
                }
                "and" => {
                    prerequisites.push(Token::Logical(Logic::And));
                    prereq_tokens.next();
                }
                "(" => {
                    prerequisites.push(Token::Logical(Logic::GroupStart));
                    prereq_tokens.next();
                }
                ")" => {
                    prerequisites.push(Token::Logical(Logic::GroupEnd));
                    prereq_tokens.next();
                }
                "at" => {
                    if prereq_tokens.nth(1).context(
                        "tokens unexpectedly ended after \"at\", epected to find \"least\"",
                    )? == "least"
                    {
                        // TODO Add error message
                        match prereq_tokens
                            .next()
                            .context("tokens unexpectedly ended after \"input\"")?
                        {
                            "junior" => prerequisites.push(Token::Seniority(Seniority {
                                freshman: false,
                                sophomore: false,
                                junior: true,
                                senior: true,
                                graduate: true,
                                doctorate: true,
                            })),
                            _ => bail!("{}", prereq_tokens.next().unwrap()), // TODO Add error message
                        }
                    } else {
                        bail!("unexpected token following \"at\"")
                    }
                }
                "freshman" | "freshmen" => {
                    if let Some(t) = prereq_tokens.nth(1) {
                        match t {
                            "pinnacle" => {
                                if prereq_tokens.peek().is_some()
                                    && *prereq_tokens.peek().unwrap() == "scholars"
                                {
                                    prereq_tokens.next();
                                }
                                if prereq_tokens.peek().is_some()
                                    && *prereq_tokens.peek().unwrap() == "only"
                                {
                                    prereq_tokens.next();
                                }
                                prerequisites.push(Token::Seniority(Seniority {
                                    freshman: true,
                                    sophomore: false,
                                    junior: false,
                                    senior: false,
                                    graduate: false,
                                    doctorate: false,
                                }));
                                prerequisites.push(Token::Logical(Logic::And));
                                prerequisites.push(Token::Special(Special::Pinnacle(true)));
                            }
                            _ => bail!("unexpected token \"{}\" after freshm[ae]n", t),
                        }
                    } else {
                        bail!("tokens unexpectedly ended after \"freshm[ae]n\"");
                    }
                }
                "junior" | "juniors" => {
                    prerequisites.push(Token::Seniority(Seniority {
                        freshman: false,
                        sophomore: false,
                        junior: true,
                        senior: false,
                        graduate: false,
                        doctorate: false,
                    }));
                    prereq_tokens.next();
                }
                "senior" | "seniors" => {
                    prerequisites.push(Token::Seniority(Seniority {
                        freshman: false,
                        sophomore: false,
                        junior: false,
                        senior: true,
                        graduate: false,
                        doctorate: false,
                    }));
                    prereq_tokens.next();
                    if prereq_tokens.peek().is_some()
                        && (*prereq_tokens.peek().unwrap() == "student"
                            || *prereq_tokens.peek().unwrap() == "students")
                    {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && (*prereq_tokens.peek().unwrap() == "only")
                    {
                        prereq_tokens.next();
                    }
                }
                "graduate" | "grad" => {
                    prereq_tokens.next();
                    if prereq_tokens.peek().is_some()
                        && (*prereq_tokens.peek().unwrap() == "student"
                            || *prereq_tokens.peek().unwrap() == "students")
                    {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && (*prereq_tokens.peek().unwrap() == "only")
                    {
                        prerequisites.push(Token::Seniority(Seniority {
                            freshman: false,
                            sophomore: false,
                            junior: false,
                            senior: false,
                            graduate: true,
                            doctorate: false,
                        }));
                        prereq_tokens.next();
                    } else {
                        prerequisites.push(Token::Seniority(Seniority {
                            freshman: false,
                            sophomore: false,
                            junior: false,
                            senior: false,
                            graduate: true,
                            doctorate: true,
                        }));
                    }
                }

                "doctoral" | "phd" => {
                    prerequisites.push(Token::Seniority(Seniority {
                        freshman: false,
                        sophomore: false,
                        junior: false,
                        senior: false,
                        graduate: false,
                        doctorate: true,
                    }));
                    prereq_tokens.next();
                    if prereq_tokens.peek().is_some()
                        && (*prereq_tokens.peek().unwrap() == "student"
                            || *prereq_tokens.peek().unwrap() == "students")
                    {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && (*prereq_tokens.peek().unwrap() == "only")
                    {
                        prereq_tokens.next();
                    }
                }
                "with" => {}
                "permission" | "instructor's" | "instructors" | "instructor" => {
                    prerequisites.push(Token::Permission(Permission::Instructor));
                    prereq_tokens.next();
                    'outer: loop {
                        if prereq_tokens.peek().is_some() {
                            match *prereq_tokens.peek().unwrap() {
                                "permission" | "required" | "of" => {
                                    prereq_tokens.next();
                                }
                                _ => break 'outer,
                            }
                        } else {
                            break 'outer;
                        }
                    }
                }
                "no" => {
                    prereq_tokens.next();
                    if *prereq_tokens.peek().unwrap() != "freshmen" {
                        bail!(
                            "unexpected token after \"no\": \"{}\" expected \"freshmen\"",
                            prereq_tokens.peek().unwrap()
                        )
                    } else {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && *prereq_tokens.peek().unwrap() != "or" {
                        bail!(
                            "unexpected token after \"freshmen\": \"{}\" expected \"or\"",
                            prereq_tokens.peek().unwrap()
                        )
                    } else {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some()
                        && *prereq_tokens.peek().unwrap() != "sophomores"
                    {
                        bail!(
                            "unexpected token after \"or\": \"{}\" expected \"sophomores\"",
                            prereq_tokens.peek().unwrap()
                        )
                    } else {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && *prereq_tokens.peek().unwrap() != "cohort"
                    {
                        bail!(
                            "unexpected token after \"sophomores\": \"{}\" expected \"cohort\"",
                            prereq_tokens.peek().unwrap()
                        )
                    } else {
                        prereq_tokens.next();
                    }
                    prerequisites.push(Token::Seniority(Seniority {
                        freshman: false,
                        sophomore: false,
                        junior: true,
                        senior: true,
                        graduate: true,
                        doctorate: true,
                    }));
                }
                "coreq" => {
                    prereq_tokens.next();
                    prerequisites.push(Token::CourseCoreq(get_course_id(&mut prereq_tokens)?));
                }
                "pre-/co-req" => {
                    prereq_tokens.next();
                    let course = get_course_id(&mut prereq_tokens)?;
                    prerequisites.push(Token::Logical(Logic::GroupStart));
                    prerequisites.push(Token::CoursePrereq(course.clone()));
                    prerequisites.push(Token::CourseCoreq(course.clone()));
                    prerequisites.push(Token::Logical(Logic::GroupEnd));
                }
                "need" => match prereq_tokens
                    .nth(5)
                    .context("tokens ended unexpected after \"need\"")?
                {
                    "498" => prerequisites.push(Token::Permission(Permission::DeanUndergraduate)),
                    "698" => prerequisites.push(Token::Permission(Permission::DeanGraduate)),
                    _ => bail!("unexpected token 5 after \"need\", expected DEAN course number"),
                },
                _ => {
                    let course = get_course_id(&mut prereq_tokens)?;
                    if prereq_tokens.peek().is_some() && *prereq_tokens.peek().unwrap() == "coreq" {
                        prerequisites.push(Token::CourseCoreq(course));
                        prereq_tokens.next();
                    } else {
                        prerequisites.push(Token::CoursePrereq(course));
                    }
                }
            };
        }
        Ok(())
    };
    match get_prereqs() {
        Err(e) => Err(e).context(format!("ID: {} - Link: {}", id, link))?,
        _ => {}
    }
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
    pub prerequisites: Vec<Token>,
    pub offered: BTreeSet<String>,
    pub distribution: BTreeSet<String>,
    pub link: String,
}
