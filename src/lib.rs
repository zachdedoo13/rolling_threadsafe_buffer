#![cfg_attr(not(test), no_std)]

use core::sync::atomic::AtomicBool;
use core::option::Option::{None, Some};
use core::marker::Send;


pub struct RollingBuffer<const S: usize, T: Default + Copy> {
    data: [T; S],
    write_head: usize,
    read_head: usize,
}
impl<const S: usize, T: Default + Copy> RollingBuffer<S, T> {
    pub const fn new(def: T) -> Self {
        Self {
            data: [def; S],
            write_head: 0,
            read_head: 0,
        }
    }

    #[inline(always)]
    fn increase_head(index: usize) -> usize {
        let t = index + 1;
        if t == S { 0 } else { t }
    }

    /// May loop read head if not called at a slower rate then read
    #[inline(always)]
    pub unsafe fn write_unchecked(&mut self, data: T) {
        self.data[self.write_head] = data;
        self.write_head = Self::increase_head(self.write_head);
    }

    /// May loop write head if not called at a faster rate then write
    #[inline(always)]
    pub unsafe fn read_unchecked(&mut self) -> Option<&T> {
        if self.read_head != self.write_head {
            let ret = Some(&self.data[self.read_head]);
            self.read_head = Self::increase_head(self.read_head);
            ret
        } else {
            None
        }
    }

    /// returns false if read head doesn't keep up
    #[inline(always)]
    pub fn write(&mut self, data: T) -> bool {
        let next = Self::increase_head(self.write_head);
        if next == self.read_head {
            return false
        };

        self.data[self.write_head] = data;
        self.write_head = next;

        true
    }

