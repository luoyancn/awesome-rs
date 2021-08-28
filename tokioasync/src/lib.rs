use std::time;
use tokio::time::sleep;

pub async fn hello_tokio() -> String {
    sleep(time::Duration::from_secs(2)).await;
    "hello tokio".to_owned()
}

pub async fn frist_tokio() {}
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
}
