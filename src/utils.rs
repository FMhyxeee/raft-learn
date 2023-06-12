use std::{collections::{HashMap, VecDeque}, sync::{mpsc::{Sender}, Mutex}, thread, time::Duration};

use protobuf::Message as protoMessage;
use raft::{Config, prelude::{Message, MessageType, Snapshot, Entry, EntryType, ConfChange, ConfChangeType}, storage::MemStorage, RawNode, StateRole};

use slog::error;
use regex::Regex;

use crate::proposal::Proposal;



pub fn example_config() -> Config {
    Config {
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    }
}

pub fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestPreVote
        || msg_type == MessageType::MsgRequestPreVote
        || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}


pub fn propose(raft_group: &mut RawNode<MemStorage>, proposal: &mut Proposal) {
    let last_index1 = raft_group.raft.raft_log.last_index() + 1;
    if let Some(( ref key,  ref value)) = proposal.normal {
        let data = format!("put {} {}", key, value).into_bytes();
        let _ = raft_group.propose(vec![], data);
    } else if let Some(ref cc) = proposal.conf_change {
        let _ = raft_group.propose_conf_change(vec![], cc.clone());
    } else if let Some(_transferee) = proposal.transfer_leader {
        unimplemented!();
    }
    
    let last_index2 = raft_group.raft.raft_log.last_index() + 1;

     if last_index1 == last_index2 {
        // Propose failed, don't forget to respond to the client.
        proposal.propose_success.send(false).unwrap();
     } else {
        proposal.proposed = last_index1;
     }
    
}

pub fn on_ready(
    raft_group: &mut RawNode<MemStorage>,
    kv_pairs: &mut HashMap<u16, String>,
    mailboxes: &HashMap<u64, Sender<Message>>,
    proposals: &Mutex<VecDeque<Proposal>>,
    logger: &slog::Logger,
) {
    if !raft_group.has_ready() {
        return;
    }

    let store = raft_group.raft.raft_log.store.clone();

    let mut ready = raft_group.ready();

    let handle_message = |msgs: Vec<Message>| {
        for msg in msgs {
            let to = msg.to;
            if mailboxes[&to].send(msg).is_err() {
                error!(
                    logger, 
                    "send raft message to {} fail, let Raft retry it", to
                );
            }
        }
    };

    if !ready.messages().is_empty() {
        handle_message(ready.take_messages());
    }

    if *ready.snapshot() != Snapshot::default() {
        let s = ready.snapshot().clone();
        if let Err(e) = store.wl().apply_snapshot(s) {
            error!(
                logger,
                "apply snapshot fail: {:?}, need to retry or panic", e
            );
            return;
        }
    }

    let mut handle_committed_entries = 
        |rn: &mut RawNode<MemStorage>, committed_entries: Vec<Entry>| {
            for entry in committed_entries {
                if entry.data.is_empty() {
                    continue;
                }

                if let EntryType::EntryConfChange = entry.get_entry_type() {
                    let mut cc = ConfChange::default();
                    cc.merge_from_bytes(&entry.data).unwrap();
                    let cs = rn.apply_conf_change(&cc).unwrap();
                    store.wl().set_conf_state(cs);
                } else {
                    // For normal proposals, extract the key-value pair and then 
                    // insert them into the kv engine

                    let data = std::str::from_utf8(&entry.data).unwrap();
                    let reg = Regex::new("put ([0-9]+) (.+)").unwrap();
                    if let Some(caps) = reg.captures(data) {
                        kv_pairs.insert(caps[1].parse().unwrap(), caps[2].to_string());
                    }
                }
                if rn.raft.state == StateRole::Leader {
                    // the leader should response to the client, tell them if their proposals 
                    // succeeded or not
                    let proposal = proposals.lock().unwrap().pop_front().unwrap();
                    proposal.propose_success.send(true).unwrap();
                }
            }  
        };
}

pub fn add_all_followers(proposals: &Mutex<VecDeque<Proposal>>) {
    for i in 2..=5u64 {
        let mut config_change = ConfChange::default();
        config_change.node_id = i;
        config_change.set_change_type(ConfChangeType::AddNode);
        loop {
            let (proposal, rx) = Proposal::conf_change(&config_change);
            proposals.lock().unwrap().push_back(proposal);
            if rx.recv().unwrap() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

    }
}