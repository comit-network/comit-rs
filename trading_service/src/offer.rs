use bitcoin_rpc::Address;
use std::collections::HashMap;
use std::sync::Mutex;
use symbol::Symbol;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Offer {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub address: Address,
}

pub struct OfferRepository {
    all_offers: Mutex<HashMap<Uuid, Offer>>,
}

impl OfferRepository {
    pub fn new() -> OfferRepository {
        OfferRepository {
            all_offers: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, offer: &Offer) {
        let mut offers = self.all_offers.lock().unwrap();
        let uid = offer.uid.clone();

        offers.insert(uid, offer.clone());
    }
}
