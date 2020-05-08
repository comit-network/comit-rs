use crate::{
    network::{
        comit_ln::SwapExists,
        protocols::announce::{protocol::ReplySubstream, SwapDigest},
    },
    swap_protocols::{HanEtherereumHalightBitcoinCreateSwapParams, LocalSwapId, SharedSwapId},
};
use libp2p::{swarm::NegotiatedSubstream, PeerId};
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Swaps {
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
    pending_creation: HashMap<SwapDigest, (PeerId, ReplySubstream<NegotiatedSubstream>)>,

    /// Stores the swap as soon as it is created
    swaps: HashMap<LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams>,

    /// Stores the shared swap id as soon as it is known.
    /// Bob defines the shared swap id when he confirms the swap by replying to
    /// an announce message from Alice.
    swap_ids: HashMap<LocalSwapId, SharedSwapId>,
}

impl Swaps {
    /// Alice or Bob creates a swap.
    /// A second function will be used to mark the swap as _pending
    /// confirmation_ or _pending announcement_ depending on the role.
    pub fn create_swap(
        &mut self,
        digest: &SwapDigest,
        local_swap_id: LocalSwapId,
        create_swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        if self.pending_announcement.contains_key(&digest)
            || self.pending_confirmation.contains_key(&digest)
        {
            return Err(anyhow::Error::from(SwapExists));
        }

        self.swaps.insert(local_swap_id, create_swap_params);

        Ok(())
    }

    /// Gets a swap that was created
    pub fn get_created_swap(
        &self,
        local_swap_id: &LocalSwapId,
    ) -> Option<HanEtherereumHalightBitcoinCreateSwapParams> {
        self.swaps.get(local_swap_id).cloned()
    }

    /// Gets a swap that was announced
    pub fn get_announced_swap(
        &self,
        local_swap_id: &LocalSwapId,
    ) -> Option<(SharedSwapId, HanEtherereumHalightBitcoinCreateSwapParams)> {
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

    /// Alice announced a swap and is waiting for a confirmation from Bob
    pub fn move_to_pending_confirmation(&mut self, digest: SwapDigest, local_swap_id: LocalSwapId) {
        self.pending_confirmation.insert(digest, local_swap_id);
    }

    /// Alice moves a swap announced (pending confirmation) to communicate upon
    /// receiving a confirmation from Bob
    pub fn move_pending_confirmation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        shared_swap_id: SharedSwapId,
    ) -> Option<(LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams)> {
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

    /// Bob moves a swap to _pending announcement_ after creation.
    pub fn move_to_pending_announcement(&mut self, digest: SwapDigest, local_swap_id: LocalSwapId) {
        self.pending_announcement.insert(digest, local_swap_id);
    }

    /// Bob received an announcement for a swap not yet created.
    pub fn insert_pending_creation(
        &mut self,
        digest: SwapDigest,
        peer: PeerId,
        io: ReplySubstream<NegotiatedSubstream>,
    ) {
        self.pending_creation.insert(digest, (peer, io));
    }

    /// Bob: get a swap created but not yet announced
    pub fn get_pending_announcement(
        &self,
        digest: &SwapDigest,
    ) -> Option<(LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams)> {
        self.pending_announcement
            .get(digest)
            .and_then(|local_swap_id| {
                self.swaps
                    .get(local_swap_id)
                    .map(|create_params| (*local_swap_id, create_params.clone()))
            })
    }

    /// Bob: Move a swap from pending announcement (created) to communicate upon
    /// receiving an announcement and replying to it
    pub fn move_pending_announcement_to_communicate(
        &mut self,
        digest: &SwapDigest,
    ) -> Option<(SharedSwapId, HanEtherereumHalightBitcoinCreateSwapParams)> {
        let local_swap_id = match self.pending_announcement.remove(digest) {
            Some(local_swap_id) => local_swap_id,
            None => return None,
        };

        let create_params = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return None,
        };

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id.clone());

        Some((shared_swap_id, create_params.clone()))
    }

    /// Bob moves a swap that was announced and pending creation to communicate
    /// after receiving an announcement from Alice
    pub fn move_pending_creation_to_communicate(
        &mut self,
        digest: &SwapDigest,
        local_swap_id: LocalSwapId,
    ) -> Option<(SharedSwapId, PeerId, ReplySubstream<NegotiatedSubstream>)> {
        let (peer, io) = match self.pending_creation.remove(&digest) {
            Some(value) => value,
            None => return None,
        };

        let shared_swap_id = SharedSwapId::default();
        self.swap_ids.insert(local_swap_id, shared_swap_id.clone());

        Some((shared_swap_id, peer, io))
    }

    /// Either role finalizes a swap that was in the communication phase
    /// This also proceeds with clean up from the various _pending_ stores.
    pub fn finalize_swap(
        &mut self,
        shared_swap_id: &SharedSwapId,
    ) -> Option<(LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams)> {
        let local_swap_id = match self.swap_ids.iter().find_map(|(key, value)| {
            if *value == *shared_swap_id {
                Some(key)
            } else {
                None
            }
        }) {
            Some(local_swap_id) => local_swap_id,
            None => return None,
        };

        let create_params = match self.swaps.get(&local_swap_id) {
            Some(create_params) => create_params,
            None => return None,
        };

        self.pending_announcement
            .retain(|_, id| *id != *local_swap_id);

        Some((*local_swap_id, create_params.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        asset::ethereum::FromWei,
        identity,
        network::DialInformation,
        swap_protocols::{EthereumIdentity, Role},
    };
    use digest::Digest;

    fn create_params() -> HanEtherereumHalightBitcoinCreateSwapParams {
        HanEtherereumHalightBitcoinCreateSwapParams {
            role: Role::Alice,
            peer: DialInformation {
                peer_id: PeerId::random(),
                address_hint: None,
            },
            ethereum_identity: EthereumIdentity::from(identity::Ethereum::random()),
            ethereum_absolute_expiry: 12345.into(),
            ethereum_amount: asset::Ether::from_wei(9_001_000_000_000_000_000_000u128),
            lightning_identity: identity::Lightning::random(),
            lightning_cltv_expiry: 12345.into(),
            lightning_amount: asset::Bitcoin::from_sat(1_000_000_000),
        }
    }

    #[test]
    fn created_swap_can_be_retrieved() {
        let create_params = create_params();
        let digest = create_params.clone().digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::default();

        let creation = swaps.create_swap(&digest, local_swap_id.clone(), create_params.clone());

        assert!(creation.is_ok());

        let create_swap = swaps.get_created_swap(&local_swap_id);

        assert!(create_swap.is_some());

        assert_eq!(create_swap.unwrap(), create_params)
    }

    #[test]
    fn same_swap_cannot_be_inserted_twice() {
        let create_params = create_params();
        let digest = create_params.clone().digest();
        let local_swap_id = LocalSwapId::default();
        let mut swaps = Swaps::default();

        let creation1 = swaps.create_swap(&digest, local_swap_id.clone(), create_params.clone());

        let creation2 = swaps.create_swap(&digest, local_swap_id.clone(), create_params.clone());

        assert!(creation2.is_err())
    }
}
