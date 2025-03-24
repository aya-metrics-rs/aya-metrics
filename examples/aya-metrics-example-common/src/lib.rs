// #![no_std] except for tests and user space!
#![cfg_attr(not(any(test, feature = "user")), no_std)]

#[derive(Copy, Clone)]
pub enum MyCounter {
    Packets,
    Bytes,
}

impl aya_metrics_common::Counter for MyCounter {
    #[cfg(any(test, feature = "user"))]
    fn name(self) -> String {
        match self {
            MyCounter::Packets => "packets_counter".to_string(),
            MyCounter::Bytes => "bytes_counter".to_string(),
        }
    }

    fn index(&self) -> u32 {
        match self {
            MyCounter::Packets => 0,
            MyCounter::Bytes => 1,
        }
    }
}
