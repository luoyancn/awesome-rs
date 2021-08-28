use futurebase;
use futures::executor::block_on;
use futures::FutureExt;

#[test]
fn test_number() {
    let num = futurebase::Number::new(98);
    let res = block_on(num);
    assert_eq!(98, res);

    let num = async { futurebase::Number::new(100).await };
    let res = block_on(num);
    assert_eq!(100, res);

    println!("The res is {}", res);
}

#[test]
fn test_get_u8() {
    let num = futurebase::get_u8();
    let res = block_on(num);
    assert_eq!(64, res);

    let num = async { futurebase::get_u8().await };
    let res = block_on(num);
    assert_eq!(64, res);

    let num = futurebase::get_u8();
    let mapped = num.map(|i| i * 3);
    let maps = block_on(mapped);
    assert_eq!(maps, 64 * 3);
}
