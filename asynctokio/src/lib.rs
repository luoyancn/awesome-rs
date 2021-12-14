use std::collections::VecDeque;
use std::env::temp_dir;
use std::ffi::OsString;
use std::str;
use std::sync::Mutex;
use std::time;

use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UnixListener};
use tokio::time::sleep;

#[macro_use]
extern crate slog_logger;

macro_rules! __serial_command__ {
    ($location: expr) => {{
        println!("Sucess poweroff the {} board", $location);
    }};
}

pub async fn hello_tokio() -> String {
    sleep(time::Duration::from_secs(2)).await;
    "hello tokio".to_owned()
}

struct Channel<T> {
    values: Mutex<VecDeque<T>>,
    notify: tokio::sync::Notify,
}

impl<T> Channel<T> {
    pub fn send(&self, value: T) {
        if let Ok(mut values) = self.values.lock() {
            values.push_back(value);
            self.notify.notify_one();
        }
    }

    pub async fn recv(&self) -> T {
        loop {
            if let Some(value) = self.values.lock().unwrap().pop_front() {
                return value;
            }
            self.notify.notified().await;
        }
    }
}

pub async fn tokio_tcp_server() {
    let quit = [113, 117, 105, 116, 13, 10];
    if let Ok(ref listerner) = TcpListener::bind("0.0.0.0:8080").await {
        loop {
            if let Ok((mut socket, addr)) = listerner.accept().await {
                info!("Recived new Client : {}", addr);
                tokio::spawn(async move {
                    let mut buf = [0; 24];
                    loop {
                        let n = match socket.read(&mut buf).await {
                            Ok(n) if n == 0 => return,
                            Ok(n) => {
                                info!("TCP server received {} bytes from {}", n, addr);
                                n
                            }
                            Err(e) => {
                                error!(
                                    r#"Failed to read from client {},
                                    ERROR: {:#?}"#,
                                    addr, e
                                );
                                return;
                            }
                        };
                        if buf[0..n].as_ref() == quit {
                            info!("Recived the quit command, disconnect the cline {}", addr);
                            if let Ok(_) = socket.shutdown().await {}
                            break;
                        }

                        if let Err(e) = socket.write_all(&buf[0..n]).await {
                            error!("Failed to write to socket, ERROR:{:#?}", e);
                            return;
                        }
                        info!("Write message to client {} finished", addr);
                    }
                });
            }
        }
    }
}

pub async fn tokio_unix_socket_server(socket_path: &str) {
    let quit = [113, 117, 105, 116, 13, 10];
    let parent_path = temp_dir();
    let bind_path = parent_path.as_path().join(socket_path);
    if let Ok(filesocks) = UnixListener::bind(&bind_path) {
        loop {
            if let Ok((mut socket, client)) = filesocks.accept().await {
                info!("Client {:?} connected", client);
                if let Err(err) = socket.write_all(b"hello unix socket").await {
                    error!("failed to write to client , ERROR:{:?}", err);
                    return;
                }
                if let Ok(_) = socket.readable().await {
                    let mut buffer = [0; 24];
                    if let Ok(size) = socket.try_read(&mut buffer) {
                        info!("Socket server recevied message :{:?}", &buffer[0..size]);
                        if buffer[0..size].as_ref() == quit {
                            break;
                        }
                    }
                }
            }
        }
        drop(filesocks);
        if let Ok(_) = tokio::fs::remove_file(bind_path).await {}
    }
}

pub async fn fs_direntry_tokio(path: &str) -> Result<Vec<OsString>, std::io::Error> {
    let mut vec_path = vec![];
    let mut entries = fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        vec_path.push(entry.file_name());
    }
    Ok(vec_path)
}

pub async fn read_file_to_bytes(path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs::File::open(path).await?;
    let mut buffer = [0; 10];
    let len = file.read(&mut buffer).await?;
    Ok(buffer[0..len].to_owned())
}

pub async fn read_file_to_string(path: &str, read_all: bool) -> Result<String, std::io::Error> {
    let file = fs::File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut buffer = String::new();
    if read_all {
        reader.read_to_string(&mut buffer).await?;
    } else {
        reader.read_line(&mut buffer).await?;
    }
    Ok(buffer)
}

