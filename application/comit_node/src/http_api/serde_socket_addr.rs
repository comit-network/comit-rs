use serde::{de, export::fmt, Deserializer};
use std::net::{Ipv4Addr, SocketAddr};

pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = SocketAddr;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an ip address with an optional port")
        }

        fn visit_str<E>(self, value: &str) -> Result<SocketAddr, E>
        where
            E: de::Error,
        {
            let parts: Vec<&str> = value.split(':').collect();

            match parts.len() {
                1 => {
                    let ip: Ipv4Addr = parts[0].parse().map_err(E::custom)?;
                    Ok(SocketAddr::from((ip, default_port())))
                }
                2 => {
                    let ip: Ipv4Addr = parts[0].parse().map_err(E::custom)?;
                    let port = parts[1].parse().map_err(E::custom)?;
                    Ok(SocketAddr::from((ip, port)))
                }
                _ => Err(E::custom("more than one ':' in socket address")),
            }
        }
    }

    deserializer.deserialize_str(Visitor)
}

fn default_port() -> u16 {
    9939
}
