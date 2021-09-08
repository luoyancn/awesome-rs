use std::env::temp_dir;
use std::ffi::OsString;
use std::str;
use std::time;

use futures;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::time::sleep;

#[macro_use]
extern crate slog_logger;

pub async fn hello_tokio() -> String {
    sleep(time::Duration::from_secs(2)).await;
    "hello tokio".to_owned()
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
            fs_direntry_tokio("/mnt/d/github.com/workrusts/cookbook/awesome/tokioasync").await;
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
}
