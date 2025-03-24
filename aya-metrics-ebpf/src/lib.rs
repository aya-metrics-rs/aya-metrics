// #![no_std] except for tests and user space!
#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]

//! Provides counter functionality with testable no_std implementations for use in BPF.

#[cfg(any(test, target_arch = "bpf"))]
use aya_metrics_common::{Counter, Meter, BPF_COUNTERS_MAX_ENTRIES};

// Module with implementations depending on the `aya-bpf` module.
// The `bpf` module only compiles with the `bpf` feature enabled.
#[cfg(target_arch = "bpf")]
mod bpf {
    use super::*;
    use aya_ebpf::macros::map;
    use aya_ebpf::maps::PerCpuArray;

    // A BPF map to store counter metrics
    #[map(name = "COUNTERS")]
    pub static mut COUNTERS: PerCpuArray<u64> =
        PerCpuArray::<u64>::with_max_entries(BPF_COUNTERS_MAX_ENTRIES as u32, 0);
}

// Include everything from the `bpf` module.
// The `bpf` module itself only exists as a namespace which is hidden behind the `bpf` feature.
#[cfg(target_arch = "bpf")]
use bpf::*;

/// Increments a counter.
///
/// Counters represent a single monotonic value, which means the value can only be incremented, not decremented, and
/// always starts out with an initial value of zero.
///
/// # Arguments
///
/// * `counter` - An identifier for a counter metric. It is used as an index into the underlying BPF map.
/// * `value`   - The amount by which the counter should be incremented.
///
#[cfg(any(test, target_arch = "bpf"))]
#[inline(always)]
pub fn counter<T: Counter>(counter: T, value: u64) {
    // SAFETY: Instances of PerCpuArray are thread local in eBPF. We can therefore be sure that concurrent
    // accesses will not happen on other threads and, within this function, counter is the sole reference to COUNTERS.
    // It is not leaked from this function, so concurrent &mut references cannot be introduced by calling this function multiple times.
    if let Some(counter) = unsafe { COUNTERS.get_ptr_mut(Meter::index(&counter)) } {
        unsafe { *counter += value };
    }
}

// Module containing mocks for the `bpf` module.
// The `bpf` module only compiles with the `bpf` feature enabled. It contains dependencies from `aya-bpf` which either
// do not compile or work correctly from user space. Defining mocks allows testing implementations that use `aya-bpf`.
// Ideally these would live in the `elastic-flow-collector-ebpf` crate if it was possible to add tests there.
#[cfg(test)]
mod bpf_mocks {
    use std::cell::Cell;

    use super::BPF_COUNTERS_MAX_ENTRIES;

    pub struct PerCpuArray<T, const N: usize> {
        pub data: Cell<[T; N]>,
        _t: core::marker::PhantomData<T>,
    }

    impl<const N: usize> PerCpuArray<u64, N> {
        pub const fn new() -> PerCpuArray<u64, N> {
            PerCpuArray {
                data: Cell::new([0u64; N]),
                _t: core::marker::PhantomData,
            }
        }

        pub fn get_ptr_mut(&mut self, index: u32) -> Option<*mut u64> {
            let data = self.data.get_mut();
            let ptr = data as *mut u64;
            let ptr_at = unsafe { ptr.offset(index.try_into().unwrap()) };
            Some(ptr_at)
        }
    }

    pub static mut COUNTERS: PerCpuArray<u64, BPF_COUNTERS_MAX_ENTRIES> =
        PerCpuArray::<u64, BPF_COUNTERS_MAX_ENTRIES>::new();
}

// Include everything from the `bpf_mocks` module for tests.
#[cfg(test)]
use bpf_mocks::*;

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    enum MockCounter {
        Test1,
        Test2,
    }

    impl Counter for MockCounter {
        fn name(self) -> String {
            match self {
                MockCounter::Test1 => "test1".to_string(),
                MockCounter::Test2 => "test2".to_string(),
            }
        }

        fn index(&self) -> u32 {
            match self {
                MockCounter::Test1 => 0,
                MockCounter::Test2 => BPF_COUNTERS_MAX_ENTRIES as u32 - 1,
            }
        }
    }

    #[test]
    fn test_counter() {
        let mut expected = [0u64; BPF_COUNTERS_MAX_ENTRIES];

        let actual = unsafe { COUNTERS.data.get() };
        assert_eq!(actual, expected);

        // test adding some numbers
        counter(MockCounter::Test1, 1);
        counter(MockCounter::Test2, 42);
        let actual = unsafe { COUNTERS.data.get() };
        *expected.first_mut().unwrap() = 1;
        *expected.last_mut().unwrap() = 42;
        assert_eq!(actual, expected);

        // test adding zero
        counter(MockCounter::Test1, 0);
        counter(MockCounter::Test2, 0);
        let actual = unsafe { COUNTERS.data.get() };
        assert_eq!(actual, expected);

        // test adding again increments existing values
        counter(MockCounter::Test1, 1);
        counter(MockCounter::Test2, 1);
        let actual = unsafe { COUNTERS.data.get() };
        *expected.first_mut().unwrap() = 2;
        *expected.last_mut().unwrap() = 43;
        assert_eq!(actual, expected);
    }
}
