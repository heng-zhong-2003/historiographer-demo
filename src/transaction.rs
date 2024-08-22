use std::{fmt::Debug, hash::{Hash, Hasher}};
use tokio::time::Instant;
use std::fmt;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct TxnId {
    pub time: Instant,
    // address or uuid to break tie
}

impl TxnId {
    pub fn new() -> TxnId {
        TxnId {
            time: Instant::now(),
        }
    }
}

// version 1 only test f := int | def | var
#[derive(Clone, Debug)]
pub enum Val {
    Int(i32),
    Def(String),
    Var(String),
}

// a single update to state var
#[derive(Clone, Debug)]
pub struct WriteToName {
    pub name: String,
    pub expr: Val,
}

// (txid, writes)
// writes := a list of updates to state vars
#[derive(Clone)]
pub struct Txn {
    pub id: TxnId,
    pub writes: Vec<WriteToName>,
}

impl PartialEq for Txn {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Txn {}

impl Hash for Txn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// impl fmt::Debug for TxnId {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("TxnId")
//         .field("id", &self.time.fmt)
//         .finish()
//     }
// }

impl fmt::Debug for Txn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Txn")
         .finish()
    }
}
