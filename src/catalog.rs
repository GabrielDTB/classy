use crate::class::*;
use crate::get_classes::*;
pub use crate::traits::Catalog as CatalogTrait;
use serde_json;

pub struct Catalog {
    classes: Vec<Class>,
    //classes_by_id: HashMap<String, &'a Class>,
    //classes_by_department: HashMap<String, Vec<&'a Class>>,
}

impl CatalogTrait<Class> for Catalog {
    fn query_by_id(&self, id: &str) -> Option<&Class> {
        let quarry = clean(id);
        for class in self.classes {
            if quarry == clean(&class.id()) {
                return Some(&class);
            }
        }
        None
    }
    fn query_by_department(&self, department: &str) -> Vec<&Class> {
        let quarry = clean(department);
        if quarry == "*" {
            return self.classes.iter().map(|c| c).collect::<Vec<&Class>>();
        }
        self.classes
            .iter()
            .filter(|c| clean(&c.department()) == quarry)
            .collect::<Vec<&Class>>()
    }
}
impl Catalog {
    /// Returns a final populated catalog that should not be changed.
    pub async fn new_filled<'a>() -> Result<Catalog, std::io::Error> {
        println!("Checking cache paths and creating if absent...");
        for path in ["./cache/responses", "./cache/classes"] {
            tokio::fs::create_dir_all(path).await?;
        }

        println!("Listing entries in ./cache/responses...");
        let cached_response_names = std::fs::read_dir("./cache/responses")
            .unwrap()
            .map(|d| d.unwrap().file_name().to_str().unwrap().to_owned())
            .collect::<Vec<_>>();

        println!("Listing entries in ./cache/classes...");
        let cached_class_names = std::fs::read_dir("./cache/classes")
            .unwrap()
            .map(|d| d.unwrap().file_name().to_str().unwrap().to_owned())
            .collect::<Vec<_>>();

        let mut classes = Vec::with_capacity(cached_class_names.len());
        if cached_class_names.len() >= cached_response_names.len() && cached_class_names.len() != 0
        {
            println!(
                "Loading {} cached classes from ./cache/classes...",
                cached_class_names.len()
            );
            classes.extend(cached_class_names.iter().map(|name| {
                let file = std::fs::File::open(format!("./cache/classes/{name}")).unwrap();
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader).unwrap()
            }));
        } else {
            println!(
                "Loading {} cached responses from ./cache/responses...",
                cached_response_names.len()
            );
            let mut responses = cached_response_names
                .iter()
                .map(|name| {
                    let file = std::fs::File::open(format!("./cache/responses/{name}")).unwrap();
                    let reader = std::io::BufReader::new(file);
                    serde_json::from_reader(reader).unwrap()
                })
                .collect::<Vec<_>>();

            println!("Checking for missing links in cached responses...");
            'outer: loop {
                let query = query_classes(&responses).await;
                for response in query {
                    let response = match response {
                        Ok(response) => response,
                        Err(_) => continue 'outer,
                    };
                    if responses
                        .iter()
                        .filter(|r| (**r).link == response.link)
                        .count()
                        == 0
                    {
                        responses.push(response);
                    }
                }
                break;
            }

            println!(
                "Writing {} new responses to ./cache/responses...",
                responses.len() - cached_response_names.len()
            );
            for response in responses.iter() {
                let sanitized_link = response.link.replace("/", "%");
                if !cached_response_names.contains(&sanitized_link) {
                    std::fs::write(
                        format!("./cache/responses/{sanitized_link}"),
                        serde_json::to_string_pretty(&response).unwrap(),
                    )
                    .unwrap();
                };
            }

            println!("Parsing responses into Class objects...");
            classes.extend(responses.into_iter().map(|r| parse_class(r)));

            println!(
                "Checking against {} cached classes...",
                cached_class_names.len()
            );
            for class in classes.iter() {
                let short_id = class
                    .id()
                    .chars()
                    .filter(|c| *c != ' ')
                    .map(|c| c.to_lowercase().next().unwrap())
                    .collect::<String>();
                if !cached_class_names.contains(&short_id) {
                    std::fs::write(
                        format!("./cache/classes/{short_id}"),
                        serde_json::to_string_pretty(class).unwrap(),
                    )
                    .unwrap();
                }
            }
            println!("Wrote new classes to ./cache/classes...");
        }
        Ok(Catalog { classes })
    }
    pub fn departments(&self) -> Vec<String> {
        self.classes.iter().map(|c| c.department()).fold(
            Vec::new(),
            |mut departments, department| {
                if !departments.contains(&department) {
                    departments.push(department);
                }
                departments
            },
        )
    }
}

fn clean(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
}
