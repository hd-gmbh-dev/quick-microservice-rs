#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Op {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, serde::Deserialize)]
pub struct Payload<T> {
    pub op: Op,
    pub old: Option<T>,
    pub new: Option<T>,
}
