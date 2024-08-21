use crate::message::{Message, PropaChange};
use crate::transaction;
use crate::worker::Worker;

use inline_colorization::*;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

pub struct VarWorker {
    pub worker: Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<transaction::Txn>,
    // pub provides: HashSet<transaction::Txn>,
    pub next_requires: HashSet<transaction::Txn>,
}
// QUESTION: why not mut all fields of VarWorker??
// instead of we taking the ownership and re-declare a
// mutable var_worker later when run the worker?

impl VarWorker {
    pub fn new(
        name: &str,
        inbox: mpsc::Receiver<Message>,
        sender_to_manager: mpsc::Sender<Message>,
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
        worker: &mut Worker,
        curr_val: &mut Option<i32>,
        msg: &Message,
        applied_txns: &mut Vec<transaction::Txn>,
        // provides: &mut HashSet<transaction::Txn>,
        next_requires: &mut HashSet<transaction::Txn>,
    ) {
        match msg {
            // srvmanager will only send Write/Read requests when it checked
            // relevant locks are already acquired
            Message::ReadVarRequest { txn } => {
                // send ReadVarResult message back to who send the request, we
                // assume ReadVarRequest can only be sent by srvmanager

                // calculate the latest applied txn on var worker
                let latest_txn = HashSet::from([applied_txns[applied_txns.len() - 1].clone()]);
                let msg_back = Message::ReadVarResult {
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
            Message::WriteVarRequest {
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
                let msg_propa = Message::PropaMessage {
                    propa_change: PropaChange {
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
                    // println!("{color_green}send propa {:?}{color_reset}", msg_propa);
                    let _ = succ.send(msg_propa.clone()).await;
                }
            }

            // retreive value message to inform svcmananger
            Message::ManagerRetrieve => {
                // println!("current var worker value is {:?}", curr_val);
                let msg = Message::ManagerRetrieveResult {
                    name: worker.name.clone(),
                    result: curr_val.clone(),
                };
                let _ = worker.sender_to_manager.send(msg).await;
            }
            Message::SubscribeRequest {
                subscriber_name,
                sender,
            } => {
                // println!("{color_green}varworker receive {:?}{color_reset}", msg);
                worker.senders_to_succs.push(sender.clone());

                // let msg_propa = Message::PropaMessage {
                //     propa_change: PropaChange {
                //         name: worker.name.to_string(),
                //         new_val: curr_val.clone().unwrap(),
                //         provides: HashSet::from([txn.clone()]),
                //         requires: requires.clone(),
                //     },
                // };
                // let _ = sender.send(msg_propa).await;
            }
            _ => panic!(),
        }
    }

    pub async fn run_varworker(mut var_worker: VarWorker) {
        while let Some(msg) = var_worker.worker.inbox.recv().await {
            let _ = VarWorker::handle_message(
                &mut var_worker.worker,
                &mut var_worker.value,
                &msg,
                &mut var_worker.applied_txns,
                &mut var_worker.next_requires,
            )
            .await;
        }
    }
}
