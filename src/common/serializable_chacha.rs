use rand::{RngCore, SeedableRng};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::{mem, ptr, slice};

#[derive(Debug)]
pub struct SerializableChaCha20 {
    rng: rand_chacha::ChaCha20Rng,
}
#[derive(Serialize, Deserialize)]
struct SerializableChaCha20Wrapper(Vec<u8>);

impl Serialize for SerializableChaCha20 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut innards = SerializableChaCha20Wrapper(vec![]);
        innards.0.extend_from_slice(unsafe { slice::from_raw_parts(&self.rng as *const _ as *const u8, mem::size_of_val(&self.rng)) });
        innards.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableChaCha20 {
    fn deserialize<D>(deserializer: D) -> Result<SerializableChaCha20, D::Error> where D: Deserializer<'de> {
        let innards = SerializableChaCha20Wrapper::deserialize(deserializer)?;
        unsafe {
            let mut ret: SerializableChaCha20 = mem::zeroed();
            let mut out: *mut u8 = &mut ret as *mut _ as *mut u8;
            for b in innards.0.into_iter() {
                ptr::write(out, b);
                out = out.offset(1);
            }
            Ok(ret)
        }
    }
}

impl RngCore for SerializableChaCha20 {
    fn next_u32(&mut self) -> u32 { self.rng.next_u32() }
    fn next_u64(&mut self) -> u64 { self.rng.next_u64() }
    fn fill_bytes(&mut self, data: &mut [u8]) { self.rng.fill_bytes(data) }
    fn try_fill_bytes(&mut self, data: &mut [u8]) -> Result<(), rand::Error> { self.rng.try_fill_bytes(data) }
}

impl SeedableRng for SerializableChaCha20 {
    type Seed = <rand_chacha::ChaCha20Rng as SeedableRng>::Seed;
    fn from_seed(seed: Self::Seed) -> Self { SerializableChaCha20 { rng: rand_chacha::ChaCha20Rng::from_seed(seed) } }
}
