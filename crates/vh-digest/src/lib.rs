//! Zero-dependency safe-Rust SHA-256 (FIPS 180-4) for content-addressed
//! evidence (post-audit C3).
//!
//! Claim boundary: this is a LOCAL implementation validated against the
//! standard known-answer vectors below. Until the verifier track
//! independently exercises those vectors from its own surface (V1), any
//! consumer must describe output as "cryptographically content-addressed
//! with an unreviewed local implementation" — never "tamper-proof" or
//! "authenticated provenance". Signatures and external trust anchors are
//! out of scope.
//!
//! Determinism: pure computation over caller-supplied bytes. No I/O, no
//! clock, no environment, no randomness, no global state, no unsafe.

/// Algorithm tag carried next to every digest this crate emits.
pub const ALGORITHM: &str = "sha256";

/// Schema tag for the digest surface itself (bump on any observable
/// change to the encoding of digests, never silently).
pub const DIGEST_SCHEMA: &str = "vh-sha256-v1";

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Incremental SHA-256 hasher.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    /// Bytes buffered toward the next 64-byte block.
    buf: [u8; 64],
    buf_len: usize,
    /// Total message length in bytes (the padding trailer needs bits).
    total_len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: H0,
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_len.wrapping_mul(8);
        // Padding: 0x80, zeros to 56 mod 64, then the 64-bit big-endian
        // bit length.
        self.update(&[0x80]);
        while self.buf_len != 56 {
            self.update(&[0x00]);
        }
        // The trailer bytes must not count toward the message length, but
        // update() already added them; the captured bit_len is correct
        // because it was taken before padding.
        self.update(&bit_len.to_be_bytes());
        debug_assert_eq!(self.buf_len, 0);
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// One-shot digest.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize()
}

/// One-shot lowercase-hex digest — the rendering every receipt uses.
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&sha256(data))
}

/// Lowercase hex of arbitrary bytes (no allocation tricks, no formatter
/// state — byte-stable everywhere).
pub fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // FIPS 180-4 / NIST CAVP known-answer vectors. These exact literals
    // are the review anchor: V1 independently re-checks them from the
    // verifier surface without copying this implementation.
    #[test]
    fn kat_empty() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn kat_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn kat_two_block_448_bit() {
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn kat_million_a() {
        let mut h = Sha256::new();
        let chunk = [b'a'; 1000];
        for _ in 0..1000 {
            h.update(&chunk);
        }
        assert_eq!(
            hex(&h.finalize()),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// Incremental hashing must be split-invariant: any partition of the
    /// message produces the one-shot digest (padding-boundary lengths
    /// 55, 56, 63, 64, 65 are the classic off-by-one traps).
    #[test]
    fn incremental_equals_one_shot_across_padding_boundaries() {
        for len in [0usize, 1, 54, 55, 56, 57, 63, 64, 65, 127, 128, 129, 1000] {
            let msg: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let expect = sha256_hex(&msg);
            // Byte-at-a-time.
            let mut h = Sha256::new();
            for &b in &msg {
                h.update(&[b]);
            }
            assert_eq!(hex(&h.finalize()), expect, "byte-at-a-time len {len}");
            // Split at every position.
            for split in 0..=len {
                let mut h = Sha256::new();
                h.update(&msg[..split]);
                h.update(&msg[split..]);
                assert_eq!(hex(&h.finalize()), expect, "split {split} len {len}");
            }
        }
    }

    /// Adversarial: a single flipped bit anywhere in a message must
    /// change the digest (spot check, not a security proof).
    #[test]
    fn bit_flip_changes_digest() {
        let msg = b"the exact bytes of an evidence bundle".to_vec();
        let base = sha256_hex(&msg);
        for i in 0..msg.len() {
            let mut tampered = msg.clone();
            tampered[i] ^= 1;
            assert_ne!(sha256_hex(&tampered), base, "flip at byte {i}");
        }
    }

    #[test]
    fn hex_is_lowercase_and_stable() {
        assert_eq!(hex(&[0x00, 0xff, 0x1a]), "00ff1a");
        assert_eq!(ALGORITHM, "sha256");
        assert_eq!(DIGEST_SCHEMA, "vh-sha256-v1");
    }
}
