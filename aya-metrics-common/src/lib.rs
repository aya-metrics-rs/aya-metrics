// #![no_std] except for tests and user space!
#![cfg_attr(not(any(test, feature = "user")), no_std)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]

//! Provides common metric functionality for use in both user space and in BPF.
//!

/// The maximum number of counters that can be inserted the BPF per CPU array.
/// Setting it to 64 is arbitrary and this should be generalized before releasing [aya-metrics-ebpf].
pub const BPF_COUNTERS_MAX_ENTRIES: usize = 64;

/// The kind of [`Meter`].
///
/// Currently only [`MeterKind::Counter`] is supported but other meters could be added.
pub enum MeterKind {
    /// Counters monitor monotonically increasing values. Counters may never be reset to a lesser value.
    Counter,
}

impl MeterKind {
    /// The name of the BPF map for this kind of [`Meter`].
    pub fn map_name(&self) -> &str {
        match self {
            MeterKind::Counter => "COUNTERS",
        }
    }
}

/// Seal traits with a supertrait.
/// A public supertrait whose name is not publicly exported.
mod private {
    pub trait Sealed {}
}

/// A base trait for all kinds of meters.
///
/// It is expected to be used on an enum representing a set of meters of the same kind in the same BPF map.
pub trait Meter: Copy + private::Sealed {
    /// The kind of meter.
    fn kind() -> MeterKind;

    /// The index of the meter in a BPF map.
    fn index(&self) -> u32;

    /// The name of the meter.
    #[cfg(any(test, feature = "user"))]
    fn name(self) -> String;

    /// The description of the meter.
    #[cfg(any(test, feature = "user"))]
    fn description(self) -> String;
}

/// A trait which should be implemented over an enumeration defining counters in the same BPF map.
///
/// Counters monitor monotonically increasing values and never reset to a lesser value.
/// Each enumeration should represents an index into a BPF map.
pub trait Counter: Meter {
    /// The index of the counter in a BPF map.
    fn index(&self) -> u32;

    /// The name of the counter.
    #[cfg(any(test, feature = "user"))]
    fn name(self) -> String;

    /// The description of the meter.
    ///
    /// Implementing this is optional and by default will return an empty string.
    #[cfg(any(test, feature = "user"))]
    fn description(self) -> String {
        String::new()
    }
}

impl<T: Counter> private::Sealed for T {}

/// Blanket implementation for all [`Counter`]s.
impl<T: Counter> Meter for T {
    fn kind() -> MeterKind {
        MeterKind::Counter
    }

    fn index(&self) -> u32 {
        Counter::index(self)
    }

    #[cfg(any(test, feature = "user"))]
    fn name(self) -> String {
        Counter::name(self)
    }

    #[cfg(any(test, feature = "user"))]
    fn description(self) -> String {
        Counter::description(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Meter, MeterKind};

    // Create a test enum implementing Counter
    #[derive(Debug, Copy, Clone)]
    enum MockCounter {
        First,
        Second,
    }

    impl super::Counter for MockCounter {
        fn name(self) -> String {
            match self {
                MockCounter::First => "first_counter".to_string(),
                MockCounter::Second => "second_counter".to_string(),
            }
        }

        fn index(&self) -> u32 {
            match self {
                MockCounter::First => 0,
                MockCounter::Second => 1,
            }
        }
    }

    #[test]
    fn test_counter() {
        let first = MockCounter::First;
        let second = MockCounter::Second;

        // Test names
        assert_eq!(first.name(), "first_counter");
        assert_eq!(second.name(), "second_counter");

        // Test index values
        assert_eq!(first.index(), 0);
        assert_eq!(second.index(), 1);
    }

    #[test]
    fn test_meter_trait() {
        let counter = MockCounter::First;

        // Test that kind returns Counter
        assert!(matches!(MockCounter::kind(), MeterKind::Counter));

        // Test that index matches through Meter trait
        assert_eq!(<MockCounter as Meter>::index(&counter), 0);

        // Test that name matches through Meter trait
        assert_eq!(<MockCounter as Meter>::name(counter), "first_counter");
    }

    #[test]
    fn test_meter_kind() {
        assert_eq!(MeterKind::Counter.map_name(), "COUNTERS");
    }
}
