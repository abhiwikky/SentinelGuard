fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rustc-link-lib=FltLib");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/sentinelguard.proto"],
            &["proto"],
        )?;
    Ok(())
}

