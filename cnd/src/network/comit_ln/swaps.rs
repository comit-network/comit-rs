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
    /// In role of Bob; swaps exist in here if Bob creates the swap _after_ an
    /// announce message is received from Alice (and up until Bob creates the
    /// swap).
    pending_creation: HashMap<SwapDigest, (PeerId, ReplySubstream<NegotiatedSubstream>)>,

    swaps: HashMap<LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams>,
    swap_ids: HashMap<LocalSwapId, SharedSwapId>,
}

impl Swaps {
    pub fn insert_pending_creation(
        &mut self,
        digest: SwapDigest,
        peer: PeerId,
        io: ReplySubstream<NegotiatedSubstream>,
    ) {
        self.pending_creation.insert(digest, (peer, io));
    }

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

    pub fn move_to_pending_confirmation(&mut self, digest: SwapDigest, local_swap_id: LocalSwapId) {
        self.pending_confirmation.insert(digest, local_swap_id);
    }

    pub fn move_to_pending_announcement(&mut self, digest: SwapDigest, local_swap_id: LocalSwapId) {
        self.pending_announcement.insert(digest, local_swap_id);
    }

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

    pub fn get_created_swap(
        &self,
        local_swap_id: &LocalSwapId,
    ) -> Option<HanEtherereumHalightBitcoinCreateSwapParams> {
        self.swaps.get(local_swap_id).cloned()
    }

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
