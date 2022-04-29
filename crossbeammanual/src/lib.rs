use std::thread;
use std::time;

use crossbeam;
//use crossbeam_channel;

pub fn find_max(array: &[u8]) -> Option<u8> {
    const THRESHOLD: usize = 2;
    if array.len() <= THRESHOLD {
        return array.iter().cloned().max();
    }

    let middle = array.len() / 2;
    let (left, right) = array.split_at(middle);

    if let Ok(res) = crossbeam::scope(|task| {
        let thread_left = task.spawn(|_| find_max(left));
        let thread_right = task.spawn(|_| find_max(right));

        let max_left = thread_left.join().unwrap()?;
        let max_right = thread_right.join().unwrap()?;

        Some(max_left.max(max_right))
    }) {
        return res;
    }
    None
}

pub fn channel_usage() {
    let (sender_first, recver_first) = crossbeam::channel::bounded(1);
    let (sender_second, recver_second) = crossbeam::channel::bounded(1);
    let msg_num = 4;
    let worker_number = 8;

    let res = crossbeam::scope(|task| {
        task.spawn(|_| {
            for i in 0..msg_num {
                if let Ok(_) = sender_first.send(i) {
                    println!("Source send {}", i);
                } else {
                    println!("Faild to send msg from source");
                }
            }
            drop(sender_first);
        });

        for _ in 0..worker_number {
            let (sender_clone, recver_clone) = (sender_second.clone(), recver_first.clone());
            task.spawn(move |_| {
                thread::sleep(time::Duration::from_millis(50));
                for msg in recver_clone.iter() {
                    println!("Worker {:?} received {}", thread::current().id(), msg);
                    if let Err(err) = sender_clone.send(msg * 2) {
                        println!("Failed to send complete message: {:?}", err);
                    }
                }
            });
        }

        drop(sender_second);

        for msg in recver_second.iter() {
            println!("The result is {}", msg);
        }
    });

    if let Ok(_) = res {}
}

pub fn unbounded_channel() -> u32 {
    use crossbeam::atomic;
    let instance = atomic::AtomicCell::<u32>::new(21);
    let (sender, recevier) = crossbeam::channel::unbounded();
    let msg_number = 5;
    if let Ok(_) = crossbeam::scope(|task| {
        task.spawn(|_| {
            for i in 0..msg_number {
                if let Ok(_) = sender.send(i) {
                    instance.fetch_add(i);
                    thread::sleep(time::Duration::from_millis(100));
                }
            }
        });
    }) {
        for _ in 0..msg_number {
            if let Ok(msg) = recevier.recv() {
                println!("Received message :{}", msg);
            }
        }
    }

    drop(sender);
    instance.into_inner()
}

