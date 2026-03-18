fn main() { let lib = libloading::Library::new("onnxruntime.dll"); match lib { Ok(_) => println!("Loaded!"), Err(e) => println!("Error: {}", e) } }
