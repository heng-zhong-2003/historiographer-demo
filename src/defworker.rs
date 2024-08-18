use crate::message::{Message, PropaChange};
use crate::transaction::{Txn, Val};
use crate::worker::Worker;

use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

#[derive(PartialEq, Eq, Hash)]
pub struct TxnAndName {
    pub txn: Txn,
    pub name: String,
}
pub struct _PropaChange {
    pub propa_id: i32,
    pub propa_change: PropaChange,
    pub deps: HashSet<TxnAndName>,
}

pub struct DefWorker {
    pub worker: Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<Txn>,
    pub prev_batch_provides: HashSet<Txn>,
    // data structure maintaining all propa_changes to be apply
    pub propa_changes_to_apply: HashMap<Txn, _PropaChange>,

    // for now expr is list of name or values calculating their sum
    pub expr: Vec<Val>,
    // direct dependency and their current value
    pub replica: HashMap<String, Option<i32>>,
    // transtitive dependencies: handled by srvmanager for local dependencies
    // and SubscribeRequest/Grant for global dependencies
    pub transtitive_deps: HashMap<String, HashSet<String>>, 
    // var ->->-> input(def)
    // input(def) -> var

    pub counter: i32,
}

impl DefWorker {
    pub fn new(
        name: &str,
        inbox: mpsc::Receiver<Message>,
        sender_to_manager: mpsc::Sender<Message>,
        expr: Vec<Val>,
        replica: HashMap<String, Option<i32>>, // HashMap { dependent name -> None }
        transtitive_deps: HashMap<String, HashSet<String>>, 
    ) -> DefWorker {
        DefWorker {
            worker: Worker::new(name, inbox, sender_to_manager),
            value: None,
            applied_txns: Vec::new(),
            prev_batch_provides: HashSet::new(),
            propa_changes_to_apply: HashMap::new(),
            expr,
            replica,
            transtitive_deps,
            counter: 0,
        }
    }

