pub mod option_method {

    pub fn serialize<S>(method: &Option<http::Method>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match method {
            Some(method) => serializer.serialize_str(method.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    struct MethodVisitor;

    impl<'de> serde::de::Visitor<'de> for MethodVisitor {
        type Value = Option<http::Method>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "an HTTP method")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse().map_err(E::custom).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<http::Method>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(MethodVisitor)
    }
}
