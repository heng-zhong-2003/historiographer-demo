pub mod defworker;
pub mod message;
pub mod srvmanager_proc;
pub mod transaction;
pub mod varworker;
pub mod worker;

use message::Message;
use srvmanager_proc::ServiceManager;
use std::collections::HashSet;
use tokio;
use transaction::{Txn, TxnId, Val, WriteToName};
use varworker::VarWorker;

#[tokio::main]
async fn main() {
    // some tests for transaction of form { f := var | int }
    let mut manager = ServiceManager::new();
    let _ = ServiceManager::create_worker(
        "a",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    )
    .await;
    // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let var_a_inbox = manager.worker_inboxes.get("a").unwrap();
    let write_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Val::Int(3),
        }],
    };
    let write_a_msg = Message::WriteVarRequest {
        txn: write_a_txn,
        write_val: 3,
        requires: HashSet::new(),
    };
    let _ = var_a_inbox.send(write_a_msg).await.unwrap();

    let read_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Val::Int(5),
        }],
    };
    let read_a_msg = Message::ReadVarRequest { txn: read_a_txn };
    let _ = var_a_inbox.send(read_a_msg).await.unwrap();

    let recv_msg = manager.receiver_from_workers.recv().await.unwrap();
    match recv_msg {
        Message::ReadVarResult {
            txn,
            name,
            result,
            result_provide,
        } => {
            println!("name {}, result {:?}", name, result);
        }
        _ => panic!(),
    }

    // Assign
    let write_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Val::Int(114514),
        }],
    };
    let write_a_msg = Message::WriteVarRequest {
        txn: write_a_txn,
        write_val: 114514,
        requires: HashSet::new(),
    };
    let _ = var_a_inbox.send(write_a_msg).await.unwrap();

    let read_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Val::Int(5),
        }],
    };
    let read_a_msg = Message::ReadVarRequest { txn: read_a_txn };
    let _ = var_a_inbox.send(read_a_msg).await.unwrap();

    let recv_msg = manager.receiver_from_workers.recv().await.unwrap();
    match recv_msg {
        Message::ReadVarResult {
            txn,
            name,
            result,
            result_provide,
        } => {
            println!("after assign, name {}, result {:?}", name, result);
        }
        _ => panic!(),
    }

    // test 1
    /*
       var a = 1;
       var b = 2;
       var c = 3;
       def d = a + b + c;
       do action {
           a = b;
           b = c;
           c = 42;
       }
       check result
    */
    // todo!()
}