pub fn unbounded_channel_lock() {
    use crossbeam::atomic;
    let array: [u8; 5] = [1, 2, 3, 4, 5];
    let instance = atomic::AtomicCell::new(array);
    let (sender, recevier) = crossbeam::channel::unbounded();
    let msg_number = 5;
    if let Ok(_) = crossbeam::scope(|task| {
        task.spawn(|_| {
            for i in 0..msg_number {
                if let Ok(_) = sender.send(i) {
                    let mut instance_value = instance.load();
                    instance_value[i] = instance_value[i] + 10 as u8;
                    instance.store(instance_value);
                    thread::sleep(time::Duration::from_millis(100));
                }
            }
        });
    }) {
        for _ in 0..msg_number {
            if let Ok(msg) = recevier.recv() {
                println!("Received message :{}", msg);
            }
        }
    }

    println!("{:?}", instance.load());

    drop(sender);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    use super::*;

    #[test]
    fn test_find_max() {
        let array: &[u8] = &[1, 2, 87, 32, 127, 23, 99];
        let res = find_max(array);
        assert_eq!(127, res.unwrap());

        let array: [u8; 9] = [12, 123, 255, 12, 34, 45, 23, 11, 90];
        let res = find_max(&array);
        assert_eq!(255, res.unwrap());
    }

    #[test]
    fn test_channel_usage() {
        channel_usage();
    }

    #[test]
    fn test_channel_unbounded() {
        let res = unbounded_channel();
        assert_eq!(31, res);
    }

    use chrono::TimeZone;
    use crossbeam::atomic;

    #[test]
    fn test_crossbeam_atomic_cell() {
        let instance = atomic::AtomicCell::new(21);

        let load_value = instance.load();
        assert_eq!(21, load_value);

        instance.store(128);
        assert_eq!(128, instance.load());

        instance.swap(256);
        assert_eq!(256, instance.load());

        let take_value = instance.take();
        assert_eq!(0, instance.load());
        assert_eq!(256, take_value);

        let res = instance.compare_exchange(0, 100);
        assert_eq!(res, Ok(0));
        let res = instance.compare_exchange(0, 200);
        assert_eq!(res, Err(100));

        let res = instance.compare_exchange(instance.load(), 300);
        assert_eq!(300, instance.load());
        assert_eq!(res, Ok(100));

        let instance_value = instance.into_inner();
        assert_eq!(300, instance_value);
    }

    #[test]
    fn test_crossbeam_atomic_lock() {
        assert_eq!(true, atomic::AtomicCell::<usize>::is_lock_free());
        assert_eq!(atomic::AtomicCell::<()>::is_lock_free(), true);
        struct Foo {
            bar: isize,
        }
        assert_eq!(atomic::AtomicCell::<Foo>::is_lock_free(), true);
        struct FooComplexe {
            bar: isize,
            status: bool,
        }
        assert_eq!(atomic::AtomicCell::<FooComplexe>::is_lock_free(), false);
        assert_eq!(atomic::AtomicCell::<[u8; 10]>::is_lock_free(), false);

        unbounded_channel_lock();
    }

    #[test]
    fn test_channel_bounded_zero_capacity() {
        let (non_sender, non_recevier) = crossbeam_channel::bounded(0);
        thread::spawn(move || {
            non_sender.send("hello world").unwrap();
            drop(non_sender);
        });

        assert_eq!(Ok("hello world"), non_recevier.recv());

        drop(non_recevier);

        let (non_sender, non_recevier) = crossbeam_channel::bounded(0);
        thread::spawn(move || {
            assert_eq!(non_recevier.recv(), Ok("hello world"));
            drop(non_recevier);
        });

        non_sender.send("hello world").unwrap();
        drop(non_sender);
    }

    #[test]
    fn test_crossbeam_shared_channel() {
        let (first_sender, first_recevier) = crossbeam_channel::bounded(0);
        let (second_sender, second_recevier) = (first_sender.clone(), first_recevier.clone());

        thread::spawn(move || {
            assert_eq!(second_recevier.recv(), Ok("This is the first sender"));
            second_sender.send("This is the second sender").unwrap();
            println!("OK");
        });

        first_sender.send("This is the first sender").unwrap();
        assert_eq!(first_recevier.recv(), Ok("This is the second sender"));

        let (sender_1, recevier_1) = crossbeam_channel::unbounded();
        let (sender_2, recevier_2) = (sender_1.clone(), recevier_1.clone());
        let (sender_3, recevier_3) = (sender_1.clone(), recevier_1.clone());

        thread::spawn(move || {
            sender_1.send(10).unwrap();
            sender_2.send(20).unwrap();
            sender_3.send(30).unwrap();
        });
        assert_eq!(recevier_3.recv(), Ok(10));
        assert_eq!(recevier_1.recv(), Ok(20));
        assert_eq!(recevier_2.recv(), Ok(30));
        println!("end the thread");

        let (scope_sender, scope_receiver) = crossbeam_channel::bounded(0);
        crossbeam::thread::scope(|task| {
            task.spawn(|_| {
                let recv = scope_receiver.recv().unwrap();
                println!("task received {}", recv);
                scope_sender.send(100).unwrap();
            });
            scope_sender.send(200).unwrap();
            let recv = scope_receiver.recv().unwrap();
            assert_eq!(100, recv);
        })
        .unwrap();
        drop(scope_sender);
        assert_eq!(true, scope_receiver.is_empty());
    }

    #[test]
    fn test_crossbeam_channel_iter() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        thread::spawn(move || {
            sender.send(1).unwrap();
            sender.send(2).unwrap();
            sender.send(3).unwrap();
            drop(sender);
        });

        /*
         * let v: Vec<_> = receiver.iter().collect();
        println!("result is {:?}", v);
        */
        for item in receiver {
            println!("result is {:?}", item);
        }
    }

    #[test]
    fn test_crossbeam_channel_select() {
        let (sender_first, recevier_first) = crossbeam_channel::unbounded();
        let (sender_second, recevier_second) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            sender_first.send(10).unwrap();
        });
        thread::spawn(move || {
            sender_second.send(100).unwrap();
        });

        crossbeam::channel::select! {
            recv(recevier_first) -> message_first => println!("The first message is {:?}", message_first),
            recv(recevier_second) -> message_second=> println!("The second message is {:?}", message_second),
            default(std::time::Duration::from_secs(10)) => println!("timeout"),
        }
    }

    #[test]
    fn test_crossbeam_time_channel() {
        let start = time::Instant::now();
        let ticker = crossbeam_channel::tick(time::Duration::from_millis(50));
        let after__ = crossbeam_channel::after(time::Duration::from_secs(5));

        loop {
            crossbeam_channel::select! {
                recv(ticker) -> _ => println!("Now elapsed :{:?}", start.elapsed()),
                recv(after__) -> _ => {
                    println!("Now is time out");
                    break;
                },
            }
        }
    }

    #[test]
    fn test_crossbeam_never_channel() {
        let (sender, recevier) = crossbeam_channel::unbounded();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(1));
            sender.send(100).unwrap();
        });

        let duration = Some(time::Duration::from_millis(50));
        let timeout = duration
            .map(|d| crossbeam_channel::after(d))
            .unwrap_or(crossbeam_channel::never());

        crossbeam_channel::select! {
            recv(recevier) -> msg => println!("The result is {:?}", msg),
            recv(timeout) -> _ => println!("Time out"),
        }
    }

    #[test]
    fn test_crossbeam_at_channel() {
        let (sender, recevier) = crossbeam_channel::unbounded();

        let do_action = time::Instant::now() + time::Duration::from_secs(5);
        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(10));
            sender.send(100).unwrap();
        });

        crossbeam_channel::select! {
            recv(crossbeam_channel::at(do_action)) -> _ => println!("time out"),
            recv(recevier) -> msg => println!("The message is {:?}", msg),
        }
    }

    #[test]
    fn test_crossbeam_channel_block() {
        let (sender_first, recver_first) = crossbeam::channel::unbounded();
        let (sender_second, recver_second) = crossbeam::channel::unbounded();

        sender_first.send(1).unwrap();

        crossbeam_channel::select! {
            recv(recver_first) -> msg => println!("The message is {:?}", msg),
            send(sender_second, 20) -> res => {
                assert_eq!(res, Ok(()));
                assert_eq!(recver_second.recv(), Ok(20));
                println!("on the send branch");
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_crossbeam_select_struct_select() {
        let mut sel = crossbeam_channel::Select::new();

        assert!(sel.try_select().is_err());

        let _ = sel.select();
    }

    #[test]
    fn test_crossbeam_select_struct() {
        let (sender, receiver) = crossbeam_channel::unbounded::<u8>();
        let mut sel = crossbeam_channel::Select::new();
        let idx = sel.send(&sender);
        let oper = sel.select();
        oper.send(&sender, 12).unwrap();

        let idx_r = sel.recv(&receiver);
        let oper = sel.select();
        let res = oper.recv(&receiver).unwrap();

        println!("The idx is {}, {}, ={}", idx, idx_r, res);
    }

    #[test]
    fn test_crossbeam_select_implement() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let (sender_2, receiver_2) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(1));
            sender.send(10).unwrap();
        });

        thread::spawn(move || {
            sender_2.send(20).unwrap();
        });

        let mut sel = crossbeam_channel::Select::new();
        let operator1 = sel.recv(&receiver);
        let operator2 = sel.recv(&receiver_2);

        let operation = sel.select();

        match operation.index() {
            idx if idx == operator2 => println!(
                "The recevier second  received {:?}",
                operation.recv(&receiver_2)
            ),
            idx if idx == operator1 => {
                println!("The recevier received {:?}", operation.recv(&receiver))
            }
            _ => unreachable!(),
        }
    }

    #[test]
    #[should_panic]
    fn test_crossbeam_select_timeout() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let (sender_2, receiver_2) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(1));
            sender.send(10).unwrap();
        });

        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(2));
            sender_2.send(20).unwrap();
        });

        let mut sel = crossbeam_channel::Select::new();
        let operator1 = sel.recv(&receiver);
        let operator2 = sel.recv(&receiver_2);

        let operation = sel.select_timeout(time::Duration::from_millis(500));

        match operation {
            Err(_) => panic!("Time out for waiting the select complete"),
            Ok(operation) => match operation.index() {
                idx if idx == operator1 => {
                    println!("The recevier received {:?}", operation.recv(&receiver))
                }
                idx if idx == operator2 => println!(
                    "The recevier second  received {:?}",
                    operation.recv(&receiver_2)
                ),
                _ => unreachable!(),
            },
        }
    }

    #[test]
    fn test_crossbeam_ready() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let (sender_2, receiver_2) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(1));
            sender.send(10).unwrap();
        });

        thread::spawn(move || {
            thread::sleep(time::Duration::from_secs(2));
            sender_2.send(20).unwrap();
        });

        let mut sel = crossbeam_channel::Select::new();
        let operator1 = sel.recv(&receiver);
        let operator2 = sel.recv(&receiver_2);

        match sel.ready() {
            idx if idx == operator1 => assert_eq!(receiver.recv(), Ok(10)),
            idx if idx == operator2 => assert_eq!(receiver_2.try_recv(), Ok(20)),
            _ => unreachable!(),
        }
    }
}
