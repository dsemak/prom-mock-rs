//! Build script for compiling Protocol Buffers definitions.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/remote.proto"], &["proto/"])?;
    Ok(())
}
