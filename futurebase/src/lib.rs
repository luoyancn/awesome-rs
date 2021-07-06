#[macro_use]
extern crate slog_logger;
extern crate tokio;

use tokio::runtime;

pub fn async_thread() {
    let rte = runtime::Builder::new_multi_thread().build().unwrap();
    for number in 1..100 {
        let future = async move {
            info!("number : {}", number);
        };
        rte.spawn(future);
    }

    info!("At the last of function named async_thread");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
