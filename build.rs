use std::io::Result;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/proto/media.proto");
    
    tonic_build::compile_protos("src/proto/media.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));
} 