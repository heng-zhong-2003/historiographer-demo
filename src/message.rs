#[derive(Debug, Clone)]
pub enum LockKind {
    WLock,
    RLock,
}

#[derive(Debug, Clone)]
pub enum Message {
    InitVar { name: String, val: i32 },
    AssignVar { name: String, val: i32 },
}
