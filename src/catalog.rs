use tantivy::{
    collector::TopDocs, doc, query::QueryParser, schema::*, Index, IndexReader, ReloadPolicy,
};

use crate::class::*;
use crate::get_classes::*;
pub use crate::traits::Catalog as CatalogTrait;
use serde_json;
use std::collections::HashMap;

pub struct Catalog {
    classes: Vec<Class>,
    departments: HashMap<String, String>,
    schema: Schema,
    reader: IndexReader,
    query_parser: QueryParser,
    //classes_by_id: HashMap<String, &'a Class>,
    //classes_by_department: HashMap<String, Vec<&'a Class>>,
}

impl CatalogTrait<Class> for Catalog {
    fn query_by_id(&self, id: &str) -> Option<&Class> {
        let quarry = clean(id);
        for class in self.classes.iter() {
            if quarry == clean(&class.id()) {
                return Some(class);
            }
        }
        None
    }
    fn query_by_department(&self, department: &str) -> Vec<&Class> {
        let quarry = clean(department);
        if quarry.is_empty() {
            return self.classes.iter().collect::<Vec<&Class>>();
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

            // println!(
            //     "Writing {} new responses to ./cache/responses...",
            //     responses.len() - cached_response_names.len()
            // );
            // for response in responses.iter() {
            //     let sanitized_link = response.link.replace("/", "%");
            //     if !cached_response_names.contains(&sanitized_link) {
            //         std::fs::write(
            //             format!("./cache/responses/{sanitized_link}"),
            //             serde_json::to_string_pretty(&response).unwrap(),
            //         )
            //         .unwrap();
            //     };
            // }

            println!("Parsing responses into Class objects...");
            classes.extend(responses.into_iter().filter_map(|r| parse_class(r)));
            println!("Parsed {} classes.", classes.len());

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
                    .expect(&format!("{:#?}", class));
                }
            }
            println!("Wrote new classes to ./cache/classes.");
        }

        println!("Parsing departments from classes...");
        let mut departments = HashMap::new();
        for class in classes.iter() {
            if !departments.contains_key(&class.department()) {
                departments.insert(class.department(), class.department_name());
            }
        }
        println!("Parsed {} departments.", departments.len());

        let mut schema_builder = Schema::builder();
        let id = schema_builder.add_text_field("id", STORED);
        let title = schema_builder.add_text_field(
            "title",
            TextOptions::default()
                .set_indexing_options(TextFieldIndexing::default().set_tokenizer("en_stem")),
        );
        let body = schema_builder.add_text_field(
            "body",
            TextOptions::default()
                .set_indexing_options(TextFieldIndexing::default().set_tokenizer("en_stem")),
        );
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());
        let mut index_writer = index.writer(100_000_000).unwrap();
        for class in classes.iter() {
            index_writer
                .add_document(doc!(
                    id => class.id(),
                    title => class.title(),
                    body => class.description(),
                ))
                .unwrap();
        }
        index_writer.commit().unwrap();
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .unwrap();
        let query_parser = QueryParser::for_index(&index, vec![title, body]);

        Ok(Catalog {
            classes,
            departments,
            schema,
            reader,
            query_parser,
        })
    }
    pub fn departments(&self) -> Vec<(String, String)> {
        let mut pairs = self
            .departments
            .iter()
            .map(|s| (s.0.clone(), s.1.clone()))
            .collect::<Vec<_>>();
        pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        pairs
    }
    pub fn search(&self, query: &str, number_results: usize) -> Vec<&Class> {
        let mut classes = Vec::new();
        let searcher = self.reader.searcher();
        let query = match self.query_parser.parse_query(query) {
            Ok(value) => value,
            Err(_) => return classes,
        };
        let top_docs = match searcher.search(&query, &TopDocs::with_limit(number_results)) {
            Ok(value) => value,
            Err(_) => return classes,
        };
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).unwrap();
            let id = retrieved_doc
                .get_first(self.schema.get_field("id").unwrap())
                .unwrap()
                .as_text()
                .unwrap();
            classes.push(self.query_by_id(id).unwrap());
            // println!("{} - {}", score, id);
        }
        classes
    }
}

fn clean(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
}
