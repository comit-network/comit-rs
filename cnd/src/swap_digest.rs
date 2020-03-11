use multihash::Multihash;

trait Digest {
    fn digest(&self) -> Multihash;
}

impl Digest for String {
    fn digest(&self) -> Multihash {
        let bytes = self.as_bytes();
        // Time the tests and take fastest hash?
        multihash::Sha3_256::digest(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_same_strings_return_same_multihash() {
        let str1 = String::from("simple string");
        let str2 = String::from("simple string");

        assert_eq!(str1.digest(), str2.digest())
    }
}
