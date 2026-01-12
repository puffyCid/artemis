use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TriageOptions {
    pub description: String,
    pub author: String,
    pub version: f32,
    pub id: String,
    pub recreate_directories: bool,
    pub targets: Vec<TriageTargets>,
}

#[derive(Debug, Deserialize)]
pub struct TriageTargets {
    pub name: String,
    pub category: String,
    pub path: String,
    pub recursive: bool,
    pub file_mask: String,
    pub save_as_filename: Option<String>,
    pub always_add_to_queue: bool,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub comment: Option<String>,
}

impl Default for TriageTargets {
    fn default() -> Self {
        TriageTargets {
            name: String::from("Default test"),
            category: String::from("Default category"),
            path: String::new(),
            recursive: false,
            file_mask: String::from("*"),
            save_as_filename: None,
            always_add_to_queue: false,
            min_size: None,
            max_size: None,
            comment: None,
        }
    }
}
