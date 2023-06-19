use futures::stream::*;
use heck::ToTitleCase;
use indicatif::ProgressBar;
use reqwest::Client;
use scraper::ElementRef;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeSet;
use std::collections::HashMap;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Error)]
pub enum ClassQueryError {
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    CachedLinkNotFound {
        cached_link: String,
    },
}

const a = ""

pub struct ClassPage {
    link: String,
    text: String,
}

/// Queries classes from the provided api link
/// and returns a vec of the response texts,
/// returning early if an error is added to
/// the vec.
pub async fn query_classes(
    link: String,
    cache: Vec<ClassPage>,
) -> Vec<Result<ClassPage, ClassQueryError>> {
    let mut links = match query_class_links(link).await {
        Ok(value) => value,
        Err(why) => return vec![Err(why)],
    };
    let mut responses = Vec::with_capacity(links.len());
    for response in cache {
        let position = links.iter().position(|l| *l == response.link);
        match position {
            Some(value) => {
                links.remove(value);
                responses.push(Ok(response.text));
            }
            None => {
                responses.push(Err(ClassQueryError::CachedLinkNotFound {
                    cached_link: response.link,
                }));
                return responses;
            }
        };
    }
    let client = Client::new();
    for link in links {
        match client.get(&link).send().await {
            Ok(value) => responses.push(Ok(ClassPage {
                link,
                text: value.text().await?,
            })),
            Err(why) => {
                responses.push(Err(why));
                return responses;
            }
        };
    }
    responses
}

async fn query_class_links(link: String) -> Result<Vec<String>, ClassQueryError> {
    let mut links = vec![];
    let response = reqwest::get(link).await?;
    let l1 = &response.json::<serde_json::Value>().await?["Children"][23];
    // TODO Rewrite
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
                links.push(
                    "https://stevens.smartcatalogiq.com/en".to_string()
                        + &*course["Path"]
                            .as_str()
                            .context("\"Path\" field missing from course")?
                            .to_lowercase(),
                );
            }
        }
    }
    Ok(links)
}

#[derive(Error)]
pub enum ClassParseError {
    TargetNotFound { target: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub description: String,
    pub credits: String,
    pub cross_listed: Vec<String>,
    pub prerequisites: String,
    pub offered: Vec<String>,
    pub distribution: Vec<String>,
    pub link: String,
}

pub fn parse_class(page: ClassPage) -> Result<Class, ClassParseError> {
    let main = match Html::parse_document(&*page.text)
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("id") == Some("main"))
    {
        Some(value) => value,
        None => {
            return Err(ClassParseError::TargetNotFound {
                target: "main".into(),
            })
        }
    };
    Ok(Class {
        id: parse_id(&main)?,
        name: parse_name(&main)?,
        description: parse_description(&main)?,
        credits: parse_credits(&main)?,
        cross_listed: parse_cross_listed(&main)?,
        prerequisites: parse_prerequisites(&main)?,
        offered: parse_offered(&main)?,
        distribution: parse_distribution(&main)?,
        link: page.link,
    })
}

fn parse_id(main: &ElementRef) -> Result<String, ClassParseError> {
    Ok(main
        .select(&Selector::parse("h1").unwrap())
        .next()
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "h1".into(),
            }),
        )
        .text()
        .nth(1)
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "second element".into(),
            }),
        )
        .trim()
        .to_string())
}
fn parse_name(main: &ElementRef) -> Result<String, ClassParseError> {
    Ok(main
        .select(&Selector::parse("h1").unwrap())
        .next()
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "h1".into(),
            }),
        )
        .text()
        .last()
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "last element".into(),
            }),
        )
        .trim()
        .to_string())
}
fn parse_description(main: &ElementRef) -> Result<String, ClassParseError> {
    let flatten = regex::Regex::new(r"\s+").unwrap();
    let description = main
        .select(&Selector::parse("div").unwrap())
        //println!("{}", serde_json::to_string_pretty(&courses).unwrap());
        .find(|element| element.value().attr("class") == Some("desc"))
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "desc".into(),
            }),
        )
        .text()
        .collect::<String>()
        //.context("last element not found in description parsing")?
        .replace("\n", " ")
        .replace("\t", " ");
    Ok(flatten.replace_all(&*description, " ").trim().to_string())
}
fn parse_credits(main: &ElementRef) -> Result<String, ClassParseError> {
    Ok(main
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("sc_credits"))
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "sc_credits".into(),
            }),
        )
        .select(&Selector::parse("div").unwrap())
        .find(|element| element.value().attr("class") == Some("credits"))
        .unwrap_or(
            return Err(ClassParseError::TargetNotFound {
                target: "credits".into(),
            }),
        )
        .text()
        .collect::<String>()
        .trim()
        .to_owned())
}
fn parse_cross_listed(main: &ElementRef) -> Result<Vec<String>, ClassParseError> {
    let mut out = vec![];
    let text = main
        .select(&Selector::parse("div.sc_credits + h3 + a.sc-courselink").unwrap())
        .next()
        .unwrap_or(
            
        ) {
        Some(a) => Some(a.text().collect::<String>()),
        None => {
            match element
                .select(&Selector::parse("div.sc_credits + h3").unwrap())
                .next()
            {
                Some(h3) => Some(
                    h3.next_sibling()
                        .unwrap()
                        .value()
                        .as_text()
                        .context(format!("{} {:?}", id, h3.next_sibling().unwrap().value()))
                        .unwrap()
                        .chars()
                        .collect::<String>(),
                ),
                _ => None,
            }
        }
    };
}
fn parse_prerequisites(main: &ElementRef) -> Result<String, ClassParseError> {}
fn parse_distribution(main: &ElementRef) -> Result<String, ClassParseError> {}
fn parse_offered(main: &ElementRef) -> Result<String, ClassParseError> {}

