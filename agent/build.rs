fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf definitions for gRPC server
    // Write to OUT_DIR so tonic::include_proto! can find them
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &["../proto/sentinelguard.proto"],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/sentinelguard.proto");

    Ok(())
}
