#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SoftFork {
    id: String,
    version: u32,
    reject: Reject,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Reject {
    status: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Bip9SoftFork {
    csv: Bip9SoftForkDetails,
    segwit: Bip9SoftForkDetails,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Bip9SoftForkDetails {
    status: String,
    bit: Option<u32>,
    #[serde(rename = "startTime")]
    start_time: i64, // In regtest, startTime is -1
    timeout: u64,
    since: u64,
    // TODO: implement before new BIP9
    /*
    "statistics": {         (object) numeric statistics about BIP9 signalling for a softfork (only for "started" status)
    "period": xx,        (numeric) the length in blocks of the BIP9 signalling period
    "threshold": xx,     (numeric) the number of blocks with the version bit set required to activate the feature
    "elapsed": xx,       (numeric) the number of blocks elapsed since the beginning of the current period
    "count": xx,         (numeric) the number of blocks with the version bit set in the current period
    "possible": xx       (boolean) returns false if there are not enough blocks left in this period to pass activation threshold
    */
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Blockchain {
    chain: String,
    blocks: u64,
    headers: u64,
    bestblockhash: String,
    difficulty: f64,
    mediantime: u64,
    verificationprogress: f64,
    initialblockdownload: bool,
    chainwork: String,
    size_on_disk: u64,
    pruned: bool,
    pruneheight: Option<u64>,
    automatic_pruning: Option<bool>,
    prune_target_size: Option<u64>,
    softforks: Vec<SoftFork>,
    bip9_softforks: Bip9SoftFork,
    warnings: String,
}
