use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::future::join_all;

#[macro_use]
extern crate slog_logger;
use async_std::fs::File;
use async_std::{
    self,
    io::{self, prelude::WriteExt},
};

pub async fn async_hello() {
    info!("This is the async hello");
}

pub async fn connect_db_fake() -> String {
    async_std::task::sleep(Duration::from_secs(1)).await;
    info!("This is the async connect_db_fake function");
    "connect_db_fake".to_owned()
}

pub async fn open_file_fake() -> String {
    async_std::task::sleep(Duration::from_secs(2)).await;
    info!("This is the async open_file_fake function");
    "open_file".to_owned()
}

pub async fn get_cities() -> Vec<String> {
    let cities = vec![
        "shanghai".to_owned(),
        "beijing".to_owned(),
        "chongqing".to_owned(),
    ];
    let city_vec = Arc::new(Mutex::new(vec![]));
    let _ = join_all(
        cities
            .into_iter()
            .map(|city| build_city(city_vec.clone(), city)),
    )
    .await;
    return city_vec.lock().unwrap().clone();
}

async fn build_city(city_vec: Arc<Mutex<Vec<String>>>, city: String) {
    async_std::task::sleep(Duration::from_secs(1)).await;
    city_vec
        .lock()
        .unwrap()
        .push(format!("Super City {}", city))
}

pub async fn async_file_opts() -> io::Result<()> {
    let mut file = File::create("a.txt").await?;
    Ok(file.write_all(b"hello world").await?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    use super::*;
    use async_std::task::block_on;
    use futures;
    use std::time;

    #[test]
    fn test_async_hello() {
        block_on(async_hello());
    }

    #[test]
    fn test_connect_db_fake() {
        let res = block_on(connect_db_fake());
        assert_eq!("connect_db_fake", res);
    }

    #[test]
    fn test_open_file_fake() {
        let res = block_on(open_file_fake());
        assert_eq!("open_file", res);
    }

    #[test]
    fn test_join_fake() {
        let now = time::Instant::now();
        let (fake_db, fake_file) =
            block_on(futures::future::join(connect_db_fake(), open_file_fake()));
        let elapsed = now.elapsed();
        assert_eq!("open_file", fake_file);
        assert_eq!("connect_db_fake", fake_db);
        println!("All of async function used {:#?} ", elapsed);
        assert!(elapsed > Duration::from_secs(1));
        assert!(elapsed < Duration::from_secs(3));
    }

    #[test]
    fn test_join_all_fake() {
        let now = time::Instant::now();
        let res = block_on(get_cities());
        assert_eq!(
            vec![
                "Super City shanghai".to_owned(),
                "Super City beijing".to_owned(),
                "Super City chongqing".to_owned()
            ],
            res
        );
        let used = now.elapsed();
        assert!(used < Duration::from_secs(2));
    }

    #[async_std::test]
    async fn test_async_file_io() -> io::Result<()> {
        async_file_opts().await
    }

    use async_std::channel::{unbounded, RecvError, TryRecvError};
    #[async_std::test]
    async fn test_async_channel() {
        let (sender, recver) = unbounded();
        assert_eq!(sender.send(1).await, Ok(()));

        assert_eq!(recver.recv().await, Ok(1));
        assert_eq!(recver.try_recv(), Err(TryRecvError::Empty));
        //assert_eq!(recver.recv().await, Err(RecvError));
        drop(sender);
        assert_eq!(recver.try_recv(), Err(TryRecvError::Closed));
        assert_eq!(recver.recv().await, Err(RecvError));
    }

    use async_std::stream::{self, StreamExt};
    #[async_std::test]
    async fn test_async_stream_base() {
        let mut set_stream = stream::from_iter(1..=10);
        assert_eq!(set_stream.next().await, Some(1));
        let all_item = set_stream.collect::<Vec<u8>>().await;
        assert_eq!(vec![2, 3, 4, 5, 6, 7, 8, 9, 10], all_item);

        let mut intvl = stream::interval(Duration::from_secs(1));
        let mut count = 0;
        //while let Some(_) = intvl.next().await {
        while let Some(_) = block_on(intvl.next()) {
            count += 1;
            if count > 5 {
                break;
            }
            println!("{} seconds elapsed ", count);
        }
        assert_eq!(6, count);

        let mut item_normal = (1..=10).into_iter();
        assert_eq!(Some(1), item_normal.next());
    }
}
