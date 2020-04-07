use rand::{RngCore, SeedableRng};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::{mem, ptr, slice};

#[derive(Clone, Debug)]
pub struct SerializableChaCha20 {
    seed: <rand_chacha::ChaCha20Rng as SeedableRng>::Seed,
    rng: rand_chacha::ChaCha20Rng,
}
#[derive(Serialize, Deserialize)]
struct SerializableChaCha20Wrapper(<rand_chacha::ChaCha20Rng as SeedableRng>::Seed, u128);

impl Serialize for SerializableChaCha20 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        /*let mut innards = vec![];
        innards.extend_from_slice(unsafe { slice::from_raw_parts(&self.rng as *const _ as *const u8, mem::size_of_val(&self.rng)) });
        SerializableChaCha20Wrapper(&innards).serialize(serializer)*/
        SerializableChaCha20Wrapper(self.seed.clone(), self.rng.get_word_pos()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableChaCha20 {
    fn deserialize<D>(deserializer: D) -> Result<SerializableChaCha20, D::Error> where D: Deserializer<'de> {
        /*let innards = SerializableChaCha20Wrapper::deserialize(deserializer)?;
        let n = mem::size_of::<rand_chacha::ChaCha20Rng>();
        if innards.0.len() != n {
            let expected: String = format!("mem::size_of::<rand_chacha::ChaCha20Rng>() ({})", n);
            return Err(serde::de::Error::invalid_length(innards.0.len(), &expected.as_str()));
        }
        unsafe {
            let mut ret: SerializableChaCha20 = SerializableChaCha20 { rng: mem::zeroed() };
            let mut out: *mut u8 = &mut ret.rng as *mut _ as *mut u8;
            for b in innards.0.iter() {
                ptr::write(out, *b);
                out = out.offset(1);
            }
            Ok(ret)
        }*/
        let innards = SerializableChaCha20Wrapper::deserialize(deserializer)?;
        let mut ret = SerializableChaCha20::from_seed(innards.0);
        ret.rng.set_word_pos(innards.1);
        Ok(ret)
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
    fn from_seed(seed: Self::Seed) -> Self { SerializableChaCha20 { seed: seed.clone(), rng: rand_chacha::ChaCha20Rng::from_seed(seed) } }
}

#[test]
fn test_chacha_size() {
    type T = rand_chacha::ChaCha20Rng;
    let tmp = T::seed_from_u64(0xdeadbeefdeadbeef);
    println!("{:?} {:?}", mem::size_of_val(&tmp), mem::size_of::<T>());
}
