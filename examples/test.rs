#![allow(static_mut_refs)]

use rolling_threadsafe_buffer::RollingBuffer;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{sleep, spawn};
use std::time::Duration;

const ITER_C: usize = 60;

fn main() {
    snake();
}

fn other() {
    static KILL: AtomicBool = AtomicBool::new(false);
    static mut DATA: RollingBuffer<25, i32> = RollingBuffer::new(0);

    let read = spawn(|| {
        let mut seen = HashSet::new();
        loop {
            if KILL.load(Ordering::SeqCst) {
                break;
            };

            if let Some(val) = unsafe { DATA.read() } {
                println!("READ {val}");
                if seen.contains(val) {
                    panic!()
                } else {
                    seen.insert(*val);
                };
            } else {
                print!(" | ");
            }

            sleep(Duration::from_millis(2));
        }
    });

    let write = spawn(|| {
        let mut i = 0;
        loop {
            if KILL.load(Ordering::SeqCst) {
                break;
            };

            while !unsafe { DATA.write(i) } {
                sleep(Duration::from_millis(10))
            }
            println!("Wrote {i}");

            i += 1;
            sleep(Duration::from_millis(5));

            if i >= ITER_C as i32 {
                KILL.store(true, Ordering::SeqCst);
            };
        }
    });

    read.join().unwrap();
    write.join().unwrap();
}

fn snake() {
    let mut buffer: RollingBuffer<10, i32> = RollingBuffer::new(0);

    for c in 0..5 {
        // Write as much as possible
        for i in 0..9 {
            assert!(buffer.write(i), "Index {i} iter {c}");
        }
        for _ in 0..50 {
            assert_eq!(buffer.write(0), false);
        }

        // Read as much as possible
        for i in 0..9 {
            assert_eq!(buffer.read(), Some(&i), "Index {i} iter {c}");
        }
        for _ in 0..50 {
            assert_eq!(buffer.read(), None);
        }

        // Buffer should now be empty
        assert_eq!(buffer.read(), None);
    }
}
