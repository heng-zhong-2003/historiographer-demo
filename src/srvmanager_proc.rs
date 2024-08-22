use crate::defworker::DefWorker;
use crate::message::{self, Lock, LockType, Message};
use crate::transaction::{Txn, Val};
use crate::varworker::VarWorker;
use crate::worker::{self, Worker};

use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::{self, Receiver, Sender};

const BUFFER_SIZE: usize = 1024;

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

    pub async fn create_var_worker(
        name: &str,
        sender_to_manager: mpsc::Sender<Message>,
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
    ) {
        // allocate channel for specific for var worker
        let (sndr, rcvr) = mpsc::channel(BUFFER_SIZE);
        let var_worker = VarWorker::new(name, rcvr, sender_to_manager.clone());

        worker_inboxes.insert(name.to_string(), sndr);

        tokio::spawn(VarWorker::run_varworker(var_worker));
    }

    // Aug. 19, added by Heng.
    pub async fn create_def_worker(
        name: &str,
        sender_to_manager: mpsc::Sender<Message>,
        expr: Vec<Val>,
        replica: HashMap<String, Option<i32>>, // HashMap { dependent name -> None }
        transtitive_deps: HashMap<String, HashSet<String>>,
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
    ) {
        let (sndr, rcvr): (Sender<Message>, Receiver<Message>) = mpsc::channel(BUFFER_SIZE);
        let def_worker = DefWorker::new(
            name,
            rcvr,
            sender_to_manager.clone(),
            expr,
            replica.clone(),
            transtitive_deps,
        );
        // println!("create def insert worker inboxes {}", name);
        worker_inboxes.insert(name.to_string(), sndr.clone());
        for (dep_name, _) in replica.iter() {
            let sender_to_dep = worker_inboxes.get(dep_name).unwrap().clone();
            let _ = sender_to_dep
                .send(Message::SubscribeRequest {
                    subscriber_name: name.to_string(),
                    sender: sndr.clone(),
                })
                .await;
        }
        
        // TODO: should know all deps add successfully then spawn def_worker
        // require all SubscribeGrant 
        
        tokio::spawn(DefWorker::run_defworker(def_worker));
    }

    pub async fn handle_transaction(
        txn: &Txn,
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
        receiver_from_workers: &mut mpsc::Receiver<Message>,
    ) {
        // analyze a transaction
        // first assume f := var | f := def | f := int
        let mut cnt = 0;
        let mut names_to_value: HashMap<String, Option<i32>> = HashMap::new();

        // require all reads
        // println!("enter iterating writes");
        for assign in txn.writes.iter() {
            match &assign.expr {
                Val::Var(name) => {
                    // requires RLock from remote srvmanagers
                    // if RLock acquistion rejected (Die), then transaction abandoned
                    cnt += 1;
                    names_to_value.insert(name.clone(), None);

                    let msg_request_read = Message::ReadVarRequest { txn: txn.clone() };
                    let worker_inbox_addr: &mpsc::Sender<Message> =
                        worker_inboxes.get(name).unwrap();

                    let _ = worker_inbox_addr.send(msg_request_read).await.unwrap();
                }
                Val::Int(_) => {
                    // let msg_write_request = Message::WriteVarRequest {
                    //     txn: txn.clone(),
                    //     write_val: *new_val,
                    //     requires: HashSet::new(),
                    // };
                    // let var_addr = worker_inboxes.get(&assign.name).unwrap();
                    // let _ = var_addr.send(msg_write_request).await;
                    continue;
                }
                Val::Def(_) => {
                    todo!()
                }
            }
        }
        // calculate the require set (R) of the transactions
        let mut requires_for_txn: HashSet<Txn> = HashSet::new();

        if cnt > 0 {
            // println!("need to read from other names");
            while let Some(msg_back) = receiver_from_workers.recv().await {
                match msg_back {
                    Message::ReadVarResult {
                        txn: txn_back,
                        name,
                        result,
                        result_provide,
                    } => {
                        // println!("receive txn {:?}", txn);
                        // make sure response from current txn's request
                        if txn_back == *txn {
                            cnt -= 1;
                            names_to_value.insert(name.clone(), result);
                            requires_for_txn =
                                requires_for_txn.union(&result_provide).cloned().collect();
                            // ?
                        }
                        // println!("current count is {}", cnt);
                        if cnt == 0 {
                            break;
                        }
                    }
                    _ => panic!("unexpected message heard from workers"),
                }
            }
            // still need to deal with deadlock when cnt > 0
        }

        // when we gained all reads
        if cnt == 0 {
            // println!("no need to read from other names or gained all reads");
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
            println!("transaction has been triggered successfully");
            return;
        }
    }
}

// TODO: maintain locks
