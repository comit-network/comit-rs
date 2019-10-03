# Secp256k1 Omni Context

A context management crate for [rust-secp256k1](https://github.com/rust-bitcoin/rust-secp256k1) crate where the context must be initialised by the user once and then kept in memory for easy access.

As the context is omnipresent, it's called _Omni Context_. Hate the (naming) game, not the player.
