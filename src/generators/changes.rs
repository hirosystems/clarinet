#[derive(Clone, Debug)]
pub struct FileCreation {
    pub comment: String,
    pub name: String,
    pub content: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct DirectoryCreation {
    pub comment: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct TOMLEdition {
    pub comment: String,
    pub path: String,
    pub section: String,
    pub content: String,
    pub index: i16,
}

#[derive(Clone, Debug)]
pub enum Changes {
    AddFile(FileCreation),
    AddDirectory(DirectoryCreation),
    EditTOML(TOMLEdition),
}
