//! This crate contains mocks for the aya crate.
//!
//! These are very crude implementations that mimic the signatures of many Aya structs/functions.
//! They can be used as drop in replacements.
//!
//! This approach is very dirty and should be used sparingly. Better mocks would be welcome.
//! Hopefully in the future this may be something offered by Aya: https://github.com/aya-rs/aya/issues/999

// GRCOV_STOP_COVERAGE

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use std::borrow::{Borrow, Cow};

use aya::maps::lpm_trie::Key;
use aya::{
    maps::{MapError, PerCpuValues},
    programs::Program,
    util::nr_cpus,
    Btf, EbpfError, Pod, VerifierLogLevel,
};

pub struct Map {}

pub struct EbpfLoader<'a> {
    btf: Option<Cow<'a, Btf>>,
    verifier_log_level: VerifierLogLevel,
}

impl<'a> EbpfLoader<'a> {
    pub fn new() -> Self {
        Self {
            btf: Btf::from_sys_fs().ok().map(Cow::Owned),
            verifier_log_level: VerifierLogLevel::default(),
        }
    }

    pub fn load(&mut self, data: &[u8]) -> Result<Ebpf, EbpfError> {
        Ebpf::load(&[])
    }

    pub fn btf(&mut self, btf: Option<&'a Btf>) -> &mut Self {
        self.btf = btf.map(Cow::Borrowed);
        self
    }

    pub fn verifier_log_level(&mut self, level: VerifierLogLevel) -> &mut Self {
        self.verifier_log_level = level;
        self
    }

    pub fn set_max_entries(&mut self, name: &'a str, size: u32) -> &mut Self {
        self
    }
}

pub struct Ebpf {}

impl Ebpf {
    pub fn load(_data: &[u8]) -> Result<Ebpf, EbpfError> {
        Ok(Self {})
    }

    pub fn program_mut(&mut self, _name: &str) -> Option<&mut Program> {
        None
    }

    pub fn map_mut(&mut self, _name: &str) -> Option<&mut aya::maps::Map> {
        None
    }

    pub fn take_map(&mut self, _name: &str) -> Option<Map> {
        Some(Map {})
    }
}

pub struct EbpfLogger;

impl EbpfLogger {
    pub fn init(_bpf: &mut Ebpf) -> Result<EbpfLogger, aya_log::Error> {
        Ok(Self {})
    }
}

#[derive(Clone)]
pub struct PerCpuArray<V: Pod> {
    inner: Arc<Mutex<Vec<Vec<V>>>>,
    _v: std::marker::PhantomData<V>,
}

impl<V: Pod> PerCpuArray<V> {
    pub fn new(len: usize, val: V) -> Self {
        let arr = vec![val.clone(); nr_cpus().unwrap()];
        PerCpuArray {
            inner: Arc::new(Mutex::new(vec![arr.clone(); len])),
            _v: core::marker::PhantomData,
        }
    }
}

impl TryFrom<Map> for PerCpuArray<u64> {
    type Error = MapError;

    fn try_from(_map: Map) -> Result<PerCpuArray<u64>, MapError> {
        Ok(PerCpuArray::new(1, 0u64))
    }
}

impl<V: Pod> PerCpuArray<V> {
    pub fn get(&self, index: &u32, _flags: u64) -> Result<PerCpuValues<V>, MapError> {
        let guard = self.inner.lock().unwrap();
        let values = guard.get(*index as usize).ok_or(MapError::OutOfBounds {
            index: *index,
            max_entries: guard.len() as u32,
        })?;
        PerCpuValues::try_from(values.clone()).map_err(|_| MapError::KeyNotFound)
    }

    pub fn set(&mut self, index: u32, values: PerCpuValues<V>, _flags: u64) -> Result<(), MapError> {
        let arr = (0..nr_cpus().unwrap())
            .map(|i| values.get(i).unwrap().clone())
            .collect::<Vec<V>>();
        let mut guard = self.inner.lock().unwrap();
        guard[index as usize] = arr;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LpmTrie<K: Eq + Hash, V: Eq + Copy> {
    // shameless plug, doesn't actually use a trie or suport LPM
    inner: HashMap<K, V>,
    _k: std::marker::PhantomData<K>,
    _v: std::marker::PhantomData<V>,
}

impl<K: Eq + Hash + Pod, V: Eq + Copy + Pod> LpmTrie<K, V> {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            _k: std::marker::PhantomData,
            _v: std::marker::PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }
}

impl TryFrom<Map> for LpmTrie<u32, u32> {
    type Error = MapError;

    fn try_from(_map: Map) -> Result<LpmTrie<u32, u32>, MapError> {
        Ok(LpmTrie::new())
    }
}

impl<K: Eq + Hash + Pod, V: Eq + Copy + Pod> LpmTrie<K, V> {
    /// Inserts a key value pair into the map.
    pub fn insert(&mut self, key: &Key<K>, value: impl Borrow<V>, _flags: u64) -> Result<(), MapError> {
        // shameless plug, ignores the prefix require for LPM
        self.inner.insert(key.data(), *value.borrow());
        Ok(())
    }
}
