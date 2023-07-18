pub use crate::traits::Class as ClassTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Class {
    department: String,
    department_name: String,
    discriminator: u8,
    title: String,
    description: String,
    credits: u8,
    prerequisites: String,
    offered: Vec<String>,
    cross_listings: Vec<String>,
    distributions: Vec<String>,
    url: String,
}

impl Class {
    pub fn new(
        department: String,
        department_name: String,
        discriminator: u8,
        title: String,
        description: String,
        credits: u8,
        prerequisites: String,
        offered: Vec<String>,
        cross_listings: Vec<String>,
        distributions: Vec<String>,
        url: String,
    ) -> Self {
        Self {
            department,
            department_name,
            discriminator,
            title,
            description,
            credits,
            prerequisites,
            offered,
            cross_listings,
            distributions,
            url,
        }
    }
}

impl ClassTrait for Class {
    fn id(&self) -> String {
        format!("{} {}", self.department, self.discriminator)
    }
    fn department(&self) -> String {
        self.department.clone()
    }
    fn department_name(&self) -> String {
        self.department_name.clone()
    }
    fn discriminator(&self) -> u8 {
        self.discriminator
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    fn credits(&self) -> u8 {
        self.credits
    }
    fn prerequisites(&self) -> String {
        self.prerequisites.clone()
    }
    fn offered(&self) -> Vec<String> {
        self.offered.clone()
    }
    fn cross_listings(&self) -> Vec<String> {
        self.cross_listings.clone()
    }
    fn distributions(&self) -> Vec<String> {
        self.distributions.clone()
    }
    fn url(&self) -> String {
        self.url.clone()
    }
}
