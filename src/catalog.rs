use crate::class::Class;
use crate::traits;
use std::collections::HashMap;

pub struct Catalog<'a> {
    classes: Vec<Class>,
    classes_by_id: HashMap<String, &'a Class>,
    classes_by_department: HashMap<String, Vec<&'a Class>>,
}

impl traits::Catalog<'_, Class> for Catalog<'_> {
    fn query_by_id(&self, id: &str) -> Option<&Class> {
        match self.classes_by_id.get(
            &id.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_uppercase().next().unwrap())
                .collect::<String>(),
        ) {
            Some(class) => Some(*class),
            None => None,
        }
    }
    fn query_by_department(&self, code: &str) -> Vec<&Class> {
        self.classes_by_department
            .get(code)
            .unwrap_or(&vec![])
            .clone()
    }
}