pub async fn write_to_file(path: &str, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path).await?;
    file.write_all(content.as_bytes()).await?;
    file.sync_all().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    use super::*;
    use tokio;
    use tokio::net::{TcpStream, UnixStream};

    #[tokio::test]
    async fn test_hello_tokio() {
        let begin = time::Instant::now();
        let res = hello_tokio().await;
        assert!(time::Duration::from_secs(2) <= begin.elapsed());
        assert_eq!(res, "hello tokio")
    }

    #[tokio::test]
    async fn test_block() {
        let block_task = tokio::task::spawn_blocking(|| {});
        block_task.await.unwrap();
    }
    use tokio::fs;
    #[tokio::test]
    async fn test_tokio_fs() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        let res = builder.create("/tmp/zhangjl/luoyan/zhangzz").await.unwrap();
        assert_eq!((), res);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_tokio_fs_panic() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(false);
        let res = builder.create("/tmp/luoyan/zhangzz").await.unwrap();
        assert_eq!((), res);
    }

    #[tokio::test]
    async fn test_tokio_fs_walk() {
        let res =
            fs_direntry_tokio("/mnt/d/github.com/workrusts/cookbook/awesome/asynctokio").await;
        assert_eq!(vec!["Cargo.toml", "src"], res.unwrap());
    }

    #[tokio::test]
    #[should_panic]
    async fn test_tokio_fs_walk_panic() {
        let res = fs_direntry_tokio("/d/github.com/workrusts/cookbook/awesome/tokioasync").await;
        assert_eq!(vec!["Cargo.toml", "src"], res.unwrap());
    }

    #[tokio::test]
    async fn test_tokio_read_file_to_bytes() {
        let res = read_file_to_bytes("/tmp/zinit/git-process-output.zsh").await;
        assert_eq!(res.as_ref().unwrap().len(), 10);
        assert_eq!("#!/usr/bin", str::from_utf8(res.as_ref().unwrap()).unwrap());
    }

    #[tokio::test]
    async fn test_tokio_read_file() {
        let res = read_file_to_string("/tmp/zinit/git-process-output.zsh", false).await;
        assert_eq!(res.unwrap().as_str(), "#!/usr/bin/env zsh\n");
    }

    #[tokio::test]
    async fn test_tokio_read_file_write_and_read_all() {
        if let Ok(()) = write_to_file("/tmp/zhangjialong", "wife: luoyan, and child: zhangzz").await
        {
            let res = read_file_to_string("/tmp/zhangjialong", true).await;
            assert_eq!(res.unwrap().as_str(), "wife: luoyan, and child: zhangzz");
        }
    }

    #[tokio::test]
    async fn test_tokio_tcp_connect() {
        if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080").await {
            if let Ok(_) = stream.write_all(b"hello tcp server").await {
                let mut buffer = [0; 20];
                let mut buffer = tokio::io::ReadBuf::new(&mut buffer);
                if let Ok(_) =
                    futures::future::poll_fn(|ctx| stream.poll_peek(ctx, &mut buffer)).await
                {
                    assert_eq!("hello tcp server", str::from_utf8(buffer.filled()).unwrap());
                }
                if let Ok(_) = stream.shutdown().await {}
            }
        }
    }

    use tokio::io::Interest;
    #[tokio::test]
    #[ignore]
    async fn test_tokio_tcp_rw() {
        if let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await {
            //loop {
            if let Ok(ready) = stream.ready(Interest::READABLE | Interest::WRITABLE).await {
                if ready.is_readable() {
                    let mut data = vec![0; 1024];
                    match stream.try_read(&mut data) {
                        Ok(n) => println!("read {} bytes", n),
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            return;
                        }
                        Err(err) => println!("Error: {:?}", err),
                    }
                }

                if ready.is_writable() {
                    match stream.try_write(b"let`s rock") {
                        Ok(n) => println!("write {} bytes", n),
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            return;
                        }
                        Err(err) => println!("Error: {:?}", err),
                    }
                }
            }
            //}
        }
    }

    #[tokio::test]
    async fn test_tokio_unix_socket() {
        let quit = [113, 117, 105, 116, 13, 10];
        if let Ok(mut stream) = UnixStream::connect("/tmp/zhangjl.socket").await {
            if let Ok(_) = stream.readable().await {
                let mut buf = Vec::with_capacity(4096);
                if let Ok(size) = stream.try_read_buf(&mut buf) {
                    println!("Read {} bytes of message: {:?}", size, buf);
                }
                if let Ok(_) = stream.write_all(&quit).await {}
            }
        }
    }

    use tokio::runtime;
    #[test]
    fn test_tokio_runtime_multi_thread() {
        if let Ok(multi_rt) = runtime::Runtime::new() {
            let mut init = 10;
            multi_rt.block_on(async {
                init += 100;
            });
            assert_eq!(init, 110);
            drop(multi_rt);
        }

        if let Ok(multi_rt) = runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(8)
            .on_thread_start(|| println!("hello multi"))
            .on_thread_stop(|| println!("goodbye"))
            .build()
        {
            let mut init = 10;
            multi_rt.block_on(async {
                init += 100;
            });
            assert_eq!(init, 110);
            drop(multi_rt);
        }
    }

    #[test]
    fn test_tokio_runtime_current_thread() {
        if let Ok(current_rt) = runtime::Builder::new_current_thread().enable_all().build() {
            let mut init = 10;
            let mut str_owned = String::new();
            current_rt.block_on(async {
                println!("Using the current runtime");
                init += 200;
                str_owned = hello_tokio().await;
                let _ = current_rt
                    .spawn(async { println!("hello spawn function") })
                    .await;
            });
            current_rt.spawn_blocking(|| println!("hello spawn blocking function"));
            assert_eq!(init, 210);
            assert_eq!("hello tokio", str_owned);
            drop(current_rt);
        }
    }

    #[test]
    fn test_tokio_runtime_handler() {
        if let Ok(current_rt) = runtime::Builder::new_multi_thread().enable_all().build() {
            let handler = current_rt.handle();
            let task = handler.spawn(async { println!("handler task") });
            let _ = handler.block_on(async { task.await });
            drop(current_rt);
        }
    }

    use tokio::time;
    #[tokio::test]
    async fn test_tokio_time() {
        let mut ticker = time::interval(time::Duration::from_secs(1));
        let mut counter = 1;
        loop {
            ticker.tick().await;
            println!("hello ticker");
            counter += 1;
            if counter >= 10 {
                break;
            }
        }
    }

    use tokio::sync;
    #[tokio::test]
    async fn test_tokio_oneshot() {
        let (sender, recevier) = sync::oneshot::channel();
        tokio::spawn(async move {
            let res = hello_tokio().await;
            if let Ok(_) = sender.send(res) {}
        });

        if let Ok(res) = recevier.await {
            println!("The result is {}", res);
            assert_eq!("hello tokio", res);
        }
    }

    #[test]
    fn test_tokio_oneshot_block_on() {
        if let Ok(rt) = runtime::Builder::new_multi_thread().enable_all().build() {
            let (sender, recevier) = sync::oneshot::channel();
            let send_task = async {
                let res = hello_tokio().await;
                if let Ok(_) = sender.send(res) {}
            };
            let recv_task = async {
                if let Ok(res) = recevier.await {
                    println!("The result in test_tokio_oneshot_block_on is {}", res);
                    assert_eq!("hello tokio", res);
                }
            };

            rt.block_on(async { tokio::join!(send_task, recv_task) });
        }
    }

    #[test]
    fn test_tokio_oneshot_block_on_all() {
        if let Ok(rt) = runtime::Builder::new_multi_thread().enable_all().build() {
            let (sender, recevier) = sync::oneshot::channel();
            rt.block_on(async move {
                tokio::spawn(async move {
                    let res = hello_tokio().await;
                    if let Ok(_) = sender.send(res) {}
                });

                if let Ok(res) = recevier.await {
                    println!("The result in test_tokio_oneshot_block_on_all is {}", res);
                    assert_eq!("hello tokio", res);
                }
            });
        }
    }

    async fn hello_index(input: u32) -> String {
        format!("Hello, {}", input)
    }

    #[tokio::test]
    async fn test_tokio_mpsc() {
        let (sender, mut receiver) = sync::mpsc::channel(24);
        tokio::spawn(async move {
            for i in 1..10 {
                let res = hello_index(i).await;
                if let Ok(_) = sender.send(res).await {
                    println!("Send {} success", i);
                }
            }
        });

        let mut index = 1;
        while let Some(res) = receiver.recv().await {
            assert_eq!(format!("Hello, {}", index), res);
            index += 1;
        }
    }

    #[test]
    fn test_tokio_mpsc_block_on() {
        let (sender, mut receiver) = sync::mpsc::channel(24);
        if let Ok(rt) = runtime::Builder::new_multi_thread().enable_all().build() {
            rt.block_on(async move {
                tokio::spawn(async move {
                    for i in 1..10 {
                        let res = hello_index(i).await;
                        if let Ok(_) = sender.send(res).await {
                            println!("Send {} success", i);
                        }
                    }
                });

                let mut index = 1;
                while let Some(res) = receiver.recv().await {
                    assert_eq!(format!("Hello, {}", index), res);
                    index += 1;
                }
            });
        }
    }

    #[tokio::test]
    async fn test_tokio_mpsc_clone() {
        let (sender, mut receiver) = sync::mpsc::channel(24);
        for i in 1..10 {
            let sender_clone = sender.clone();
            tokio::spawn(async move { if let Ok(_) = sender_clone.send(i).await {} });
        }

        drop(sender);

        let mut index = 1;
        while let Some(res) = receiver.recv().await {
            assert_eq!(index, res);
            index += 1;
        }
    }

    struct Temp {
        age: u8,
    }

    #[tokio::test]
    async fn test_tokio_mpsc_clone_datatype() {
        let (sender, mut receiver) = sync::mpsc::channel::<Temp>(24);
        for i in 1..10 {
            let sender_clone = sender.clone();
            tokio::spawn(async move { if let Ok(_) = sender_clone.send(Temp { age: i }).await {} });
        }

        drop(sender);

        let mut index = 1;
        while let Some(res) = receiver.recv().await {
            assert_eq!(index, res.age);
            index += 1;
        }
    }

    enum Commond {
        Increment,
    }

    #[tokio::test]
    async fn test_tokio_mpsc_oneshot_combine() {
        let (cmd_sender, mut cmd_receiver) =
            sync::mpsc::channel::<(Commond, sync::oneshot::Sender<u32>)>(32);
        tokio::spawn(async move {
            let mut counter: u32 = 0;
            while let Some((cmd, oneshot_sender)) = cmd_receiver.recv().await {
                match cmd {
                    Commond::Increment => {
                        let prev = counter;
                        counter += 1;
                        if let Ok(_) = oneshot_sender.send(prev) {}
                    }
                }
            }
        });

        let mut join_handlers: Vec<tokio::task::JoinHandle<Option<u32>>> = vec![];
        for _ in 0..10 {
            let cmd_sender_clone = cmd_sender.clone();
            let join_handler = tokio::spawn(async move {
                let (oneshot_sender, oneshot_receiver) = sync::oneshot::channel::<u32>();
                if let Some(_) = cmd_sender_clone
                    .send((Commond::Increment, oneshot_sender))
                    .await
                    .ok()
                {
                    if let Ok(res) = oneshot_receiver.await {
                        return Some(res);
                    }
                }
                None
            });
            join_handlers.push(join_handler);
        }

        drop(cmd_sender);

        let mut res_number = vec![];
        for join_handler in join_handlers.drain(..) {
            if let Ok(Some(res)) = join_handler.await {
                res_number.push(res);
            }
        }

        assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], res_number);
    }

    #[tokio::test]
    async fn test_tokio_boardcast_channel() {
        let (sender, recevier_first) = sync::broadcast::channel(16);
        let recevier_second = sender.subscribe();
        let recevier_third = sender.subscribe();

        let mut handlers = vec![];

        for (mut recv, name) in [
            (recevier_first, stringify!(recevier_first)),
            (recevier_second, stringify!(recevier_second)),
            (recevier_third, stringify!(recevier_third)),
        ] {
            handlers.push(tokio::spawn(async move {
                let mut resvec = vec![];
                while let Ok(res) = recv.recv().await {
                    println!("recevier {} recevied {}", name, res);
                    resvec.push(res);
                }
                resvec
            }));
        }

        for i in 0..10 {
            if let Ok(_) = sender.send(i) {}
        }

        drop(sender);

        for joinhandle in handlers {
            if let Ok(rev) = joinhandle.await {
                assert_eq!(Vec::from_iter(0..10), rev);
            }
        }
    }

    #[tokio::test]
    async fn test_tokio_watch_basic() {
        let (watch_sender, mut watch_receiver) = sync::watch::channel("hello");

        tokio::spawn(async move { if let Ok(_) = watch_sender.send("world") {} });

        while let Ok(()) = watch_receiver.changed().await {
            println!("{}", *watch_receiver.borrow());
        }
    }

    use std::sync::Arc;
    #[tokio::test]
    async fn test_tokio_barrier() {
        let mut handlers = Vec::with_capacity(10);
        let barrier = Arc::new(sync::Barrier::new(10));
        for i in 0..10 {
            let barrier_clone = barrier.clone();
            handlers.push(tokio::spawn(async move {
                println!("Before wait in thread {}", i);
                let wait_result = barrier_clone.wait().await;
                println!("After wait");
                if wait_result.is_leader() {
                    println!("Thread {} is leader", i);
                }
                wait_result
            }));
        }

        let mut numbers = 0;
        for handle in handlers {
            if let Ok(wait_result) = handle.await {
                if wait_result.is_leader() {
                    numbers += 1;
                }
            }
        }

        assert_eq!(1, numbers);
    }

    #[tokio::test]
    async fn test_tokio_mutex() {
        let data1 = Arc::new(sync::Mutex::new(0));
        let data2 = Arc::clone(&data1);

        let handler = tokio::spawn(async move {
            let mut lock = data2.lock().await;
            *lock += 2;
            *lock
        });

        let res = handler.await.unwrap();
        println!("The result is {}, {:?}", res, data1);
        {
            let mut lock = data1.lock().await;
            *lock += 2;
        }
        println!("{:?}", data1);
    }

    #[tokio::test]
    async fn test_tokio_mutex_second() {
        let count = Arc::new(sync::Mutex::new(10));
        let copy_count = Arc::clone(&count);
        for i in 0..5 {
            let local_count = Arc::clone(&count);
            tokio::spawn(async move {
                for j in 0..10 {
                    let mut lock = local_count.lock().await;
                    *lock += 1;
                    println!("{}, {}, {}", i, j, lock);
                }
            });
        }
        loop {
            if *copy_count.lock().await > 50 {
                break;
            }
        }
        println!("Count hit 50");
    }

    #[tokio::test]
    async fn test_tokio_mutex_third() {
        let count = Arc::new(sync::Mutex::new(10));
        let rel = count.lock_owned().await;
        println!("The rel is {}", *rel);

        let count = Arc::new(sync::Mutex::new(10));
        let rel = count.lock().await;
        println!("The rel is {}", rel);

        use std::sync::Mutex;
        let count_std = Arc::new(Mutex::new(10));
        let rel = count_std.lock().unwrap();
        println!("rel is {}", rel);

        let count_std = Mutex::new(10);
        let rel = count_std.into_inner().unwrap();
        println!("rel is {}", rel);
    }

    #[test]
    fn block_test_tokio() {
        async fn __in__() {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            __serial_command__!(123);
        }
        async fn __out__() {
            println!("hello world");
        }

        if let Ok(multi_rt) = runtime::Runtime::new() {
            multi_rt.block_on(async {
                tokio::join!(__in__(), __out__());
            });
            drop(multi_rt);
        }

        println!("let`s go out of the async tasks");
    }

    #[tokio::test]
    async fn test_tokio_notify_base() {
        let notity = Arc::new(tokio::sync::Notify::new());
        let notify_copy = notity.clone();

        let handle = tokio::spawn(async move {
            notify_copy.notified().await;
            println!("Notify arrived");
        });

        //tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Send the notify");
        notity.notify_one();
        //notity.notify_waiters();
        if let Ok(_) = handle.await {}
    }

    #[tokio::test]
    async fn test_tokio_notify_channel() {
        let channel: Channel<u8> = Channel {
            values: Mutex::new(VecDeque::new()),
            notify: tokio::sync::Notify::new(),
        };

        channel.send(32);
        let res = channel.recv().await;
        println!("The res is {}", res);
    }

    #[tokio::test]
    async fn test_tokio_notify_all() {
        let notify = Arc::new(tokio::sync::Notify::new());
        let notify_copy = notify.clone();

        let notified_1 = notify.notified();
        let notified_2 = notify.notified();

        tokio::spawn(async move {
            println!("sending notifications");
            notify_copy.notify_waiters();
        });

        notified_1.await;
        notified_2.await;
    }

    #[tokio::test]
    async fn test_tokio_notify_all_other() {
        let notify = Arc::new(tokio::sync::Notify::new());
        let notify_copy_1 = notify.clone();
        let notify_copy_2 = notify.clone();

        let task_1 = tokio::spawn(async move {
            notify_copy_1.notified().await;
            println!("task1 completed");
        });

        notify.notify_one();
        task_1.await;

        let task_2 = tokio::spawn(async move {
            notify_copy_2.notified().await;
            println!("task2 completed");
        });

        notify.notify_one();
        task_2.await;
    }

    #[tokio::test]
    async fn test_tokio_rwlock() {
        let rwlock = Arc::new(tokio::sync::RwLock::new(100));
        let rw_lock_copy_1 = rwlock.clone();
        let rw_lock_copy_2 = rwlock.clone();

        let task_1 = tokio::spawn(async move {
            let res1 = rw_lock_copy_1.read().await;
            println!("The res1 is {}", res1);
        });

        let task_2 = tokio::spawn(async move {
            let res2 = rw_lock_copy_2.read().await;
            println!("The res2 is {}", res2);
        });

        {
            let mut new_value = rwlock.write().await;
            *new_value += 100;
            println!("Send complete nofify");
        }

        let _ = tokio::join!(task_1, task_2);

        let mut owned = rwlock.write_owned().await;
        *owned = 300;
        println!("{:?}", owned);
    }

    #[tokio::test]
    async fn test_tokio_singal_ctrl_c() {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            println!("ctrl-c received!");
        }
    }

    #[tokio::test]
    async fn test_tokio_linux_signal() {
        if let Ok(mut stream) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::quit())
        {
            loop {
                stream.recv().await;
                println!("Got the signal quit");
            }
        }
    }

    use tokio_stream::{self as stream, StreamExt};

    #[tokio::test]
    async fn test_tokio_select_pin() {
        let mut stream = stream::iter(vec![1, 2, 3]);
        let __sleep__ = tokio::time::sleep(tokio::time::Duration::from_secs(1));
        tokio::pin!(__sleep__);

        loop {
            tokio::select! {
                maybe_v = stream.next() => {
                    if let Some(v) = maybe_v {
                        println!("Got = {}", v);
                    }else {
                        println!("complete");
                        break;
                    }
                }
                _ = &mut __sleep__ => {
                    println!("Time out");
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_tokio_select_ticker() {
        let mut stream = stream::iter(vec![1, 2, 3]);
        let mut __ticker__ = tokio::time::interval(tokio::time::Duration::from_millis(1));
        loop {
            tokio::select! {
                _ = __ticker__.tick() => {
                    println!("waiting for the opertion complete...");
                }
                maybe_v = stream.next() => {
                    if let Some(v) = maybe_v {
                        println!("Got = {}", v);
                    }else {
                        println!("complete");
                        break;
                    }
                }
            }
        }
    }
}
