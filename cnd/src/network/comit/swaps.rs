use crate::{
    swap_protocols::{Herc20HalightBitcoinCreateSwapParams, LocalSwapId, SharedSwapId},
    timestamp::Timestamp,
};
use ::comit::network::protocols::announce::{protocol::ReplySubstream, SwapDigest};
use digest::Digest;
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

/// T is ReplySubstream<NegotiatedSubstream>
#[derive(Debug)]
pub struct Swaps<T> {
    /// In role of Alice; swaps exist in here once a swap is created by Alice
    /// (and up until an announce confirmation is received from Bob).
    pending_confirmation: HashMap<SwapDigest, LocalSwapId>,

    /// In role of Bob; swaps exist in here if Bob creates the swap _before_ an
    /// announce message is received from Alice (and up until the announce
    /// message arrives).
    pending_announcement: HashMap<SwapDigest, LocalSwapId>,
    /// In role of Bob; swaps exist in here if Bob receives an announce message
    /// from Alice _before_ Bob creates the swap (and up until Bob creates the
    /// swap).
    pending_creation: HashMap<SwapDigest, (PeerId, T)>,

    /// Stores the swap as soon as it is created
    swaps: HashMap<LocalSwapId, Herc20HalightBitcoinCreateSwapParams>,

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
    /// Gets a swap that was created
    pub fn get_created_swap(
        &self,
        local_swap_id: &LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        self.swaps.get(local_swap_id).cloned()
    }

    /// Alice created and announced it a swap and is waiting for a confirmation
    /// from Bob
    pub fn create_as_pending_confirmation(
        &mut self,
        digest: SwapDigest,
        local_swap_id: LocalSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> Result<(), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        self.swaps.insert(local_swap_id, create_swap_params);

        self.pending_confirmation
            .insert(digest.clone(), local_swap_id);

        self.timestamps.insert(digest, Timestamp::now());

        Ok(())
    }