async fn get_course(link: &str, client: &Client) -> Result<Course> {
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
                .replace(",", " ")
                .replace("/", " ")
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
            bail!(
                "course id numbers not all numeric in: {} -- {}",
                letters,
                numbers
            )
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
                    prereq_tokens.next();
                    if *prereq_tokens.peek().context(
                        "tokens unexpectedly ended after \"at\", epected to find \"least\"",
                    )? != "least"
                    {
                        bail!(
                            "unexpected token \"{}\" after at",
                            *prereq_tokens.peek().unwrap()
                        );
                    }
                    prereq_tokens.next();
                    if *prereq_tokens
                        .peek()
                        .context("tokens unexpectedly ended after \"least\"")?
                        == "a"
                    {
                        prereq_tokens.next();
                    }
                    match *prereq_tokens
                        .peek()
                        .context("tokens unexpectedly ended in \"at\" branch")?
                    {
                        "junior" => {
                            prerequisites.push(START);
                            prerequisites.push(JUNIOR);
                            prerequisites.push(OR);
                            prerequisites.push(SENIOR);
                            prerequisites.push(END);
                        }
                        _ => bail!(
                            "unexpected token within \"at\" branch: {}",
                            prereq_tokens.next().unwrap()
                        ),
                    }
                    prereq_tokens.next();
                }
                "freshman" | "freshmen" => {
                    prereq_tokens.next();
                    prerequisites.push(FRESHMAN);
                }
                "junior" | "juniors" => {
                    prereq_tokens.next();
                    if prereq_tokens.peek().is_some() {
                        match *prereq_tokens.peek().unwrap() {
                            "standing" => {
                                prerequisites.push(START);
                                prerequisites.push(JUNIOR);
                                prerequisites.push(OR);
                                prerequisites.push(SENIOR);
                                prerequisites.push(END);
                                prereq_tokens.next();
                            }
                            _ => prerequisites.push(JUNIOR),
                        }
                    } else {
                        prerequisites.push(JUNIOR);
                    }
                }
                "senior" | "seniors" => {
                    prerequisites.push(SENIOR);
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
                        prerequisites.push(GRADUATE);
                        prereq_tokens.next();
                    } else {
                        prerequisites.push(START);
                        prerequisites.push(GRADUATE);
                        prerequisites.push(OR);
                        prerequisites.push(DOCTORATE);
                        prerequisites.push(END);
                    }
                }

                "doctoral" | "phd" => {
                    prerequisites.push(DOCTORATE);
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
                "pinnacle" => {
                    prerequisites.push(Token::Special(Special::Pinnacle(true)));
                    prereq_tokens.next();
                    if prereq_tokens.peek().is_some()
                        && (*prereq_tokens.peek().unwrap() == "scholars"
                            || *prereq_tokens.peek().unwrap() == "scholar")
                    {
                        prereq_tokens.next();
                    }
                    if prereq_tokens.peek().is_some() && *prereq_tokens.peek().unwrap() == "only" {
                        prereq_tokens.next();
                    }
                }

                "with" => {
                    prereq_tokens.next();
                    if *prereq_tokens
                        .peek()
                        .context("tokens unexpectedly ended after \"with\"")?
                        != "cgpa"
                    {
                        bail!(
                            "unexpected token after \"with\": {}",
                            *prereq_tokens.peek().unwrap()
                        );
                    }
                    prereq_tokens.next();
                    if *prereq_tokens
                        .peek()
                        .context("tokens unexpectedly ended after \"cgpa\"")?
                        != "=>"
                    {
                        bail!(
                            "unexpected token after \"cgpa\": {}",
                            *prereq_tokens.peek().unwrap()
                        );
                    }
                    prereq_tokens.next();
                    prerequisites.push(Token::Special(Special::Cgpa(
                        prereq_tokens
                            .next()
                            .context("tokens unexpectedly ended after \"=>\"")?
                            .to_owned(),
                    )));
                }
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
                    prerequisites.push(START);
                    prerequisites.push(JUNIOR);
                    prerequisites.push(OR);
                    prerequisites.push(SENIOR);
                    prerequisites.push(END);
                }
                "coreq" => {
                    prereq_tokens.next();
                    prerequisites.push(Token::CourseCoreq(get_course_id(&mut prereq_tokens)?));
                }
                "pre-" => {
                    prereq_tokens.next();
                    if *prereq_tokens.peek().context("")? != "co-req" {
                        bail!(
                            "unexpected token after \"pre-\": {}",
                            *prereq_tokens.peek().unwrap()
                        );
                    }
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
    match &*id {
        "BIO 682" => {
            prerequisites = vec![
                major("chem bio"),
                AND,
                START,
                GRADUATE,
                OR,
                CERTIFICATE,
                OR,
                DOCTORATE,
                END,
            ]
        }
        "BIO 689" => {
            prerequisites = vec![
                pre("bio 381"),
                OR,
                pre("ch 381"),
                OR,
                START,
                GRADUATE,
                AND,
                major("chem bio"),
                END,
            ]
        }
        "BME 343" => {
            prerequisites = vec![
                START,
                pre("ma 226"),
                OR,
                pre("ma 227"),
                END,
                AND,
                pre("bme 306"),
            ]
        }
        "bme 424" => prerequisites = vec![cor("ide 402"), AND, pre("bme 423")],
        "bme 520" => {
            prerequisites = vec![
                START,
                pre("bio 281"),
                AND,
                pre("ma 221"),
                AND,
                pre("pep 221"),
                END,
                OR,
                START,
                major("bio med"),
                AND,
                GRADUATE,
                END,
            ]
        }
        "" => prerequisites = vec![],
        "" => prerequisites = vec![],
        "" => prerequisites = vec![],
        "" => prerequisites = vec![],
        "" => prerequisites = vec![],
        "" => prerequisites = vec![],
        _ => match get_prereqs() {
            Err(e) => Err(e).context(format!("ID: {} - Link: {}", id, link))?,
            _ => {}
        },
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
        cross_listed,
        prerequisites,
        offered,
        distribution,
        link: link.to_owned(),
    })
}

