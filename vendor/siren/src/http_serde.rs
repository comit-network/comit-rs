pub mod method {

    pub fn serialize<S>(method: &http::Method, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(method.as_ref())
    }

    struct MethodVisitor;

    impl<'de> serde::de::Visitor<'de> for MethodVisitor {
        type Value = http::Method;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "an HTTP method")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse().map_err(E::custom)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<http::Method, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(MethodVisitor)
    }
}
