use asyncstd;
use futures::executor::block_on;

#[test]
fn test_hello_async() {
    let r = asyncstd::hello_async();
    block_on(r);
}

#[test]
fn test_togther() {
    let r = asyncstd::togther();
    block_on(r);
}

#[test]
fn test_collect() {
    asyncstd::collect();
}
