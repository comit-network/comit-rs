{
    mstore(0, 0x23b872dd) // Transfer function identifier
    mstore(32, 0x3000000000000000000000000000000000000000000000000000000000000003) // From
    mstore(64, 0x4000000000000000000000000000000000000000000000000000000000000004) // To
    mstore(96, 0x5000000000000000000000000000000000000000000000000000000000000005) // Amount
    mstore(128, 1)

    // TODO: Change gas
    call(4000000, 0x6000000000000000000000000000000000000006, 0, 28, 100, 128, 32) // Token Contract address
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
