extern crate tower_grpc_build;

fn main() {
    // Build lnd
    tower_grpc_build::Config::new()
        .enable_client(true)
        .build(&["proto/lnd.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}
