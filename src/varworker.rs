use crate::worker;
use crate::transaction;
use crate::message;

use std::collections::{Vec, HashSet, HashMap};

pub struct VarWorker {
    pub worker: worker::Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<transaction::Txn>,
    pub provides: HashSet<transaction::Txn>,
    pub requires: HashSet<transaction::Txn>,
}

impl VarWorker {
    pub fn new(
        name: &str,
        inbox: mpsc::Receiver<message::Message>,
        sender_to_manager: mpsc::Sender<message::Message>,
    ) -> VarWorker {
        VarWorker {
            worker::new(name, inbox, sender_to_manager),
            value: None,
            applied_txns: Vec::new(),
            provides: HashSet::new(),
            requires: HashSet::new(),
        }
    }

    pub async fn handle_message(
        worker: worker::Worker,
        curr_val: Option<i32>,
        applied_txns: Vec<transaction::Txn>,
        ... 
    ) {
        match msg {
            // srvmanager will only send Write/Read requests when it checked
            // relevant locks are already acquired
            message::Message::ReadVarRequest{ txn } => {
                // send ReadVarResult message back to who send the request, we 
                // assume ReadVarRequest can only be sent by srvmanager

                // calculate the latest applied txn on var worker
                let mut latest_txn = HashSet::new();
                latest_txn.insert(applied_txns[applied_txns.length() - 1]);
                let backmsg = message::Message::ReadVarResult {
                    txn, 
                    curr_val, 
                    latest_txn,
                },
                let _ = worker.sender_to_manager.send(backmsg).await.expect("...")
            }
            message::Message::WriteVarRequest{ txn,  write_val } => {
                ... 
            }
            _ => panic!()

        }
    }
}
