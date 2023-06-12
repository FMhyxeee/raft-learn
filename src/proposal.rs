use std::sync::mpsc::SyncSender;

use raft::prelude::ConfChange;

pub struct Proposal {
    pub normal: Option<(u16, String)>,
    pub conf_change: Option<ConfChange>,
    pub transfer_leader: Option<u16>,
    // If it's proposed, it will be set to the index of the entry.
    pub proposed: u64,
    pub propose_sucess: SyncSender<bool>,
}