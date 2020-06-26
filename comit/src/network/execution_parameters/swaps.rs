use crate::{network::*, LocalSwapId, Role, SharedSwapId, Timestamp};
use libp2p::{swarm::NegotiatedSubstream, PeerId};
use std::collections::HashMap;

#[derive(thiserror::Error, Clone, Copy, Debug, PartialEq)]
pub enum Error {
    #[error("A swap with the same digest already exists")]
    AlreadyExists,
    #[error("An announced swap with the same digest is already pending creation")]
    AlreadyPendingCreation,
    #[error("Swap not found")]
    NotFound,
    #[error("Peer id from announcement and creation are not matching")]
    PeerIdMismatch,
    #[error("Internal Failure encountered")]
    InternalFailure,
}

/// Tracks the state of a swap in the communication phase.
///
/// We use a type parameter for the substream type so we can write unit tests
/// without creating actual substreams.
#[derive(Debug)]
pub struct Swaps<T = ReplySubstream<NegotiatedSubstream>> {
    /// In role of Alice; swaps exist in here once a swap is created by Alice
    /// (and up until an announce confirmation is received from Bob).
    pending_confirmation: HashMap<SwapDigest, LocalSwapId>,

    /// In role of Bob; swaps exist in here if Bob creates the swap _before_ an
    /// announce message is received from Alice (and up until the announce
    /// message arrives).
    pending_announcement: HashMap<SwapDigest, (LocalSwapId, PeerId)>,

    /// In role of Bob; swaps exist in here if Bob receives an announce message
    /// from Alice _before_ Bob creates the swap (and up until Bob creates the
    /// swap).
    pending_creation: HashMap<SwapDigest, (PeerId, T)>,

    /// Stores the swap as soon as it is created.
    swaps: HashMap<LocalSwapId, LocalData>,

    /// Stores the swap role as soon as the swap is created.
    roles: HashMap<LocalSwapId, Role>,

    /// Stores the shared swap id as soon as it is known.
    /// Bob defines the shared swap id when he confirms the swap by replying to
    /// an announce message from Alice.
    swap_ids: HashMap<LocalSwapId, SharedSwapId>,

    /// Stores timestamps from when we are first aware of a swap.
    /// This is used for tracking if a swap that was not actioned on can be
    /// removed.
    timestamps: HashMap<SwapDigest, Timestamp>,
}

impl<T> Swaps<T> {
    pub fn get_local_swap_id(&self, shared_swap_id: SharedSwapId) -> Option<LocalSwapId> {
        for (local, shared) in self.swap_ids.iter() {
            if *shared == shared_swap_id {
                return Some(*local);
            }
        }
        None
    }

    /// Gets a swap that was created
    pub fn get_local_data(&self, local_swap_id: &LocalSwapId) -> Option<LocalData> {
        self.swaps.get(local_swap_id).cloned()
    }

    /// Alice created and announced the swap, it is now waiting for a
    /// confirmation from Bob.
    pub fn create_as_pending_confirmation(
        &mut self,
        digest: SwapDigest,
        local_swap_id: LocalSwapId,
        data: LocalData,
    ) -> Result<(), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        self.swaps.insert(local_swap_id, data);

        self.pending_confirmation
            .insert(digest.clone(), local_swap_id);

        self.timestamps.insert(digest, Timestamp::now());

