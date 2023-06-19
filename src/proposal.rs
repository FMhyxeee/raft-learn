use std::sync::mpsc::{self, Receiver, SyncSender};

use raft::prelude::ConfChange;

pub struct Proposal {
    pub normal: Option<(u16, String)>,
    pub conf_change: Option<ConfChange>,
    pub transfer_leader: Option<u16>,
    // If it's proposed, it will be set to the index of the entry.
    pub proposed: u64,
    pub propose_success: SyncSender<bool>,
}

impl Proposal {
    // 变更配置文件
    pub fn conf_change(cc: &ConfChange) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: None,
            conf_change: Some(cc.clone()),
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }
    // 正常发送一个(key, value)对
    pub fn normal(key: u16, value: String) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: Some((key, value)),
            conf_change: None,
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }
}
