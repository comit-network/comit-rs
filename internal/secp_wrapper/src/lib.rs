//! A context management crate for [rust-secp256k1](https://github.com/rust-bitcoin/rust-secp256k1).
//!
//! The secp context must be initialised by the user when creating a new
//! `SecretKey`. The context is kept inside the secret key to facilitate
//! cryptographic actions such as signing or deriving a public key.
//!
//! # How to use
//!
//! Use the builder to generate a secret key from a random source, then sign a
//! message with the provided method.
//!
//! ```
//! use secp_wrapper::{
//!     secp256k1::{self, rand, Message, Secp256k1},
//!     Builder,
//! };
//!
//! let mut rng = rand::OsRng::new().unwrap();
//! let secp: Secp256k1<secp256k1::All> = Secp256k1::new();
//!
//! let secret_key = Builder::new(secp).rng(&mut rng).build().unwrap();
//!
//! let message_to_sign = Message::from_slice(&b"I said that.").unwrap();
//! let signature = secret_key.sign(message_to_sign);
//! ```

#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use crate::secret_key::*;
pub use secp256k1::{self, PublicKey};

mod secret_key;
