# Secp256k1 Wrapper

A context management crate for [rust-secp256k1](https://github.com/rust-bitcoin/rust-secp256k1).
 
The secp context must be initialised by the user when creating a new `SecretKey`.
The context is kept inside the secret key to facilitate cryptographic actions such as signing or deriving a public key.
