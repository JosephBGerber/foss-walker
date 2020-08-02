use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Put the linker script somewhere the linker can find it
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");

    // for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("images")).unwrap() {
    //     let entry = entry.unwrap();
    //
    //     let image = image::open(entry.path()).unwrap();
    //     let (width, height) = image.dimensions();
    //
    //     let mut image_buffer: Vec<u8> = Vec::with_capacity(((width * height) / 8) as usize);
    //     let mut byte: u8 = 0;
    //
    //     for (x, _y, pixel) in image.pixels() {
    //         if pixel[0] > 127 {
    //             byte |= (1 << (7 - (x % 8))) as u8;
    //         }
    //
    //         if x % 8 == 7 {
    //             image_buffer.push(byte);
    //             byte = 0;
    //         }
    //     }
    //
    //     assert_eq!(image_buffer.len(), ((width * height) / 8) as usize);
    //
    //     let file_name = entry.file_name().into_string().unwrap();
    //     let output_path = Path::new(&env::var("OUT_DIR").unwrap()).join(&file_name[..file_name.len() - 4]);
    //     fs::write(output_path, image_buffer).unwrap();
    // }
}