        Ok(())
    }

    /// Alice moves an announced swap (pending confirmation) to communicate upon
    /// receiving a confirmation from Bob.
    pub fn move_pending_confirmation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        shared_swap_id: SharedSwapId,
    ) -> Option<LocalData> {
        let local_swap_id = match self.pending_confirmation.remove(digest) {
            Some(local_swap_id) => local_swap_id,
            None => return None,
        };

        let data = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return None,
        };

        self.swap_ids.insert(local_swap_id, shared_swap_id);

        Some(*data)
    }

    /// Bob created a swap and it is pending announcement.
    pub fn create_as_pending_announcement(
        &mut self,
        digest: SwapDigest,
        local_swap_id: LocalSwapId,
        peer_id: PeerId,
        data: LocalData,
    ) -> Result<(), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        self.swaps.insert(local_swap_id, data);

        self.pending_announcement
            .insert(digest.clone(), (local_swap_id, peer_id));

        self.timestamps.insert(digest, Timestamp::now());

        Ok(())
    }

    /// Bob received an announcement for a swap not yet created.
    pub fn insert_pending_creation(
        &mut self,
        digest: SwapDigest,
        peer: PeerId,
        io: T,
    ) -> Result<(), Error> {
        if self
            .pending_creation
            .insert(digest.clone(), (peer, io))
            .is_some()
        {
            return Err(Error::AlreadyPendingCreation);
        }

        self.timestamps.insert(digest, Timestamp::now());

        Ok(())
    }

    /// Bob: Move a swap from pending announcement (created) to communicate upon
    /// receiving an announcement and replying to it.
    pub fn move_pending_announcement_to_communicate(
        &mut self,
        digest: &SwapDigest,
        peer_id: &PeerId,
    ) -> Result<(SharedSwapId, LocalData), Error> {
        let local_swap_id = match self.pending_announcement.get(&digest) {
            Some((swap_id, pending_peer_id)) => {
                if *peer_id != *pending_peer_id {
                    return Err(Error::PeerIdMismatch);
                }
                swap_id
            }
            None => {
                return Err(Error::NotFound);
            }
        };

        let data = match self.swaps.get(&local_swap_id) {
            Some(data) => data,
            None => return Err(Error::InternalFailure),
        };

        let (local_swap_id, _) = self
            .pending_announcement
            .remove(digest)
            .expect("We did a `get` on the hashmap already.");

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id);

        Ok((shared_swap_id, *data))
    }

    /// Bob moves a swap that was announced and pending creation to communicate
    /// after receiving an announcement from Alice
    pub fn move_pending_creation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        local_swap_id: LocalSwapId,
        peer_id: PeerId,
        data: LocalData,
    ) -> Result<(SharedSwapId, PeerId, T), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        let (stored_peer_id, _) = match self.pending_creation.get(&digest) {
            Some(value) => value,
            None => return Err(Error::NotFound),
        };

        if *stored_peer_id != peer_id {
            return Err(Error::PeerIdMismatch);
        }

        let (stored_peer_id, io) = self
            .pending_creation
            .remove(&digest)
            .expect("should not fail because we just did a get on this hashmap");

        self.swaps.insert(local_swap_id, data);

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id);

        Ok((shared_swap_id, stored_peer_id, io))
    }

    /// Either role finalizes a swap that was in the communication phase
    /// This also proceeds with clean up from the various _pending_ stores.
    pub fn finalize_swap(&mut self, shared_swap_id: &SharedSwapId) -> Result<LocalSwapId, Error> {
        let local_swap_id = match self.swap_ids.iter().find_map(|(key, value)| {
            if *value == *shared_swap_id {
                Some(key)
            } else {
                None
            }
        }) {
            Some(local_swap_id) => local_swap_id,
            None => return Err(Error::NotFound),
        };

        Ok(*local_swap_id)
    }

    /// Remove all pending (not finalized) swap older than `older_than`
    pub fn clean_up_pending_swaps(&mut self, older_than: Timestamp) {
        let digests = self
            .timestamps
            .iter()
            .filter(|(_, timestamp)| **timestamp < older_than)
            .map(|(digest, _)| digest)
            .cloned()
            .collect::<Vec<_>>();

        self.pending_confirmation
            .retain(|digest, _| !digests.contains(digest));
        self.pending_announcement
            .retain(|digest, _| !digests.contains(digest));
        self.pending_creation
            .retain(|digest, _| !digests.contains(digest));
        self.timestamps
            .retain(|digest, _| !digests.contains(digest));
    }

    /// This does not test external behaviour but the aim is to ensure we are
    /// not consuming memory for no reason.
    #[cfg(test)]
    fn swap_in_pending_hashmaps(&self, digest: &SwapDigest) -> bool {
        self.pending_confirmation.get(digest).is_some()
            || self.pending_announcement.get(digest).is_some()
            || self.pending_creation.get(digest).is_some()
            || self.timestamps.get(digest).is_some()
    }
}

impl Default for Swaps<ReplySubstream<NegotiatedSubstream>> {
    fn default() -> Self {
        Swaps {
            pending_confirmation: Default::default(),
            pending_announcement: Default::default(),
            pending_creation: Default::default(),
            swaps: Default::default(),
            roles: Default::default(),
            swap_ids: Default::default(),
            timestamps: Default::default(),
        }
    }
}

