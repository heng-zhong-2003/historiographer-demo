use crate::transaction::Txn;
use tokio::sync::mpsc::{self, Sender};

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum LockType {
    WLock,
    RLock,
}

pub struct Lock {
    pub txn: Txn, // can use TxnId instead of whole transaction
    pub lock_t: LockType,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PropaChange {
    pub name: String,
    pub new_val: i32,
    pub provides: HashSet<Txn>,
    pub requires: HashSet<Txn>,
}

#[derive(Clone, Debug)]
// Message types received by state var nodes
pub enum Message {
    // Lock acquire is only needed inter-ServiceManagers
    // LockAcquire {

    // },
    // LockRelease {

    // },
    // LockFail {

    // },
    // LockGrant {

    // },
    // srvmanager -> var worker
    ReadVarRequest {
        txn: Txn,
    },
    // srvmanager -> def worker
    ReadDefRequest {
        txn: Txn,
        requires: HashSet<Txn>, // ?? Why we need this
    },
    // var worker -> srvmanager
    ReadVarResult {
        txn: Txn,
        name: String,
        result: Option<i32>,
        result_provide: HashSet<Txn>,
    },
    ReadDefResult {
        txn: Txn,
        name: String,
        result: Option<i32>,
        result_provide: HashSet<Txn>,
    },
    // srvmanager -> var worker
    WriteVarRequest {
        txn: Txn,
        write_val: i32,
        requires: HashSet<Txn>,
    },
    // var worker -> def worker (succs)
    // propagate message type (new_value, P set, R set)
    PropaMessage {
        propa_change: PropaChange,
    },

    ManagerRetrieve,
    ManagerRetrieveResult {
        name: String,
        result: Option<i32>,
    },
    SubscribeRequest {
        subscriber_name: String,
        sender: Sender<Message>,
    },
    SubscribeGrant {},
}
