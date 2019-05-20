use itertools::Itertools;

#[derive(Debug)]
pub struct Offset {
    pub start: usize,
    pub length: usize,
    pub excluded_end: usize,
    pub name: String,
}

impl Offset {
    pub fn new(name: String, start: usize, excluded_end: usize, length: usize) -> Offset {
        Offset {
            name,
            start,
            excluded_end,
            length,
        }
    }

    fn row_format(&self) -> String {
        format!(
            "| `{}` | {}..{} | {} |",
            self.name, self.start, self.excluded_end, self.length
        )
    }
}

pub fn to_markdown(offsets: Vec<Offset>) -> String {
    let mut res = String::from("| Name | Byte Range | Length (bytes) |\n|:--- |:--- |:--- |");
    for offset in offsets
        .iter()
        .sorted_by(|a, b| Ord::cmp(&b.start, &a.start))
    {
        res = format!("{}\n{}", res, offset.row_format())
    }
    res
}
