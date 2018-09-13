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

	// Jump to success if hashes match
	success
	jumpi

    timestamp

    // Substract current timestamp from deployment timestamp (pushed to stack as first instruction of this contract)
    sub

    // Placeholder for relative expiry time
    0x20000002

    // Compare relative expiry timestamp with result of substraction
	lt

	// Jump to refund if time is expired
	refund
	jumpi

    // Don't do anything if we get here (e.g. secret didn't match and time didn't expire)
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
    0x3000000000000000000000000000000000000003 // success address
    0x4000000000000000000000000000000000000004 // refund address
    0x6000000000000000000000000000000000000000000000000000000000000006 // amount
    0x7000000000000000000000000000000000000007 //token contract address

*/
success:
    mstore(32,0x3000000000000000000000000000000000000003) // success address
	finishTransferTokens	
	jump

refund:
    mstore(32, 0x4000000000000000000000000000000000000004) // refund address
	finishTransferTokens	
	jump

finishTransferTokens:
    mstore(0, 0xa9059cbb) // first 4bytes of keccak256("transfer(address,uint256)")
    mstore(64, 0x5000000000000000000000000000000000000000000000000000000000000005) // Amount
	call(
	  sub(gas,100000), 
	  0x6000000000000000000000000000000000000006,
	  00, // Ether to transfer
	  28, // = 32-4
	  68, // = 2*32+4
	  96, // return location
	  32  // return size
	) // Token Contract address

	selfdestruct(mload(32)) 
}