#[cfg(test)]
impl Default for Swaps<()> {
    fn default() -> Self {
        Swaps {
            pending_confirmation: Default::default(),
            pending_announcement: Default::default(),
            pending_creation: Default::default(),
            swaps: Default::default(),
            roles: Default::default(),
            swap_ids: Default::default(),
            timestamps: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::{self, ethereum::FromWei},
        identity,
        network::{swap_digest, LocalData, SwapDigest},
    };
    use digest::Digest;

    // Usage of this function relies on the fact that token_contract is
    // random. We should use property based testing.
    fn digest() -> SwapDigest {
        swap_digest::Herc20Halbit {
            ethereum_absolute_expiry: 12345.into(),
            erc20_amount: asset::Erc20Quantity::from_wei(9_001_000_000_000_000_000_000u128),
            token_contract: identity::Ethereum::random(),
            lightning_cltv_expiry: 12345.into(),
            lightning_amount: asset::Bitcoin::from_sat(1_000_000_000),
        }
        .digest()
        .into()
    }

    // The same applies here as for digest() re property based testing.
    fn local_data() -> LocalData {
        LocalData {
            secret_hash: None,
            shared_swap_id: Some(SharedSwapId::default()),
            ethereum_identity: Some(identity::Ethereum::random()),
            lightning_identity: Some(identity::Lightning::random()),
            bitcoin_identity: None,
        }
    }

    #[test]
    fn old_pending_swaps_are_cleaned_up() {
        let mut swaps = Swaps::<()>::default();

        let digest1 = digest();
        let digest2 = digest();
        let digest3 = digest();

        let data1 = local_data();
        let data2 = local_data();

        swaps
            .create_as_pending_confirmation(digest1.clone(), LocalSwapId::default(), data1)
            .unwrap();

        swaps
            .create_as_pending_announcement(
                digest2.clone(),
                LocalSwapId::default(),
                PeerId::random(),
                data2,
            )
            .unwrap();

        swaps
            .insert_pending_creation(digest3.clone(), PeerId::random(), ())
            .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
        let time = Timestamp::now();
        swaps.clean_up_pending_swaps(time);

        assert!(!swaps.swap_in_pending_hashmaps(&digest1));
        assert!(!swaps.swap_in_pending_hashmaps(&digest2));
        assert!(!swaps.swap_in_pending_hashmaps(&digest3));
    }

    #[test]
    fn old_finalized_swaps_are_not_cleaned_up() {
        let mut swaps = Swaps::<()>::default();

        let digest = digest();
        let id = LocalSwapId::default();
        let data = local_data();
        let peer_id = PeerId::random();

        swaps
            .create_as_pending_announcement(digest.clone(), id, peer_id.clone(), data)
            .unwrap();

        let (shared_swap_id, _) = swaps
            .move_pending_announcement_to_communicate(&digest, &peer_id)
            .unwrap();

        swaps.finalize_swap(&shared_swap_id).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
        let time = Timestamp::now();
        swaps.clean_up_pending_swaps(time);

        // assertions
        swaps.swaps.get(&id).expect("swap to still be in swaps");
        swaps
            .swap_ids
            .get(&id)
            .expect("swap to still be in swap_ids");
    }

    #[test]
    fn young_pending_swaps_are_not_cleaned_up() {
        let mut swaps = Swaps::<()>::default();

        let digest1 = digest();
        let digest2 = digest();
        let digest3 = digest();

        let data1 = local_data();
        let data2 = local_data();

        swaps
            .create_as_pending_confirmation(digest1.clone(), LocalSwapId::default(), data1)
            .unwrap();

        swaps
            .create_as_pending_announcement(
                digest2.clone(),
                LocalSwapId::default(),
                PeerId::random(),
                data2,
            )
            .unwrap();

        swaps
            .insert_pending_creation(digest3.clone(), PeerId::random(), ())
            .unwrap();

        let time = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_secs(1));
        swaps.clean_up_pending_swaps(time);

        assert!(swaps.swap_in_pending_hashmaps(&digest1));
        assert!(swaps.swap_in_pending_hashmaps(&digest2));
        assert!(swaps.swap_in_pending_hashmaps(&digest3));
    }

    #[test]
    fn given_bob_receives_announcement_with_wrong_peer_id_then_error() {
        let mut swaps = Swaps::<()>::default();

        let data = local_data();
        let digest = digest();

        swaps
            .create_as_pending_announcement(
                digest.clone(),
                LocalSwapId::default(),
                PeerId::random(),
                data,
            )
            .unwrap();

        let res = swaps.move_pending_announcement_to_communicate(&digest, &PeerId::random());

        assert_eq!(res, Err(Error::PeerIdMismatch));
    }

    #[test]
    fn given_bob_receives_creation_with_different_peer_id_then_error() {
        let mut swaps = Swaps::<()>::default();

        let data = local_data();
        let digest = digest();
        let local_swap_id = LocalSwapId::default();

        let _ = swaps.insert_pending_creation(digest.clone(), PeerId::random(), ());
        let res = swaps.move_pending_creation_to_communicate(
            &digest,
            local_swap_id,
            PeerId::random(),
            data,
        );

        assert_eq!(res, Err(Error::PeerIdMismatch));
    }
}
