use tokio::sync::mpsc;
use crate::transaction;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum LockKind {
    WLock,
    RLock,
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
        txn: transaction::Txn,
    },
    // var worker -> srvmanager
    ReadVarResult {
        txn: transaction::Txn,
        result: Option<i32>,
        result_provide: HashSet<transaction::Txn>,
    },
    // srvmanager -> var worker
    WriteVarRequest {
        txn: transaction::Txn,
        write_val: i32,
        requires: HashSet<transaction::Txn>,
    },
    // var worker -> def worker (succs)
    // propagate message type (new_value, P set, R set)
    PropaMessage {
        new_val: i32, 
        provides: HashSet<transaction::Txn>,
        requires: HashSet<transaction::Txn>,
    }
}