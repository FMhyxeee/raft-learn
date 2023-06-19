use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use raft::{
    prelude::{Message, Snapshot},
    storage::MemStorage,
    RawNode,
};
use slog::o;

use crate::utils::{example_config, is_initial_msg};

pub struct Node {
    // 原始的raft node，内部使用MemStorage作为存储
    pub raft_group: Option<RawNode<MemStorage>>,
    // 自己的消息通道
    pub my_mailbox: Receiver<Message>,
    // 发给其他节点的消息通道
    pub mailboxes: HashMap<u64, Sender<Message>>,
    // Key-Value对，模拟状态机
    pub kv_pairs: HashMap<u16, String>,
}

impl Node {
    pub fn create_raft_leader(
        id: u64,
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
        logger: &slog::Logger,
    ) -> Self {
        let mut cfg = example_config();
        cfg.id = id;
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let mut s = Snapshot::default();
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];
        let storage = MemStorage::new();
        storage.wl().apply_snapshot(s).unwrap();
        let raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
        Node {
            raft_group,
            my_mailbox,
            mailboxes,
            kv_pairs: Default::default(),
        }
    }

    pub fn create_raft_follower(
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
    ) -> Self {
        Node {
            raft_group: None,
            my_mailbox,
            mailboxes,
            kv_pairs: Default::default(),
        }
    }

    // 从消息中初始化raft follower
    pub fn initailize_raft_from_message(&mut self, msg: &Message, logger: &slog::Logger) {
        if !is_initial_msg(msg) {
            return;
        }

        let mut cfg = example_config();
        cfg.id = msg.to;
        let logger = logger.new(o!("tag" => format!("peer_{}", msg.to)));
        let storage = MemStorage::new();
        self.raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
    }

    // step a raft message, initialize the raft if need.
    // 开始处理消息
    pub fn step(&mut self, msg: Message, logger: &slog::Logger) {

        // 如果raft node没有初始化，那接受到一个消息后，进行初始化
        if self.raft_group.is_none() {
            if is_initial_msg(&msg) {
                self.initailize_raft_from_message(&msg, logger);
            } else {
                return;
            }
        }

        let raft_group = self.raft_group.as_mut().unwrap();
        let _ = raft_group.step(msg);
    }
}
