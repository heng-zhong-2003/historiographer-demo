use crate::message::Message;
use crate::transaction::{Txn, Val};
use crate::worker::Worker;

use std::collections::{HashMap, HashSet};

pub struct DefWorker {
    pub worker: Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<Txn>,

    pub replica: HashMap<String, Option<i32>>,
    // for now expr is list of name or values calculating their sum
    pub expr: Vec<Val>,
    // pub ...
}

impl DefWorker {
    pub fn new() -> DefWorker {
        todo!()
    }

    pub async fn handle_message(msg: &Message) {
        match msg {
            Message::ReadDefRequest { txn } => {
                // { f := def }
                // ?

                // add to pending queue ?

                //
                //   while true {
                //        handle_message
                //        default process
                //   }

                // all txns in requires must already be applied by def worker

                // if all requires already in applied_txns, then send back to srvmananger
            }
            Message::PropaMessage {
                new_val,
                provides,
                requires,
            } => {}
            _ => panic!(),
        }

        pub async fn run_defworker(mut def_worker: DefWorker) {
            while let Some(msg) = def_worker.worker.inbox.recv().await {
                let _ = DefWorker::handle_message(&msg).await;

                // search for valid batch

                // apply valid batch
            }
        }
    }
}
