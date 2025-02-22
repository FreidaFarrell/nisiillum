use {
    crate::config::RandomnessOptions,
    anyhow::{
        ensure,
        Result,
    },
    sha3::{
        Digest,
        Keccak256,
    },
    std::error::Error,
};

/// A HashChain.
pub struct PebbleHashChain {
    hash: Vec<[u8; 32]>,
    next: usize,
}

impl PebbleHashChain {
    // Given a secret, we hash it with Keccak256 len times to get the final hash, this is an S/KEY
    // like protocol in which revealing the hashes in reverse proves knowledge.
    pub fn new(secret: [u8; 32], length: usize) -> Self {
        let mut hash = Vec::<[u8; 32]>::with_capacity(length);
        hash.push(Keccak256::digest(secret).into());
        for _ in 1..length {
            hash.push(Keccak256::digest(&hash[hash.len() - 1]).into());
        }

        hash.reverse();

        Self { hash, next: 0 }
    }

    // TODO: possibly take the chain id here to ensure different hash chains on every blockchain
    pub fn from_config(opts: &RandomnessOptions, random: [u8; 32]) -> Result<Self, Box<dyn Error>> {
        let mut secret: [u8; 32] = [0u8; 32];
        secret.copy_from_slice(&hex::decode(opts.secret.clone())?[0..32]);
        let secret: [u8; 32] = Keccak256::digest([random, secret].flatten()).into();

        Ok(Self::new(secret, opts.chain_length.try_into()?))
    }

    /// Reveal the next hash in the chain using the previous proof.
    pub fn reveal(&mut self) -> Result<[u8; 32]> {
        ensure!(self.next < self.len(), "no more hashes in the chain");
        let next = self.hash[self.next].clone();
        self.next += 1;
        Ok(next)
    }

    pub fn reveal_ith(&self, i: usize) -> Result<[u8; 32]> {
        ensure!(i < self.len(), "index not in range");
        Ok(self.hash[i].clone())
    }

    pub fn len(&self) -> usize {
        self.hash.len()
    }
}

/// `HashChainState` tracks the mapping between on-chain sequence numbers to hash chains.
/// This struct is required to handle the case where the provider rotates their commitment,
/// which requires tracking multiple hash chains here.
pub struct HashChainState {
    // The sequence number where the hash chain starts. Must be stored in sorted order.
    pub offsets:     Vec<usize>,
    pub hash_chains: Vec<PebbleHashChain>,
}

impl HashChainState {
    pub fn reveal(&self, sequence_number: u64) -> Result<[u8; 32]> {
        let sequence_number: usize = sequence_number.try_into()?;
        let chain_index = self.offsets.partition_point(|x| x <= &sequence_number) - 1;
        self.hash_chains[chain_index].reveal_ith(sequence_number - self.offsets[chain_index])
    }
}
