use p256::elliptic_curve::ecdh::{diffie_hellman, SharedSecret};
use p256::{NistP256, PublicKey, SecretKey};
use rand_core::{CryptoRng, Error, RngCore};

pub struct KeyManager {
    random: [u8;16],
    private_key: SecretKey,
    peer_public_key: Option<PublicKey>,
    shared_secret: Option<SharedSecret<NistP256>>,
}

impl KeyManager {
    pub fn new<R>(rng: R) -> Self
    where
        R: RngCore,
    {
        let mut wrapper = RngWrapper(rng);
        let mut random = [0;16];
        wrapper.fill_bytes(&mut random);
        let secret = SecretKey::random(&mut wrapper);
        Self {
            random,
            private_key: secret,
            peer_public_key: None,
            shared_secret: None,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        self.private_key.public_key()
    }

    pub fn add_peer_public_key(&mut self, pk: PublicKey) {
        self.shared_secret
            .replace(diffie_hellman(&self.private_key.to_nonzero_scalar(), pk.as_affine()));
        self.peer_public_key.replace(pk);
    }
}

struct RngWrapper<R: RngCore>(R);

impl<R: RngCore> RngCore for RngWrapper<R> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.0.try_fill_bytes(dest)
    }
}

impl<R: RngCore> CryptoRng for RngWrapper<R> {}

impl<R: RngCore> CryptoRng for &RngWrapper<R> {}
