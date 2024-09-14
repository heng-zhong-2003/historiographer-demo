use crate::message::{Message, PropaChange};
use crate::transaction::{Txn, Val};
use crate::worker::Worker;
use crate::{defworker, srvmanager_proc};

use inline_colorization::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TxnAndName {
    pub txn: Txn,
    pub name: String,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct _PropaChange {
    pub propa_id: i32,
    pub propa_change: PropaChange,
    pub deps: HashSet<TxnAndName>,
}

impl Hash for _PropaChange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.propa_id.hash(state);
    }
}

pub struct DefWorker {
    pub worker: Worker,
    pub value: Option<i32>,
    pub applied_txns: Vec<Txn>,
    pub prev_batch_provides: HashSet<Txn>,
    // data structure maintaining all propa_changes to be apply
    pub propa_changes_to_apply: HashMap<TxnAndName, _PropaChange>,

    // for now expr is list of name or values calculating their sum
    pub expr: Vec<Val>,
    // direct dependency and their current value
    pub all_inputs_ready: bool,
    pub replica: HashMap<String, Option<i32>>,
    // transtitive dependencies: handled by srvmanager for local dependencies
    // and SubscribeRequest/Grant for global dependencies
    pub transtitive_deps: HashMap<String, HashSet<String>>,
    // var ->->-> input(def)
    // input(def) -> var
    /*
       a   b    c 
        \  |   /
           d 
        a -> [] // or reflexively contain a ? 
        b -> [] // or reflexively contain b ? 
        c -> [] // or reflexively contain c ?
     */
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
            all_inputs_ready: false,
            replica,
            transtitive_deps,
            counter: 0,
        }
    }

    pub fn next_count(counter_ref: &mut i32) -> i32 {
        *counter_ref += 1;
        *counter_ref
    }

    pub async fn handle_message(
        worker: &Worker,
        curr_val: &mut Option<i32>,
        counter_ref: &mut i32,
        // all_inputs_ready: &mut bool,
        // replica: &HashMap<String, Option<i32>>,
        transtitive_deps: &HashMap<String, HashSet<String>>,
        propa_changes_to_apply: &mut HashMap<TxnAndName, _PropaChange>,
        applied_txns: &Vec<Txn>,
        msg: &Message,
    ) {
        match msg {
            Message::ReadDefRequest { txn, requires } => {
                // check whether Def work already get all required txns:
                // TODO: something on requires, (not sure why necessary)?
                // for all txn in requires, txn should be in applied if they are relavant to this def

                let result_provide = applied_txns.clone().clone().into_iter().collect();
                // TODO: set smaller than applied_txns should also work ...
                let msg_back = Message::ReadDefResult {
                    txn: txn.clone(), 
                    name: worker.name.clone(),
                    result: curr_val.clone(),   
                    result_provide: result_provide, 
                };

                // send message back to srvmanager
                let _ = worker.sender_to_manager.send(msg_back).await;

                // TODO: do we need merge this transaction into future required sets


            }
            Message::PropaMessage { propa_change } => {
                println!("{color_blue}PropaMessage{color_reset}");
                let _propa_change =
                    Self::processed_propachange(counter_ref, propa_change, transtitive_deps);

                for txn in &propa_change.provides {
                    println!("{color_blue}insert propa_changes_to_apply{color_reset}");
                    propa_changes_to_apply.insert(
                        TxnAndName {
                            txn: txn.clone(),
                            name: propa_change.name.clone(),
                        },
                        _propa_change.clone(),
                    );
                }
                println!("after receiving propamsg, the graph is {:#?}", propa_changes_to_apply);
            }

            // for test only
            // Message::ManagerRetrieve => {
            //     let msg = Message::ManagerRetrieveResult {
            //         name: worker.name.clone(),
            //         result: curr_val.clone(),
            //     };
            //     let _ = worker.sender_to_manager.send(msg).await;
            // }
            _ => panic!(),
        }
    }

    pub async fn run_defworker(mut def_worker: DefWorker) {
        while let Some(msg) = def_worker.worker.inbox.recv().await {
            println!("{color_red}defworker receive msg {:?}{color_reset}", msg);
            let _ = DefWorker::handle_message(
                &def_worker.worker,
                &mut def_worker.value,
                &mut def_worker.counter,
                &def_worker.transtitive_deps,
                &mut def_worker.propa_changes_to_apply,
                &def_worker.applied_txns,
                &msg,
            )
            .await;

            // search for valid batch
            let valid_batch = DefWorker::search_batch(
                &def_worker.propa_changes_to_apply,
                &def_worker.applied_txns,
            );

            // apply valid batch
            println!("{color_yellow}apply batch called{color_reset}");
            let (all_provides, all_requires, new_value) = DefWorker::apply_batch(
                valid_batch,
                // &def_worker.worker,
                &mut def_worker.value,
                &mut def_worker.applied_txns,
                &mut def_worker.prev_batch_provides,
                &mut def_worker.propa_changes_to_apply,
                &mut def_worker.replica,
            );

            // for test, ack srvmanager
            if new_value != None {
                let msg = Message::ManagerRetrieveResult {
                    name: def_worker.worker.name.clone(),
                    result: new_value.clone(),
                };
                let _ = def_worker.worker.sender_to_manager.send(msg).await;

                // broadcast the update to subscribers
                let msg_propa = Message::PropaMessage {
                    propa_change: PropaChange {
                        name: def_worker.worker.name.clone(),
                        new_val: new_value.unwrap(),
                        provides: all_provides.clone(),
                        requires: all_requires.clone(),
                    },
                };
                for succ in def_worker.worker.senders_to_succs.iter() {
                    let _ = succ.send(msg_propa.clone()).await;
                }
            }

            // println!(
            //     "{color_red}run def worker, def_worker.value after apply_batch: {:?}{color_reset}",
            //     def_worker.value
            // );
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
    //   for all i in inputs(d),
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
    pub fn processed_propachange(
        counter_ref: &mut i32,
        propa_change: &PropaChange,
        transtitive_deps: &HashMap<String, HashSet<String>>,
        // expect inputs(d) maps to transtitively depending vars
    ) -> _PropaChange {
        println!("def should have inputs: {:?}", transtitive_deps);
        let mut deps: HashSet<TxnAndName> = HashSet::new();

        for txn in propa_change.provides.iter() {
            for write in txn.writes.iter() {
                let var_name = write.name.clone();
                println!("def name: {:?}", var_name);

                let mut inputs: Vec<String> = Vec::new();
                for (i, dep_vars) in transtitive_deps.iter() {
                    match dep_vars.get(&var_name) {
                        Some(_) => {
                            println!("def add input: {:?}", i);
                            inputs.push(i.clone());
                        }
                        None => {}
                    }
                }
                println!("def has inputs: {:?}", inputs);

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
                        Some(_) => {
                            inputs.push(i.clone());
                        }
                        None => {}
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

        _PropaChange {
            propa_id: Self::next_count(counter_ref),
            propa_change: propa_change.clone(),
            deps,
        }
    }

    pub fn dfs(
        curr_node: &TxnAndName,
        visited: &mut HashSet<TxnAndName>,
        batch_acc: &mut HashSet<_PropaChange>,
        applied_txns: &HashSet<Txn>,
        graph: &HashMap<TxnAndName, _PropaChange>,
    ) -> bool {
        println!("dfs current node {:?}", curr_node);
        if visited.get(curr_node) != None
        // there already exists an change (sent by name) in B_acc, s.t. txn in change
            || applied_txns.get(&curr_node.txn) != None
        {
            // TODO!: make sure the understanding is correct
            // txn already exists in applied batch set
            // we can infer for all {txn, name} -> change, change in applied batch set
            println!("dfs find node {:?}", curr_node);
            return true;
        } else {
            match graph.get(curr_node) {
                None => return false,
                Some(_propa_change) => {
                    visited.insert(curr_node.clone());
                    batch_acc.insert(_propa_change.clone());
                    for succ in _propa_change.deps.iter() {
                        if !Self::dfs(succ, visited, batch_acc, applied_txns, graph) {
                            println!("dfs cannot find {:?}", succ);
                            return false;
                        }
                    }
                    return true;
                }
            }
        }
    }

    pub fn search_batch(
        propa_changes_to_apply: &HashMap<TxnAndName, _PropaChange>,
        applied_txns: &Vec<Txn>,
    ) -> HashSet<_PropaChange> {
        // DFS on propa_changes_to_apply,
        // println!("propa_changes_to_apply: {:?}", propa_changes_to_apply);
        let applied_txns_set: HashSet<Txn> = applied_txns.iter().cloned().collect();
        let mut visited: HashSet<TxnAndName> = HashSet::new();
        let mut batch_acc: HashSet<_PropaChange> = HashSet::new();

        for (node, _) in propa_changes_to_apply.iter() {
            if Self::dfs(
                node,
                &mut visited,
                &mut batch_acc,
                &applied_txns_set,
                &propa_changes_to_apply,
            ) {
                println!("find a batch: {:#?}", batch_acc);
                return batch_acc;
            } 
            else {
                visited = HashSet::new();
                batch_acc = HashSet::new();
            }
        }

        println!("cannot find a batch: {:#?}", batch_acc);
        batch_acc
        
    }

    pub fn apply_batch(
        batch: HashSet<_PropaChange>,
        // worker: &Worker,
        value: &mut Option<i32>,
        applied_txns: &mut Vec<Txn>,
        prev_batch_provides: &mut HashSet<Txn>,
        propa_changes_to_apply: &mut HashMap<TxnAndName, _PropaChange>,
        replica: &mut HashMap<String, Option<i32>>,
    ) -> (HashSet<Txn>, HashSet<Txn>, Option<i32>) {
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
            all_requires = all_requires
                .union(&change.propa_change.requires)
                .cloned()
                .collect();

            for txn in change_txns_toapply.iter() {
                propa_changes_to_apply.remove(&TxnAndName {
                    txn: txn.clone(),
                    name: change.propa_change.name.clone(),
                });
            }

            if let Some(id) = latest_change.get(&change.propa_change.name) {
                if change.propa_id < *id {
                    continue;
                }
            }

            replica.insert(
                change.propa_change.name.clone(),
                Some(change.propa_change.new_val),
            );
            latest_change.insert(change.propa_change.name.clone(), change.propa_id);
        }

        // apply all txns in all_provides, the result should be calculated from
        // replicas now
        println!("{color_green}batch: {:?}{color_reset}", batch);
        println!(
            "{color_yellow}replica before compute_val: {:?}{color_reset}",
            replica
        );
        *value = compute_val(&replica);
        println!(
            "{color_yellow}value after compute_val: {:?}{color_reset}",
            value
        );
        for txn in all_provides.iter() {
            applied_txns.push(txn.clone());
        }

        // update prev batch's applied txns, i.e. to be all_provides
        *prev_batch_provides = all_provides.clone();

        return (all_provides, all_requires, *value);

        // // broadcast the update to subscribers
        // let msg_propa = Message::PropaMessage { propa_change:
        //     PropaChange {
        //         name: worker.name.clone(),
        //         new_val: value.clone().unwrap(),
        //         provides: all_provides.clone(),
        //         requires: all_requires.clone(),
        //     }
        // };
        // for succ in worker.senders_to_succs.iter() {
        //     let _ = succ.send(msg_propa.clone()).await;
        // }
    }
}

// TODO: now we only assume def f := f1 + f2 + ... + f_n
pub fn compute_val(replica: &HashMap<String, Option<i32>>) -> Option<i32> {
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
//     let mut cnt = 0;
//     // let cnt_ref = &mut cnt;
//     move|| {
//         cnt += 1;
//         cnt
//     }
// }

// pub struct NatGen {
//     pub state: i32,
// }

// impl NatGen {
//     pub fn new() -> NatGen {
//         NatGen {state: 0}
//     }
//     pub fn next(&mut self) -> i32 {
//         self.state += 1;
//         self.state
//     }
// }

// some uses
// let mut gen = NatGen::new();
//     println!("{}", gen.next());
//     println!("{}", gen.next());
//     println!("{}", gen.next());
