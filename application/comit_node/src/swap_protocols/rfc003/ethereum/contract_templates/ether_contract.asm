{
    // Placeholder for deployment timestamp
    0x50000005

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

    timestamp

    // Subtract current timestamp from deployment timestamp (pushed to stack as first instruction of this contract)
    sub

    // Placeholder for relative expiry time
    0x20000002

    // Compare relative expiry timestamp with result of subtraction
    lt

    // Jump to refund if time is expired
    refund
    jumpi

    // Don't do anything if we get here (e.g. secret didn't match and time didn't expire)
    return(0, 0)

redeem:
    selfdestruct(0x3000000000000000000000000000000000000003) 

refund:
    selfdestruct(0x4000000000000000000000000000000000000004)
}
