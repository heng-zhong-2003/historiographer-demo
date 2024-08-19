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
    ) .await;
    let _ = ServiceManager::create_worker(
        "b",
        manager.sender_to_manager.clone(),
        &mut manager.worker_inboxes,
    ) .await;

    let var_a_inbox = manager.worker_inboxes.get("a").unwrap().clone();
    let var_b_inbox = manager.worker_inboxes.get("b").unwrap().clone();
 
    /* let write_a_txn = Txn {
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
    } */

    let write_a_txn = Txn {
        id: TxnId::new(),
        writes: vec![
            WriteToName {
                name: "a".to_string(),
                expr: Val::Int(3),
            },
        ],
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

    // let read_a_txn = Txn {
    //     id: TxnId::new(),
    //     writes: vec![],
    // };

    // println!("start handle read transaction");
    // ServiceManager::handle_transaction(
    //     &read_a_txn,
    //     &mut manager.worker_inboxes,
    //     &mut manager.receiver_from_workers,
    // )
    // .await;

    println!("start printing results");
    // retrieve msg sent to var worker ...
    let _ = var_a_inbox.send(Message::ManagerRetreive).await;
    let _ = var_b_inbox.send(Message::ManagerRetreive).await;
    while let Some(rcv_val) = manager.receiver_from_workers.recv().await {
        println!("receive value: {:?}", rcv_val);
    }

    // test 1
    /*
       var a = 1;
       var b = 2;
       var c = 3;
       do action {
           a = b;
           b = c;
           c = 42;
       }
       check result
    */
    // todo!()

    // Now do the test:
    // def d = a + b + c
    // do action {
    //     var a 
    //     var b 
    //     var c
    // }

    // srvmanager retreive d's value
}
