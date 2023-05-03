
pub struct Tp2File {
    pub backup: String,
    pub author: String,
    pub flags: Vec<Tp2Flag>,
    pub languages: Vec<Language>,
    pub components: Vec<Component>,
}

pub enum Tp2Flag {

}

pub struct Language {
    name: String,
    directory: String,
    default_language_tra: Vec<String>,
}

pub struct Component {
    pub name: String,
    pub flag: Vec<ComponentFlag>,
    pub action: Vec<ComponentAction>,
}
