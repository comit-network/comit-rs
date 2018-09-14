{
    mstore(0, 0x23b872dd) // first 4bytes of keccak256("transferFrom(address,address,uint256)")
    mstore(32, 0x4000000000000000000000000000000000000004) // From (refund address)
    mstore(64, 0x3000000000000000000000000000000000000003) // To (htlc contract address)
    mstore(96, 0x5000000000000000000000000000000000000000000000000000000000000005) // Amount

    // TODO: Change gas for contract deployment
    call(sub(gas,1000000), 0x6000000000000000000000000000000000000006, 0, 28, 100, 0, 0) // Token Contract address
    deploy
    jumpi
    revert(0,0)

deploy:
    mstore(0, timestamp)
    mstore8(27, 0x63)
    codecopy(32, add(0x1001, 5), 0x2002)
    return(27, 0x2002)
}
