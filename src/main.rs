pub mod message;
pub mod transaction;
pub mod srvmanager_proc;
pub mod worker;
pub mod varworker;
pub mod defworker;

fn main() {
    todo!()
    // some tests for transaction of form { f := var | int }
    // test 1
    /*
        var a = 1;
        var b = 2;
        var c = 3;
        // def d = a + b + c;
        do action {
            a = b;
            b = c;
            c = 42;
        }
        // check result
     */
}