    #[inline(always)]
    pub fn read(&mut self) -> Option<&T> {
        if self.read_head != self.write_head {
            let next = Self::increase_head(self.read_head);
            let ret = Some(&self.data[self.read_head]);
            self.read_head = next;
            ret
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_write_unchecked() {
        let mut buff: RollingBuffer<4, usize> = RollingBuffer::new(0);
        unsafe {
            buff.write_unchecked(1);
            buff.write_unchecked(2);
            buff.write_unchecked(3);
            assert_eq!(buff.data, [1, 2, 3, 0]);

            buff.write_unchecked(4);
            buff.write_unchecked(5);
            assert_eq!(buff.data, [5, 2, 3, 4]);
        }
    }

    #[test]
    fn read_write_unchecked() {
        let mut buff: RollingBuffer<4, usize> = RollingBuffer::new(0);
        unsafe {
            buff.write_unchecked(1);
            buff.write_unchecked(2);
            buff.write_unchecked(3);
            assert_eq!(buff.data, [1, 2, 3, 0]);

            assert_eq!(buff.read_unchecked(), Some(&1));
            assert_eq!(buff.read_unchecked(), Some(&2));
            assert_eq!(buff.read_unchecked(), Some(&3));
            assert_eq!(buff.read_unchecked(), None);

            buff.write_unchecked(54);
            assert_eq!(buff.read_unchecked(), Some(&54));
        }
    }

    #[test]
    fn just_write_checked() {
        let mut buff: RollingBuffer<5, usize> = RollingBuffer::new(0);
        buff.write(1);
        buff.write(2);
        buff.write(3);
        assert_eq!(buff.write(4), true);
        assert_eq!(buff.write(5), false);
    }

    #[test]
    fn read_write_checked() {
        let mut buff: RollingBuffer<4, usize> = RollingBuffer::new(0);
        buff.write(1);
        buff.write(2);
        assert_eq!(buff.read(), Some(&1));
        buff.write(3);
        assert_eq!(buff.write(4), true);

        // assert_eq!(buff.write(5), true);
        // assert_eq!(buff.write(6), false);
    }

    #[test]
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

    #[cfg(test)]
    mod threaded {
        use super::*;
        use std::collections::HashSet;
        use std::sync::atomic::Ordering;
        use std::thread::{sleep, spawn};
        use std::time::Duration;

        const ITER_C: usize = 60;

        // #[test]
        // fn write_limited_unchecked() {
        //     static KILL: AtomicBool = AtomicBool::new(false);
        //     static DATA: GlobalData<RollingBuffer<25, i32>> =
        //         GlobalData::new(RollingBuffer::new(0));
        //
        //     let read = spawn(|| {
        //         let r = DATA.get_mut_ref();
        //         let mut seen = HashSet::new();
        //         loop {
        //             if KILL.load(Ordering::SeqCst) {
        //                 break;
        //             };
        //
        //             if let Some(val) = unsafe { r.read_unchecked() } {
        //                 // println!("READ {val}");
        //                 if seen.contains(val) {
        //                     panic!()
        //                 } else {
        //                     seen.insert(*val);
        //                 };
        //             }
        //
        //             sleep(Duration::from_millis(0));
        //         }
        //     });
        //     let write = spawn(|| {
        //         let w = DATA.get_mut_ref();
        //         let mut i = 0;
        //         loop {
        //             if KILL.load(Ordering::SeqCst) {
        //                 break;
        //             };
        //
        //             unsafe {
        //                 // println!("WRITE {i}");
        //                 w.write_unchecked(i);
        //             }
        //             i += 1;
        //             sleep(Duration::from_millis(40));
        //
        //             if i >= ITER_C as i32 {
        //                 KILL.store(true, Ordering::SeqCst);
        //             };
        //         }
        //     });
        //
        //     read.join().unwrap();
        //     write.join().unwrap();
        // }
        //
        // #[test]
        // fn write_limited_chunk_unchecked() {
        //     static KILL: AtomicBool = AtomicBool::new(false);
        //     static DATA: GlobalData<RollingBuffer<25, i32>> =
        //         GlobalData::new(RollingBuffer::new(0));
        //
        //     let read = spawn(|| {
        //         let r = DATA.get_mut_ref();
        //         let mut seen = HashSet::new();
        //         loop {
        //             if KILL.load(Ordering::SeqCst) {
        //                 break;
        //             };
        //
        //             for _ in 0..6 {
        //                 if let Some(val) = unsafe { r.read_unchecked() } {
        //                     // println!("READ {val}");
        //                     if seen.contains(val) {
        //                         panic!()
        //                     } else {
        //                         seen.insert(*val);
        //                     };
        //                 }
        //             }
        //
        //             sleep(Duration::from_millis(30));
        //         }
        //     });
        //     let write = spawn(|| {
        //         let w = DATA.get_mut_ref();
        //         let mut i = 0;
        //         loop {
        //             if KILL.load(Ordering::SeqCst) {
        //                 break;
        //             };
        //
        //             unsafe {
        //                 // println!("WRITE {i}");
        //                 w.write_unchecked(i);
        //             }
        //             i += 1;
        //             sleep(Duration::from_millis(10));
        //
        //             if i >= ITER_C as i32 {
        //                 KILL.store(true, Ordering::SeqCst);
        //             };
        //         }
        //     });
        //
        //     read.join().unwrap();
        //     write.join().unwrap();
        // }

        #[cfg(test)]
        mod safe {
            #![allow(static_mut_refs)]

            use super::*;

            use std::collections::HashSet;
            use std::path::Component::ParentDir;
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::thread::{sleep, spawn};
            use std::time::Duration;

            const ITER_C: usize = 60;

            #[test]
            fn test() {
                static KILL: AtomicBool = AtomicBool::new(false);
                static mut DATA: RollingBuffer<25, i32> = RollingBuffer::new(0);

                let read = spawn(|| {
                    let mut seen = HashSet::new();
                    loop {
                        if KILL.load(Ordering::SeqCst) {
                            break;
                        };


                        if let Some(val) = unsafe { DATA.read() } {
                            // println!("READ {val}");
                            if seen.contains(val) {
                                panic!()
                            } else {
                                seen.insert(*val);
                            };
                        } else {
                            // print!(" | ");
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
                        // println!("Wrote {i}");

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

            #[test]
            fn snake() {
                static KILL: AtomicBool = AtomicBool::new(false);
                static mut DATA: RollingBuffer<25, i32> = RollingBuffer::new(0);
                static SEL: AtomicBool = AtomicBool::new(false);

                let read = spawn(|| {
                    'outer: loop {
                        if KILL.load(Ordering::SeqCst) {
                            break;
                        }
                        unsafe {
                            if SEL.load(Ordering::SeqCst) {
                                for _ in 0..10 {
                                    if let None = DATA.read() {
                                        panic!()
                                    }
                                }
                                for _ in 0..53 {
                                    if let None = DATA.read() {
                                        SEL.store(false, Ordering::SeqCst);
                                        continue 'outer;
                                    }
                                }

                            }
                        }
                    }
                });

                let write = spawn(|| {
                    let mut c = 0;
                    'outer: loop {
                        if c == 5 {
                            KILL.store(true, Ordering::SeqCst);
                            break;
                        }
                        unsafe {
                            if !SEL.load(Ordering::SeqCst) {
                                c += 1;
                                for i in 0..15 {
                                    if !DATA.write(i) {
                                        panic!()
                                    }
                                }
                                for x in 22..156 {
                                    if !DATA.write(x) {
                                        SEL.store(true, Ordering::SeqCst);
                                        continue 'outer;
                                    }
                                }
                            }
                        }
                    }
                });

                read.join().unwrap();
                write.join().unwrap();
            }
        }
    }
}
