use crate::message;
use crate::transaction;
use crate::worker::Worker;

use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

pub struct VarWorker {
    pub worker: Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<transaction::Txn>,
    // pub provides: HashSet<transaction::Txn>,
    pub next_requires: HashSet<transaction::Txn>,
}

impl VarWorker {
    pub fn new(
        name: &str,
        inbox: mpsc::Receiver<message::Message>,
        sender_to_manager: mpsc::Sender<message::Message>,
    ) -> VarWorker {
        VarWorker {
            worker: Worker::new(name, inbox, sender_to_manager),
            value: None,
            applied_txns: Vec::new(),
            // provides: HashSet::new(),
            next_requires: HashSet::new(),
        }
    }

    pub async fn handle_message(
        worker: &Worker,
        curr_val: &mut Option<i32>,
        msg: &message::Message,
        applied_txns: &mut Vec<transaction::Txn>,
        // provides: &mut HashSet<transaction::Txn>,
        next_requires: &mut HashSet<transaction::Txn>,
    ) {
        match msg {
            // srvmanager will only send Write/Read requests when it checked
            // relevant locks are already acquired
            message::Message::ReadVarRequest { txn } => {
                // send ReadVarResult message back to who send the request, we
                // assume ReadVarRequest can only be sent by srvmanager

                // calculate the latest applied txn on var worker
                let latest_txn = HashSet::from([applied_txns[applied_txns.len() - 1].clone()]);
                let msg_back = message::Message::ReadVarResult {
                    txn: txn.clone(), // ?
                    name: worker.name.clone(),
                    result: curr_val.clone(),   // current value of state var
                    result_provide: latest_txn, // state var's latest applied txn
                };

                // send message back to srvmanager
                let _ = worker.sender_to_manager.send(msg_back).await;
                // lock then should be released by srvmanager

                next_requires.insert(txn.clone());
            }
            message::Message::WriteVarRequest {
                txn,
                write_val,
                requires,
            } => {
                // do write
                *curr_val = Some(write_val.clone());

                // add requires to next requires
                for r_txn in requires.iter() {
                    next_requires.insert(r_txn.clone());
                }

                // build propagation message
                let msg_propa = message::Message::PropaMessage {
                    propa_change: message::PropaChange {
                        name: worker.name.to_string(),
                        new_val: curr_val.clone().unwrap(),
                        provides: HashSet::from([txn.clone()]),
                        requires: requires.clone(),
                    },
                    
                };

                // update applied txns
                applied_txns.push(txn.clone());
                // update next requires
                next_requires.insert(txn.clone());

                // send to subscribers (def workers)
                for succ in worker.senders_to_succs.iter() {
                    let _ = succ.send(msg_propa.clone()).await;
                }
            }
            _ => panic!(),
        }
    }

    pub async fn run_varworker(mut var_worker: VarWorker) {
        while let Some(msg) = var_worker.worker.inbox.recv().await {
            let _ = VarWorker::handle_message(
                &var_worker.worker,
                &mut var_worker.value,
                &msg,
                &mut var_worker.applied_txns,
                &mut var_worker.next_requires,
            )
            .await;
        }
    }
}
