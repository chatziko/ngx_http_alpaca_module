// extern crate cmake;
extern crate cc;

fn main(){
    cc::Build::new()
                .file("src/libmap/map.c")
                .include("src")
                .compile("libmap.a");

    // let dst = Config::new("libmap").build();

    // println!("cargo:rustc-link-search=native={}",dst.display());
    // println!("cargo:rustc-link-lib=static=map");
}