    /// Alice moves a swap announced (pending confirmation) to communicate upon
    /// receiving a confirmation from Bob
    pub fn move_pending_confirmation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        shared_swap_id: SharedSwapId,
    ) -> Option<(LocalSwapId, Herc20HalightBitcoinCreateSwapParams)> {
        let local_swap_id = match self.pending_confirmation.remove(digest) {
            Some(local_swap_id) => local_swap_id,
            None => return None,
        };

        let create_params = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return None,
        };

        self.swap_ids.insert(local_swap_id, shared_swap_id);

        Some((local_swap_id, create_params.clone()))
    }

    /// Bob created a swap and it is pending announcement
    pub fn create_as_pending_announcement(
        &mut self,
        digest: SwapDigest,
        local_swap_id: LocalSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> Result<(), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        self.swaps.insert(local_swap_id, create_swap_params);

        self.pending_announcement
            .insert(digest.clone(), local_swap_id);

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
    /// receiving an announcement and replying to it
    pub fn move_pending_announcement_to_communicate(
        &mut self,
        digest: &SwapDigest,
        peer_id: &PeerId,
    ) -> Result<(SharedSwapId, Herc20HalightBitcoinCreateSwapParams), Error> {
        let local_swap_id = match self.pending_announcement.get(&digest) {
            Some(local_swap_id) => local_swap_id,
            None => return Err(Error::NotFound),
        };

        let create_params = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return Err(Error::InternalFailure),
        };

        if *peer_id != create_params.peer.peer_id {
            return Err(Error::PeerIdMismatch);
        }

        let local_swap_id = self
            .pending_announcement
            .remove(digest)
            .expect("We did a `get` on the hashmap already.");

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id.clone());

        Ok((shared_swap_id, create_params.clone()))
    }

    /// Bob moves a swap that was announced and pending creation to communicate
    /// after receiving an announcement from Alice
    pub fn move_pending_creation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        local_swap_id: LocalSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> Result<(SharedSwapId, PeerId, T), Error> {
        if self.swaps.get(&local_swap_id).is_some() {
            return Err(Error::AlreadyExists);
        }

        let (peer, _) = match self.pending_creation.get(&digest) {
            Some(value) => value,
            None => return Err(Error::NotFound),
        };

        if *peer != create_swap_params.peer.peer_id {
            return Err(Error::PeerIdMismatch);
        }

        let (peer, io) = self
            .pending_creation
            .remove(&digest)
            .expect("Get already done");

        self.swaps.insert(local_swap_id, create_swap_params);

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id.clone());

        Ok((shared_swap_id, peer, io))
    }

    /// Either role finalizes a swap that was in the communication phase
    /// This also proceeds with clean up from the various _pending_ stores.
    pub fn finalize_swap(
        &mut self,
        shared_swap_id: &SharedSwapId,
    ) -> Result<(LocalSwapId, Herc20HalightBitcoinCreateSwapParams), Error> {
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

        let create_params = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return Err(Error::NotFound),
        };

        self.pending_announcement
            .retain(|_, id| *id != *local_swap_id);

        let finalized_digest = create_params.digest();

        self.timestamps
            .retain(|digest, _| *digest != finalized_digest);

        Ok((*local_swap_id, create_params.clone()))
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
            swap_ids: Default::default(),
            timestamps: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset, asset::ethereum::FromWei, identity, network::DialInformation, swap_protocols::Role,
    };
    use digest::Digest;

    impl<T> Swaps<T> {
        /// Gets a swap that was announced
        pub fn get_announced_swap(
            &self,
            local_swap_id: &LocalSwapId,
        ) -> Option<(SharedSwapId, Herc20HalightBitcoinCreateSwapParams)> {
            let create_params = match self.swaps.get(local_swap_id) {
                Some(create_params) => create_params,
                None => return None,
            };

            let shared_swap_id = match self.swap_ids.get(local_swap_id) {
                Some(shared_swap_id) => shared_swap_id,
                None => return None,
            };

            Some((*shared_swap_id, create_params.clone()))
        }
    }

    fn create_params() -> Herc20HalightBitcoinCreateSwapParams {
        Herc20HalightBitcoinCreateSwapParams {
            role: Role::Alice,
            peer: DialInformation {
                peer_id: PeerId::random(),
                address_hint: None,
            },
            ethereum_identity: identity::Ethereum::random(),
            ethereum_absolute_expiry: 12345.into(),
            ethereum_amount: asset::Erc20Quantity::from_wei(9_001_000_000_000_000_000_000u128),
            lightning_identity: identity::Lightning::random(),
            lightning_cltv_expiry: 12345.into(),
            lightning_amount: asset::Bitcoin::from_sat(1_000_000_000),
            token_contract: identity::Ethereum::random(),
        }
    }

    #[test]
    fn created_swap_as_pending_confirmation_can_be_retrieved() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        let creation = swaps.create_as_pending_confirmation(
            digest,
            local_swap_id.clone(),
            create_params.clone(),
        );

        assert!(creation.is_ok());
        let created_swap = swaps.get_created_swap(&local_swap_id);
        assert!(created_swap.is_some());
        assert_eq!(created_swap.unwrap(), create_params)
    }

    #[test]
    fn created_swap_as_pending_announcement_can_be_retrieved() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        let creation = swaps.create_as_pending_announcement(
            digest,
            local_swap_id.clone(),
            create_params.clone(),
        );

        assert!(creation.is_ok());
        let created_swap = swaps.get_created_swap(&local_swap_id);
        assert!(created_swap.is_some());
        assert_eq!(created_swap.unwrap(), create_params)
    }

    #[test]
    fn given_alice_creates_dupe_swap_then_stored_params_are_unchanged() {
        let first_create_params = create_params();
        let mut second_create_params = first_create_params.clone();
        // Ethereum identity is not part of the digest so both swaps should be
        // considered the same
        second_create_params.ethereum_identity = identity::Ethereum::random();

        let digest = first_create_params.digest();
        let second_digest = second_create_params.digest();

        // The test is based on this assumption so making sure it's true
        assert_eq!(digest, second_digest);

        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        let _ = swaps.create_as_pending_confirmation(
            digest.clone(),
            local_swap_id.clone(),
            first_create_params.clone(),
        );

        let stored_params = swaps.get_created_swap(&local_swap_id).unwrap();

        assert_eq!(stored_params, first_create_params);

        let creation =
            swaps.create_as_pending_confirmation(digest, local_swap_id, second_create_params);

        assert!(creation.is_err());
        let stored_params = swaps.get_created_swap(&local_swap_id).unwrap();
        assert_eq!(stored_params, first_create_params);
    }

    #[test]
    fn given_bob_creates_dupe_swap_before_announcement_then_stored_params_are_unchanged() {
        let first_create_params = create_params();
        let mut second_create_params = first_create_params.clone();

        // Ethereum identity is not part of the digest so both swaps should be
        // considered the same
        second_create_params.ethereum_identity = identity::Ethereum::random();

        let digest = first_create_params.digest();
        let second_digest = second_create_params.digest();

        // The test is based on this assumption so making sure it's true
        assert_eq!(digest, second_digest);

        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        swaps
            .create_as_pending_announcement(
                digest.clone(),
                local_swap_id.clone(),
                first_create_params.clone(),
            )
            .unwrap();

        let stored_params = swaps.get_created_swap(&local_swap_id).unwrap();

        assert_eq!(stored_params, first_create_params);

        let second_creation =
            swaps.create_as_pending_announcement(digest, local_swap_id, second_create_params);

        assert!(second_creation.is_err());
        let stored_params = swaps.get_created_swap(&local_swap_id).unwrap();
        assert_eq!(stored_params, first_create_params);
    }

    #[test]
    fn from_creation_to_finalisation_for_alice() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        swaps
            .create_as_pending_confirmation(digest.clone(), local_swap_id, create_params.clone())
            .unwrap();

        let shared_swap_id = SharedSwapId::default();
        let (stored_local_swap_id, stored_create_params) = swaps
            .move_pending_confirmation_to_communicate(&digest, shared_swap_id)
            .unwrap();

        assert_eq!(local_swap_id, stored_local_swap_id);
        assert_eq!(create_params, stored_create_params);

        let (stored_shared_swap_id, stored_create_params) =
            swaps.get_announced_swap(&local_swap_id).unwrap();

        assert_eq!(shared_swap_id, stored_shared_swap_id);
        assert_eq!(create_params, stored_create_params);

        let (_, stored_create_params) = swaps.finalize_swap(&shared_swap_id).unwrap();

        assert_eq!(create_params, stored_create_params);
        assert!(!swaps.swap_in_pending_hashmaps(&digest));
    }

    #[test]
    fn from_creation_then_announcement_to_finalisation_for_bob() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();

        let mut swaps = Swaps::<ReplySubstream<NegotiatedSubstream>>::default();

        swaps
            .create_as_pending_announcement(digest.clone(), local_swap_id, create_params.clone())
            .unwrap();

        let (shared_swap_id, stored_create_params) = swaps
            .move_pending_announcement_to_communicate(&digest, &create_params.peer.peer_id)
            .unwrap();

        assert_eq!(create_params, stored_create_params);

        let (stored_shared_swap_id, stored_create_params) =
            swaps.get_announced_swap(&local_swap_id).unwrap();

        assert_eq!(shared_swap_id, stored_shared_swap_id);
        assert_eq!(create_params, stored_create_params);

        let (_local_swap_id, stored_create_params) = swaps.finalize_swap(&shared_swap_id).unwrap();

        assert_eq!(create_params, stored_create_params);
        assert!(!swaps.swap_in_pending_hashmaps(&digest));
    }

    #[test]
    fn from_announcement_then_creation_to_finalisation_for_bob() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::default();

        swaps
            .insert_pending_creation(digest.clone(), create_params.peer.peer_id.clone(), ())
            .unwrap();

        let (shared_swap_id, _peer, _io) = swaps
            .move_pending_creation_to_communicate(&digest, local_swap_id, create_params.clone())
            .unwrap();

        let (stored_shared_swap_id, stored_create_params) =
            swaps.get_announced_swap(&local_swap_id).unwrap();

        assert_eq!(shared_swap_id, stored_shared_swap_id);
        assert_eq!(create_params, stored_create_params);

        let (stored_local_swap_id, stored_create_params) =
            swaps.finalize_swap(&shared_swap_id).unwrap();

        assert_eq!(local_swap_id, stored_local_swap_id);
        assert_eq!(create_params, stored_create_params);
        assert!(!swaps.swap_in_pending_hashmaps(&digest));
    }

    #[test]
    fn old_pending_swaps_are_cleaned_up() {
        let mut swaps = Swaps::<()>::default();

        let create_params1 = create_params();
        let digest1 = create_params1.digest();
        let create_params2 = create_params();
        let digest2 = create_params2.digest();
        let create_params3 = create_params();
        let digest3 = create_params3.digest();

        swaps
            .create_as_pending_confirmation(digest1.clone(), LocalSwapId::default(), create_params1)
            .unwrap();

        swaps
            .create_as_pending_announcement(digest2.clone(), LocalSwapId::default(), create_params2)
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
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<ReplySubstream<NegotiatedSubstream>>::default();

        swaps
            .create_as_pending_announcement(digest.clone(), local_swap_id, create_params.clone())
            .unwrap();
        let (shared_swap_id, _) = swaps
            .move_pending_announcement_to_communicate(&digest, &create_params.peer.peer_id)
            .unwrap();

        swaps.get_announced_swap(&local_swap_id).unwrap();

        swaps.finalize_swap(&shared_swap_id).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
        let time = Timestamp::now();

        swaps.clean_up_pending_swaps(time);

        assert!(swaps.get_announced_swap(&local_swap_id).is_some());
    }

    #[test]
    fn young_pending_swaps_are_not_cleaned_up() {
        let mut swaps = Swaps::<()>::default();

        let create_params1 = create_params();
        let digest1 = create_params1.digest();
        let create_params2 = create_params();
        let digest2 = create_params2.digest();
        let create_params3 = create_params();
        let digest3 = create_params3.digest();

        let time = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_secs(1));

        swaps
            .create_as_pending_confirmation(digest1.clone(), LocalSwapId::default(), create_params1)
            .unwrap();

        swaps
            .create_as_pending_announcement(digest2.clone(), LocalSwapId::default(), create_params2)
            .unwrap();

        swaps
            .insert_pending_creation(digest3.clone(), PeerId::random(), ())
            .unwrap();

        swaps.clean_up_pending_swaps(time);

        assert!(swaps.swap_in_pending_hashmaps(&digest1));
        assert!(swaps.swap_in_pending_hashmaps(&digest2));
        assert!(swaps.swap_in_pending_hashmaps(&digest3));
    }

    #[test]
    fn given_bob_creates_dupe_swap_after_announcement_then_stored_params_are_unchanged() {
        let first_create_params = create_params();
        let mut second_create_params = first_create_params.clone();
        second_create_params.ethereum_identity = identity::Ethereum::random();

        let digest = first_create_params.digest();
        let second_digest = second_create_params.digest();

        assert_eq!(digest, second_digest);

        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::default();

        swaps
            .insert_pending_creation(digest.clone(), first_create_params.peer.peer_id.clone(), ())
            .unwrap();

        let (shared_swap_id, _peer, _io) = swaps
            .move_pending_creation_to_communicate(
                &digest,
                local_swap_id,
                first_create_params.clone(),
            )
            .unwrap();

        let (stored_shared_swap_id, stored_create_params) =
            swaps.get_announced_swap(&local_swap_id).unwrap();

        assert_eq!(stored_shared_swap_id, shared_swap_id);
        assert_eq!(stored_create_params, first_create_params);

        let res = swaps.move_pending_creation_to_communicate(
            &digest,
            local_swap_id,
            second_create_params,
        );

        assert!(res.is_err());
        let (stored_shared_swap_id, stored_create_params) =
            swaps.get_announced_swap(&local_swap_id).unwrap();
        assert_eq!(stored_shared_swap_id, shared_swap_id);
        assert_eq!(stored_create_params, first_create_params);
    }

    #[test]
    fn given_move_pending_creation_to_communicate_errors_then_no_side_effects() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::<()>::default();

        let res = swaps.move_pending_creation_to_communicate(&digest, local_swap_id, create_params);

        assert!(res.is_err());
        let res = swaps.get_announced_swap(&local_swap_id);
        assert!(res.is_none());
    }

    #[test]
    fn given_bob_receives_announcement_with_wrong_peer_id_then_error() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();

        let mut swaps = Swaps::<()>::default();

        swaps
            .create_as_pending_announcement(digest.clone(), local_swap_id, create_params)
            .unwrap();

        let res = swaps.move_pending_announcement_to_communicate(&digest, &PeerId::random());

        assert_eq!(res, Err(Error::PeerIdMismatch));
    }

    #[test]
    fn given_bob_receives_creation_with_different_peer_id_then_error() {
        let create_params = create_params();
        let digest = create_params.digest();
        let local_swap_id = LocalSwapId::default();

        let mut swaps = Swaps::<()>::default();

        let _ = swaps.insert_pending_creation(digest.clone(), PeerId::random(), ());

        let res = swaps.move_pending_creation_to_communicate(&digest, local_swap_id, create_params);

        assert_eq!(res, Err(Error::PeerIdMismatch));
    }
}
