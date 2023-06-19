mod node;
mod proposal;
mod signal;
mod utils;

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        mpsc::{self, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use raft::{prelude::Message, StateRole};
use slog::{info, Drain};

use crate::{
    node::Node,
    proposal::Proposal,
    signal::check_singals,
    utils::{add_all_followers, on_ready, propose},
};

fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, slog::o!());

    // 建立一个5个node的raft集群
    // 1. 建立交互通道,使用线程模拟raft node，使用mpsc::channel作为交互通道
    const NUM_NODES: usize = 5;

    let (mut tx_vec, mut rx_vec) = (Vec::new(), Vec::new());

    for _ in 0..NUM_NODES {
        let (tx, rx) = mpsc::channel();
        tx_vec.push(tx);
        rx_vec.push(rx);
    }
    // 2. 建立一个停止通道，用于停止集群
    let (tx_stop, rx_stop) = mpsc::channel();
    let rx_stop = Arc::new(Mutex::new(rx_stop));

    // 3. 使用VecDeque作为消息队列
    let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));

    let mut handles = Vec::new();

    for (i, rx) in rx_vec.into_iter().enumerate() {
        let mailboxes = (1..=5)
            .zip(tx_vec.iter().cloned())
            .collect::<HashMap<u64, Sender<Message>>>();
        let mut node = match i {
            0 => Node::create_raft_leader(1, rx, mailboxes, &logger),
            _ => Node::create_raft_follower(rx, mailboxes),
        };
        let proposals = Arc::clone(&proposals);

        let mut t = Instant::now();

        let rx_stop_clone = Arc::clone(&rx_stop);
        let logger = logger.clone();

        let handle = thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(10));
            loop {
                match node.my_mailbox.try_recv() {
                    Ok(msg) => node.step(msg, &logger),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }

            let raft_group = match node.raft_group {
                Some(ref mut r) => r,
                _ => continue,
            };

            if t.elapsed() >= Duration::from_millis(100) {
                raft_group.tick();
                t = Instant::now();
            }

            if raft_group.raft.state == StateRole::Leader {
                let mut proposals = proposals.lock().unwrap();
                for p in proposals.iter_mut().skip_while(|p| p.proposed > 0) {
                    propose(raft_group, p);
                }
            }

            // Handle readies from the raft
            on_ready(
                raft_group,
                &mut node.kv_pairs,
                &node.mailboxes,
                &proposals,
                &logger,
            );

            if check_singals(&rx_stop_clone) {
                return;
            };
        });
        handles.push(handle);
    }

    add_all_followers(proposals.as_ref());

    // put 100 key-value pairs
    info!(
        logger,
        "We get a 5 node raft cluster, now we propose 100 proposals"
    );

    (0..100u16)
        .filter(|i| {
            let (proposal, rx) = Proposal::normal(*i, format!("value{}", i));
            proposals.lock().unwrap().push_back(proposal);
            rx.recv().unwrap()
        })
        .count();

    info!(logger, "Propose 100 proposals success!");

    // 停止集群
    for _ in 0..NUM_NODES {
        tx_stop.send(signal::Signal::Terminate).unwrap();
    }

    for th in handles {
        th.join().unwrap();
    }
}
