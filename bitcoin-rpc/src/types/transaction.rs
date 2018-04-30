#[derive(Deserialize, Serialize, Debug)]
pub struct TransactionId(String);

impl<'a> From<&'a str> for TransactionId {
    fn from(s: &'a str) -> Self {
        TransactionId(s.to_string())
    }
}

pub struct Transaction {

}