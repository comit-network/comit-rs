{
    mstore(0, timestamp)
    mstore8(27, 0x63)
    codecopy(32, add(0x1001, 5), 0x2002)
    return(27, 0x2002)
}
