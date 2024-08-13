use crate::worker::{Worker, self};
use crate::varworker::{VarWorker};
use crate::message::{self, Lock, LockType, Message};
use crate::transaction::{Txn, Val};

use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

const BUFFER_SIZE: usize = 1024;

async fn run_varworker(mut var_worker: VarWorker) {
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

pub struct ServiceManager {
    // channels
    // a map of worker name to their inboxes
    pub worker_inboxes: HashMap<String, mpsc::Sender<Message>>,
    // inbox of service manager
    pub sender_to_manager: mpsc::Sender<Message>,
    // outbox of all workers
    pub receiver_from_workers: mpsc::Receiver<Message>,
    // locks, each name should have either one WLock or multiple RLocks
    pub locks: HashMap<String, Option<Lock>>,
    // typing env
        // pub typenv: HashMap<String, Option<typecheck::Type>>,
        // pub var_or_def_env: HashMap<String, VarOrDef>,
    // dependency graph
        // pub dependgraph: HashMap<String, HashSet<String>>,
} 

impl ServiceManager {
    pub fn new() -> Self {
        let (sndr, rcvr) = mpsc::channel(BUFFER_SIZE);
        ServiceManager {
            worker_inboxes: HashMap::new(),
            sender_to_manager: sndr,
            receiver_from_workers: rcvr,
            locks: HashMap::new(),
            // typenv: HashMap::new(),
            // var_or_def_env: HashMap::new(),
            // dependgraph: HashMap::new(),
        }
    }  

    pub async fn create_worker(
        name: &str,
        sender_to_manager: mpsc::Sender<Message>,
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
        
    ) {
        // allocate channel for specific for var worker
        let (sndr, rcvr) = mpsc::channel(BUFFER_SIZE);
        let mut var_worker = VarWorker::new(
            name, 
            rcvr, 
            sender_to_manager.clone());
        
        tokio::spawn(run_varworker(var_worker));    
    }

    pub async fn handle_transaction(
        txn: &Txn, 
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
        receiver_from_workers: &mut mpsc::Receiver<Message>,
    ) {
        // analyze a transaction
            // first assume f := var | f := int
        let mut cnt = 0;
        let mut names_to_value: HashMap<String, Option<i32>> = HashMap::new();

        // require all reads 
        for assign in txn.writes.iter() {
            match &assign.expr {
                Val::Var(name) => {
                    // requires RLock from remote srvmanagers 
                    // if RLock acquistion rejected (Die), then transaction abandoned 

                    cnt += 1;
                    names_to_value.insert(name.clone(), None);

                    let msg_request_read = Message::ReadVarRequest { txn: txn.clone() };
                    let worker_inbox_addr: &mpsc::Sender<Message> = worker_inboxes.get(name).unwrap();

                    let _ = worker_inbox_addr.send(msg_request_read).await.unwrap();

                },
                Val::Int(_) => continue,
                Val::Def(_) => continue,
            }
        }

        // calculate the require set (R) of the transactions 
        let mut requires_for_txn: HashSet<Txn> = HashSet::new();
        
        while let Some(msg_back) = receiver_from_workers.recv().await {
            match msg_back {
                Message::ReadVarResult { 
                    txn, 
                    name, 
                    result, 
                    result_provide } => {
                        cnt -= 1;
                        names_to_value.insert(name.clone(), result);
        
                        requires_for_txn = requires_for_txn.union(&result_provide).cloned().collect(); // ?

                        // when we gained all reads
                        if cnt == 0 {
                            // send out all the write requests to var workers 
                            for write in txn.writes.iter() {
                                let new_val = match &write.expr {
                                    Val::Int(x) => *x,
                                    Val::Var(n) => names_to_value.get(n).unwrap().unwrap(),
                                    Val::Def(_) => todo!(),   
                                };
                                let msg_write_request = Message::WriteVarRequest { 
                                    txn: txn.clone(), 
                                    write_val: new_val, 
                                    requires: requires_for_txn.clone(),  
                                };
                                let var_addr = worker_inboxes.get(&write.name).unwrap();
                                let _ = var_addr.send(msg_write_request).await;
                            }
                        }
                    },
                _ => panic!("unexpected message heard from workers"),
            }
        }
    }
}

// TODO: maintain locks

