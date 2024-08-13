use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use crate::message;
use tokio::sync::mpsc;

pub struct Worker {
    pub name: String,
    // worker input message box
    pub inbox: mpsc::Receiver<message::Message>,
    // worker output to manager
    pub sender_to_manager: mpsc::Sender<message::Message>,
    // List of worker outputs to subscribers
    pub senders_to_succs: Vec<mpsc::Sender<message::Message>>,
}

impl Worker {
    // allocate a new worker
    // notice if new worker dependents on previous workers,
    // then we need to update previous workers' senders_to_succs set.
    pub fn new(
        name: &str,
        inbox: mpsc::Receiver<message::Message>,
        sender_to_manager: mpsc::Sender<message::Message>,
    ) -> Worker {
        Worker {
            name: name.to_string(), 
            inbox, 
            sender_to_manager,
            senders_to_succs: Vec::new(),
        }
    }
}
