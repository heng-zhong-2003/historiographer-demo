pub mod defworker;
pub mod message;
pub mod srvmanager_proc;
pub mod transaction;
pub mod varworker;
pub mod worker;

use inline_colorization::*;
use message::Message;
use srvmanager_proc::ServiceManager;
use std::collections::{HashMap, HashSet};
use tokio;
use transaction::{Txn, TxnId, Val, WriteToName};

#[tokio::main]
async fn main() {
    // test1().await;
    test2().await;

    // srvmanager retreive d's value
}

// Expected: a = 42, b = 3
pub async fn test1() {
    let mut manager = ServiceManager::new();
    let _ = ServiceManager::create_var_worker(
        "a",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;
    let _ = ServiceManager::create_var_worker(
        "b",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;

    let var_a_inbox = manager.worker_inboxes.get("a").unwrap().clone();
    let var_b_inbox = manager.worker_inboxes.get("b").unwrap().clone();

    let write_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Val::Int(3),
        }],
    };

    ServiceManager::handle_transaction(
        &write_a_txn,
        &mut manager.worker_inboxes,
        &mut manager.receiver_from_workers,
    )
    .await;

    let write_b_txn = Txn {
        id: TxnId::new(),
        writes: vec![
            WriteToName {
                name: "a".to_string(),
                expr: Val::Int(42),
            },
            WriteToName {
                name: "b".to_string(),
                expr: Val::Var("a".to_string()),
            },
        ],
    };

    ServiceManager::handle_transaction(
        &write_b_txn,
        &mut manager.worker_inboxes,
        &mut manager.receiver_from_workers,
    )
    .await;

    println!("start printing results");
    // retrieve msg sent to var worker ...
    let _ = var_a_inbox.send(Message::ManagerRetrieve).await;
    let _ = var_b_inbox.send(Message::ManagerRetrieve).await;
    while let Some(rcv_val) = manager.receiver_from_workers.recv().await {
        println!("receive value: {:?}", rcv_val);
    }
}

// var a = 1, b = 2, c = 3
// def d = a + b + c
// do action {
//     a := b
//     b := c
//     c := 42
// }
// Expected:
pub async fn test2() {
    let mut manager = ServiceManager::new();
    let _ = ServiceManager::create_var_worker(
        "a",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;
    let _ = ServiceManager::create_var_worker(
        "b",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;
    let _ = ServiceManager::create_var_worker(
        "c",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;
    let mut transtitive_deps: HashMap<String, HashSet<String>> = HashMap::new();
    transtitive_deps.insert("a".to_string(), vec!["d".to_string()].into_iter().collect());
    transtitive_deps.insert("b".to_string(), vec!["d".to_string()].into_iter().collect());
    transtitive_deps.insert("c".to_string(), vec!["d".to_string()].into_iter().collect());
    let _ = ServiceManager::create_def_worker(
        "d",
        manager.sender_to_manager.clone(),
        vec![
            Val::Var("a".to_string()),
            Val::Var("b".to_string()),
            Val::Var("c".to_string()),
        ],
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
            .into_iter()
            .map(|n| (n, None))
            .collect(),
        transtitive_deps,
        &mut manager.worker_inboxes,
    )
    .await;

    let var_a_inbox = manager.worker_inboxes.get("a").unwrap().clone();
    let var_b_inbox = manager.worker_inboxes.get("b").unwrap().clone();
    let var_c_inbox = manager.worker_inboxes.get("c").unwrap().clone();
    let def_d_inbox = manager.worker_inboxes.get("d").unwrap().clone();

    let init_abc_txn = Txn {
        id: TxnId::new(),
        writes: vec![
            WriteToName {
                name: "a".to_string(),
                expr: Val::Int(1),
            },
            WriteToName {
                name: "b".to_string(),
                expr: Val::Int(2),
            },
            WriteToName {
                name: "c".to_string(),
                expr: Val::Int(3),
            },
        ],
    };

    ServiceManager::handle_transaction(
        &init_abc_txn,
        &mut manager.worker_inboxes,
        &mut manager.receiver_from_workers,
    )
    .await;

    // std::thread::sleep(std::time::Duration::from_secs(5));
    let _ = def_d_inbox.send(Message::ManagerRetrieve).await;
    while let Some(rcv_val) = manager.receiver_from_workers.recv().await {
        println!("receive value: {:?}", rcv_val);
    }
}
