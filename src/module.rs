#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub path: Option<String>,
    pub content: Option<String>,
}