    pub fn next_count(counter_ref: &mut i32,) -> i32 {
        *counter_ref += 1;
        *counter_ref
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
                propa_change,
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

    // valid batch definition w.r.t. def d
    // if (f := new value, P, R) in Batch, then 
    // for all (t, writes) in P of the propa_change message, we have 
    // (1)
    // - (idea) current worker def d will apply all effects of transaction t
    //   if a change message c requires a t, it means that before we can apply c, 
    //   we have to wait for change messages from all of our inputs that 
    //   (transitively) depend on the variables that t writes to.
    // - (mathematically) 
    //   for all i in direct_depenency(d), 
    //      if there exists a write to transitive_dependency(i) in writes,
    //      then we want to see a propa change (i := _, P', R') in Batch
    //         s.t. (t, writes) in P'
    // - (implement) dependency graph 
    //   - map[{t, name}] -> _PropaChange message (dependent on a bunch of {t, name})
    //   - for all (t, writes) in provides
    //          for var in writes 
    //              for name in (inputs(d) effected by var)
    //                  change.deps.insert({t, name})
    // (2)
    // - (idea/math) for all (t', write') <= (t, write), either t' has been 
    // applied or t' in change in this Batch
    // - (implement) 
    //   - for all (t', writes') in requires
    //      // then assuming (t', writes') in batch or applied implies:
    //          for var in writes 
    //              for name in (inputs(d) effected by var)
    //                  change.deps.insert({t, name})
    pub fn update_propa_changes_to_apply(
        counter_ref: &mut i32,
        propa_change: &PropaChange,
        transtitive_deps: &HashMap<String, HashSet<String>>
    ) -> _PropaChange
    {
        let mut deps: HashSet<TxnAndName> = HashSet::new();

        for txn in propa_change.provides.iter() {
            for write in txn.writes.iter() {
                let var_name = write.name.clone();

                let mut inputs: Vec<String> = Vec::new();
                for (i, dep_vars) in transtitive_deps.iter() {
                    match dep_vars.get(&var_name) {
                        Some(_) => { inputs.push(i.clone()); },
                        None => {},
                    }
                }

                for i_name in inputs.iter() {
                    let txn_name = TxnAndName {
                        txn: txn.clone(),
                        name: i_name.clone(),
                    };
                    deps.insert(txn_name);
                }
            }
        }

        // Not fully sure about below:
        for txn in propa_change.requires.iter() {
            for write in txn.writes.iter() {
                let var_name = write.name.clone();

                let mut inputs: Vec<String> = Vec::new();
                for (i, dep_vars) in transtitive_deps.iter() {
                    match dep_vars.get(&var_name) {
                        Some(_) => { inputs.push(i.clone()); },
                        None => {},
                    }
                }

                for i_name in inputs.iter() {
                    let txn_name = TxnAndName {
                        txn: txn.clone(),
                        name: i_name.clone(),
                    };
                    deps.insert(txn_name);
                }
            }
        }

        _PropaChange{
            propa_id: Self::next_count(counter_ref),
            propa_change: propa_change.clone(),
            deps,
        }
    }

    pub fn search_batch(
        propa_change: &PropaChange,
    ) -> HashSet<_PropaChange> {
        let mut batch: HashSet<_PropaChange> = HashSet::new();
        todo!()
    }

    pub async fn apply_batch(
        batch: HashSet<_PropaChange>,
        worker: &Worker,
        value: &mut Option<i32>,
        applied_txns: &mut Vec<Txn>,
        prev_batch_provides: &mut HashSet<Txn>,
        propa_changes_to_apply: &mut HashMap<Txn, _PropaChange>,
        replica: &mut HashMap<String, Option<i32>>,
    ) {
        let mut all_provides: HashSet<Txn> = HashSet::new();
        let mut all_requires: HashSet<Txn> = prev_batch_provides.clone();

        // latest change to prevent applying older updates after younger ones 
        // from the same dependency (only apply one dependency's latest update 
        // in the batch, and ignore others)
        let mut latest_change: HashMap<String, i32> = HashMap::new();
        
        for change in batch.iter() {
            // change := (value, P, R)
            let change_txns_toapply = &change.propa_change.provides;
            all_provides = all_provides.union(change_txns_toapply).cloned().collect();
            all_requires = all_requires.union(&change.propa_change.requires).cloned().collect();
            
            for txn in change_txns_toapply.iter() {
                propa_changes_to_apply.remove(txn);
            }

            if let Some(id) = latest_change.get(&change.propa_change.name) {
                if change.propa_id < *id { continue; }
            }

            replica.insert(change.propa_change.name.clone(), Some(change.propa_change.new_val));
            latest_change.insert(change.propa_change.name.clone(), change.propa_id);
        }

        // apply all txns in all_provides, the result should be calculated from 
        // replicas now 
        *value = compute_val(&replica);
        for txn in all_provides.iter() {
            applied_txns.push(txn.clone());
        }

        // update prev batch's applied txns, i.e. to be all_provides
        *prev_batch_provides = all_provides.clone();

        // broadcast the update to subscribers 
        let msg_propa = Message::PropaMessage { propa_change: 
            PropaChange { 
                name: worker.name.clone(), 
                new_val: value.clone().unwrap(), 
                provides: all_provides.clone(), 
                requires: all_requires.clone(), 
            }
        };
        for succ in worker.senders_to_succs.iter() {
            let _ = succ.send(msg_propa.clone()).await;
        }
    }
}

// TODO: now we only assume def f := f1 + f2 + ... + f_n
pub fn compute_val(replica: & HashMap<String, Option<i32>>) -> Option<i32> {
    let mut sum = 0;
    for (k, value) in replica.iter() {
        match value {
            Some(v) => sum += v,
            None => return None,
        }
    }
    return Some(sum);
}


// ??? Is there any way to implement counter as closure?
// pub fn id_gen() -> impl FnMut () -> i32 {
//     let mut cnt_ref = Box::new(0);
//     || {
//         *cnt_ref += 1;
//         *cnt_ref
//     }
// }

