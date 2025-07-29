use crate::lowercase::LwcString;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniqueComponent {
    pub mod_key: LwcString,
    pub index: u32,
    pub name: Option<String>,
}

impl UniqueComponent {
    pub fn short_desc(&self) -> String {
        format!("{}:{}", self.mod_key, self.index)
    }
}
