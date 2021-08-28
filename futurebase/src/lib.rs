use futures::{
    future::{self, Either, Future, FutureExt},
    pin_mut,
};
use std::task::Poll;

pub struct Number {
    number: u8,
    polled: bool,
}

impl Number {
    pub fn new(number: u8) -> Self {
        Self {
            number: number,
            polled: false,
        }
    }
}

impl Future for Number {
    type Output = u8;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut this = self.get_mut();
        if this.polled {
            Poll::Ready(this.number)
        } else {
            cx.waker().wake_by_ref();
            this.polled = true;
            Poll::Pending
        }
    }
}

pub async fn get_u8() -> u8 {
    64
}

pub async fn fake_select<A, B>(a: A, b: B) -> impl Future<Output = (A::Output, B::Output)>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    future::select(a, b).then(|either| match either {
        Either::Left((x, b)) => b.map(move |y| (x, y)).left_future(),
        Either::Right((y, a)) => a.map(move |x| (x, y)).right_future(),
    })
}

pub async fn fake_select_two() -> u8 {
    let future1 = async {
        future::pending::<()>().await;
        1
    };
    let future2 = async { future::ready(2).await };
    pin_mut!(future1);
    pin_mut!(future2);
    match future::select(future1, future2).await {
        Either::Left((value, _ignore_future_2)) => value,
        Either::Right((value, _ignore_future_1)) => value,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    use super::*;
    use futures::executor::block_on;
    use futures::stream::{self, StreamExt};
    #[test]
    fn test_number_new() {
        let num = Number::new(9);
        assert_eq!(num.number, 9);
        assert_eq!(num.polled, false);
    }
    #[test]
    fn test_number_new_false() {
        let num = Number::new(10);
        assert_ne!(num.number, 9);
        assert_ne!(num.polled, true);
    }
    #[test]
    fn test_number_future() {
        let num = Number::new(10);
        let res = block_on(num);
        assert_eq!(res, 10);
    }
    #[test]
    fn test_number_future_false() {
        let num = Number::new(10);
        let res = block_on(num);
        assert_ne!(res, 100);
    }
    #[test]
    fn test_futures_stream() {
        let mut stream_int = stream::iter(1..=9);
        assert_eq!(block_on(stream_int.next()), Some(1));
        let mut stream_int = stream::iter(vec![9, 8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(block_on(stream_int.next()), Some(9));

        let stream_int = stream::iter(1..=3);
        let stream_int = stream_int.map(|input| input * 3);
        assert_eq!(vec![3, 6, 9], block_on(stream_int.collect::<Vec<_>>()));

        let stream_int = stream::iter(1..=3);
        let mut stream_k_v = stream_int.enumerate();
        assert_eq!(Some((0, 1)), block_on(stream_k_v.next()));

        let stream_int = stream::iter(1..=3);
        let then_result = stream_int.then(|input| async move { input * 3 });
        assert_eq!(vec![3, 6, 9], block_on(then_result.collect::<Vec<_>>()));
    }

    #[test]
    fn test_select_either() {
        let res = block_on(fake_select_two());
        assert_eq!(2, res);
    }

    #[test]
    fn test_stream_future() {
        let mut streamf = stream::iter(1..=10);
        let res = block_on(streamf.next());
        assert_eq!(Some(1), res);

        let all_item = block_on(streamf.collect::<Vec<u8>>());
        assert_eq!(vec![2, 3, 4, 5, 6, 7, 8, 9, 10], all_item);
    }
}