fn simplify_prerequisites(tokens: &mut Vec<Token>) -> bool {
    // Remove parenthesis enclosing groups of size 0 or 1

    // Search for two adjacent groups containing only one type of operator connected by the same
    // operator and combine them into the same group

    // If a higher ranking seniority token is to the left of a lower ranking one, swap them

    // If a class' id evaluates to less than a left adjacent class, swap them

    // Search for...
    // "Exactly Freshman or Exactly Sophomore or Exactly Junior or Exactly Senior or Exactly Graduate or Exactly Doctorate"
    // and replace it with "Minimum Freshman"
    // then search for...
    // "Exactly Sophomore or Exactly Junior or Exactly Senior or Exactly Graduate or Exactly Doctorate"
    // and replace it with "Minimum Sophomore"
    // all the way to...
    // "Exactly Graduate or Exactly Doctorate"
    // and replace it with "Minimum Graduate"

    false
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
pub struct Course {
    pub id: String,
    pub name: String,
    pub description: String,
    pub credits: String,
    pub cross_listed: Option<String>,
    pub prerequisites: Vec<Token>,
    pub offered: BTreeSet<String>,
    pub distribution: BTreeSet<String>,
    pub link: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
enum Logic {
    Or,
    And,
    GroupStart,
    GroupEnd,
    Equivalence,
}
impl std::fmt::Display for Logic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Logic::Or => write!(f, "or"),
            Logic::And => write!(f, "and"),
            Logic::GroupStart => write!(f, "("),
            Logic::GroupEnd => write!(f, ")"),
            Logic::Equivalence => write!(f, "/"),
        }
    }
}
const OR: Token = Token::Logical(Logic::Or);
const AND: Token = Token::Logical(Logic::And);
const START: Token = Token::Logical(Logic::GroupStart);
const END: Token = Token::Logical(Logic::GroupEnd);
const EQUIVALENCE: Token = Token::Logical(Logic::Equivalence);

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Seniority {
    Minimum(MinimumSeniority),
    Exact(ExactSeniority),
}
impl std::fmt::Display for Seniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Seniority::Minimum(s) => write!(f, "{s}"),
            Seniority::Exact(s) => write!(f, "{s}"),
        }
    }
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum MinimumSeniority {
    Certificate,
    Freshman,
    Sophomore,
    Junior,
    Senior,
    Graduate,
    Doctorate,
}
impl std::fmt::Display for MinimumSeniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MinimumSeniority::Certificate => write!(f, "Minimum Certificate"),
            MinimumSeniority::Freshman => write!(f, "Minimum Freshman"),
            MinimumSeniority::Sophomore => write!(f, "Minimum Sophomore"),
            MinimumSeniority::Junior => write!(f, "Minimum Junior"),
            MinimumSeniority::Senior => write!(f, "Minimum Senior"),
            MinimumSeniority::Graduate => write!(f, "Minimum Graduate"),
            MinimumSeniority::Doctorate => write!(f, "Minimum Doctorate"),
        }
    }
}
const CERTIFICATE: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Certificate));
const FRESHMAN: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Freshman));
const JUNIOR: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Junior));
const SENIOR: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Senior));
const GRADUATE: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Graduate));
const DOCTORATE: Token = Token::Seniority(Seniority::Exact(ExactSeniority::Doctorate));
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ExactSeniority {
    Certificate,
    Freshman,
    Sophomore,
    Junior,
    Senior,
    Graduate,
    Doctorate,
}
impl std::fmt::Display for ExactSeniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExactSeniority::Certificate => write!(f, "Exactly Certificate"),
            ExactSeniority::Freshman => write!(f, "Exactly Freshman"),
            ExactSeniority::Sophomore => write!(f, "Exactly Sophomore"),
            ExactSeniority::Junior => write!(f, "Exactly Junior"),
            ExactSeniority::Senior => write!(f, "Exactly Senior"),
            ExactSeniority::Graduate => write!(f, "Exactly Graduate"),
            ExactSeniority::Doctorate => write!(f, "Exactly Doctorate"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
enum Permission {
    DeanUndergraduate,
    DeanGraduate,
    Instructor,
}
impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Permission::DeanUndergraduate => write!(f, "Permission of Undergraduate Dean"),
            Permission::DeanGraduate => write!(f, "Permission of Graduate Dean"),
            Permission::Instructor => write!(f, "Permission of Instructor"),
        }
    }
}
const DEAN_GRADUATE: Token = Token::Permission(Permission::DeanGraduate);
const DEAN_UNDERGRADUATE: Token = Token::Permission(Permission::DeanUndergraduate);
const INSTRUCTOR: Token = Token::Permission(Permission::Instructor);

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
enum Special {
    Major(String),
    Pinnacle(bool),
    Cgpa(String),
}
impl std::fmt::Display for Special {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Special::Major(s) => write!(f, "{} Major", s.to_title_case()),
            Special::Pinnacle(s) => match s {
                true => write!(f, "Pinnacle"),
                false => write!(f, "Not Pinnacle"),
            },
            Special::Cgpa(s) => write!(f, "Cumulative GPA {}+", s),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Token {
    CoursePrereq(String),
    CourseCoreq(String),
    Logical(Logic),
    Seniority(Seniority),
    Permission(Permission),
    Special(Special),
}
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::CoursePrereq(t) => write!(f, "{}", t.to_uppercase()),
            Token::CourseCoreq(t) => write!(f, "{}", t.to_uppercase()),
            Token::Logical(t) => write!(f, "{}", t),
            Token::Seniority(t) => write!(f, "{}", t),
            Token::Permission(t) => write!(f, "{}", t),
            Token::Special(t) => write!(f, "{}", t),
        }
    }
}
fn pre(name: &str) -> Token {
    Token::CoursePrereq(name.to_owned())
}
fn cor(name: &str) -> Token {
    Token::CourseCoreq(name.to_owned())
}
fn major(name: &str) -> Token {
    Token::Special(Special::Major(name.to_owned()))
}
