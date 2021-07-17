extern crate futures;

use futures::future::{self, Future};

#[macro_use]
extern crate slog_logger;

pub async fn hello_async() {
    info!("Hello world from async std");
}

async fn learn_sing() {
    info!("learning sing ...");
}

async fn sing_song() {
    info!("Sing song ...");
}

async fn dance() {
    info!("Dancing ...");
}

async fn learn_and_song() {
    learn_sing().await;
    sing_song().await;
}

pub async fn togther() {
    let t1 = learn_and_song();
    let t2 = dance();
    futures::join!(t1, t2);
}

fn async_read_file(name: &str) -> impl Future<Output = String> {
    future::ready(String::from(name))
}

async fn read_file(name: &str) -> String {
    String::from(name)
}

fn async_with_lifetime<'a>(input: &'a u8) -> impl Future<Output = u8> + 'a {
    future::ready(*input)
}

async fn lifetime<'a>(input: &'a u8) -> u8 {
    *input
}

pub fn collect() {
    let content = async_read_file("zhangjl");
    let cont = futures::executor::block_on(content);
    info!("The content is {}", cont);
    futures::executor::block_on(togther());
    let input: u8 = 32;
    let lifetime_future = async_with_lifetime(&input);
    let lifetime_res = futures::executor::block_on(lifetime_future);
    info!("The lifetime result is {}", lifetime_res);

    let f_future = read_file("luoyan");
    let l_future = lifetime(&input);
    let f = futures::executor::block_on(f_future);
    let l = futures::executor::block_on(l_future);
    info!("The f is {} and l is {}", f, l);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
