{
    caller
    pop
    mstore(0, 0xa9059cbb) // Transfer function identifier
    mstore(32, 0x3000000000000000000000000000000000000000000000000000000000000003) // HTLC Contract address
    mstore(64, 0x4000000000000000000000000000000000000000000000000000000000000004) // Amount
    mstore(96, 1)

    // TODO: Change gas
    call(100000, 0x5000000000000000000000000000000000000005, 0, 28, 68, 96, 32) // Token Contract address
    // mload(96)
    // and
    success
    jumpi
    revert(0,0)


success:
    mstore(0, timestamp)
    mstore8(27, 0x63)
    codecopy(32, add(0x1001, 5), 0x2002)
    return(0, 0)
}
