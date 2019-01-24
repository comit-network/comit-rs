{
    // Load received secret size
    calldatasize

    // Check if secret is zero length
    iszero

    // If secret is zero length, jump to branch that checks if expiry time has been reached
    check_expiry
    jumpi

    // Load expected secret size
    32

    // Load received secret size
    calldatasize

    // Compare secret size
    eq
    iszero

    // If passed secret is wrong size, jump to exit contract
    exit
    jumpi

    // Load secret into memory
    calldatacopy(0, 0, 32)

    // Hash secret with SHA-256 (pre-compiled contract 0x02)
    call(72, 0x02, 0, 0, 32, 33, 32)

    // Placeholder for correct secret hash
    0x1000000000000000000000000000000000000000000000000000000000000001

    // Load hashed secret from memory
    mload(33)

    // Compare hashed secret with existing one
    eq

    // Combine `eq` result with `call` result
    and

    // Jump to redeem if hashes match
    redeem
    jumpi

    // Exit if hashes don't match
    return(0, 0)

check_expiry:
    // Timestamp of the current block in seconds since the epoch
    timestamp

    // Placeholder for refund timestamp 
    0x20000002

    // Compare refund timestamp with current timestamp
    lt

    // Jump to refund if time is expired
    refund
    jumpi

exit:
    // Exit
    return(0, 0)

/*
    memory  layout
    0 secret
    32 hash return value
    ->
    0 transfer pointer
    32 to
    64 amount
    96 transfer return

    place holders
    0x3000000000000000000000000000000000000003 // redeem address
    0x4000000000000000000000000000000000000004 // refund address
    0x6000000000000000000000000000000000000000000000000000000000000006 // amount
    0x7000000000000000000000000000000000000007 //token contract address

*/
redeem:
    log1(0, 32, 0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413) // log keccak256(Redeemed())
    mstore(32,0x3000000000000000000000000000000000000003) // redeem address
    finishTransferTokens
    jump

refund:
    log1(0, 0, 0x5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178) // log keccak256(Refunded())
    mstore(32, 0x4000000000000000000000000000000000000004) // refund address
    finishTransferTokens
    jump

finishTransferTokens:
    mstore(0, 0xa9059cbb) // first 4bytes of keccak256("transfer(address,uint256)")
    mstore(64, 0x5000000000000000000000000000000000000000000000000000000000000005) // Amount
    call(
      sub(gas,100000), 
      0x6000000000000000000000000000000000000006,
      0,  // Ether to transfer
      28, // = 32-4
      68, // = 2*32+4
      96, // return location
      32  // return size
    ) // Token Contract address
    pop

    selfdestruct(mload(32))
}
