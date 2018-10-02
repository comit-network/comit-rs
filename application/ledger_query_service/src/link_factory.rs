#[derive(Debug)]
pub struct LinkFactory {
    port: Option<u16>,
    host: String,
    protocol: String,
}

impl LinkFactory {
    pub fn new<P: Into<String>, H: Into<String>>(protocol: P, host: H, port: Option<u16>) -> Self {
        LinkFactory {
            port,
            host: host.into(),
            protocol: protocol.into(),
        }
    }

    pub fn create_link<S: Into<String>>(&self, path: S) -> String {
        let port = self
            .port
            .map(|port| format!(":{}", port))
            .unwrap_or_default();

        let path = {
            let path = path.into();

            if path.starts_with('/') {
                path
            } else {
                String::from("/") + &path
            }
        };

        format!("{}://{}{}{}", self.protocol, self.host, port, path)
    }
}
#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn create_link_with_port() {
        let link_factory = LinkFactory::new("http", "localhost", Some(8000));

        assert_that(&link_factory.create_link("/foo/bar"))
            .is_equal_to(&String::from("http://localhost:8000/foo/bar"));
    }

    #[test]
    fn create_link_without_port() {
        let link_factory = LinkFactory::new("http", "example.org", None);

        assert_that(&link_factory.create_link("/foo/bar"))
            .is_equal_to(&String::from("http://example.org/foo/bar"));
    }

    #[test]
    fn handle_no_leading_slash() {
        let link_factory = LinkFactory::new("http", "example.org", None);

        assert_that(&link_factory.create_link("foo/bar"))
            .is_equal_to(&String::from("http://example.org/foo/bar"));
    }

}
