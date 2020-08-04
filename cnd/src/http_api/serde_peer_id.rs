//! A serde module that defines how we want to serialize PeerIds on the
//! HTTP-API.

use libp2p::PeerId;
use serde::Serializer;

pub fn serialize<S>(peer_id: &PeerId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = peer_id.to_string();
    serializer.serialize_str(&string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use spectral::prelude::*;

    #[derive(Serialize)]
    struct SerializablePeerId(#[serde(with = "super")] PeerId);

    #[test]
    fn maker_id_serializes_as_expected() {
        let peer_id = SerializablePeerId(
            "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY"
                .parse()
                .unwrap(),
        );

        let got = serde_json::to_string(&peer_id).expect("failed to serialize peer id");

        assert_that(&got)
            .is_equal_to(r#""QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY""#.to_string());
    }
}
