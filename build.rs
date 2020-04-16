use std::{env, fs, path::Path};

fn main() {
    let profile = if cfg!(debug_assertions) {
        "Debug"
    } else {
        "Release"
    };

    // this must be linked first!
    // or else we get debug assertion popups about heap corruption/crt memory
    // also you can't build debug cef without linking this
    if profile == "Debug" {
        // links to ucrtbased.dll
        println!("cargo:rustc-link-lib=static=ucrtd");
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    println!(
        "cargo:rustc-link-search=native=cef_interface/cef_binary/{}",
        profile
    );

    println!("cargo:rustc-link-lib=dylib=libcef");

    println!("cargo:rerun-if-changed=cef_interface/CMakeLists.txt");
    println!("cargo:rerun-if-changed=cef_interface/interface.hh");
    println!("cargo:rerun-if-changed=cef_interface/interface.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.hh");
    println!("cargo:rerun-if-changed=cef_interface/client.cc");
    println!("cargo:rerun-if-changed=cef_interface/client.hh");

    let cmake_path = cmake::Config::new("cef_interface")
        .static_crt(true)
        .build_target("cef_interface")
        .profile(profile)
        .build();

    // link to libcef_dll_wrapper
    println!(
        "cargo:rustc-link-search=native={}",
        cmake_path
            .join("build/libcef_dll_wrapper")
            .join(profile)
            .display()
    );
    println!("cargo:rustc-link-lib=static=libcef_dll_wrapper");

    // link to cef_interface
    println!(
        "cargo:rustc-link-search=native={}",
        cmake_path.join("build/").join(profile).display()
    );
    println!("cargo:rustc-link-lib=static=cef_interface");

    fs::copy(
        cmake_path
            .join("build/cef_binary/tests/cefsimple")
            .join(profile)
            .join("cefsimple.exe"),
        Path::new(&out_dir).join("cefsimple.exe"),
    )
    .unwrap();

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .derive_copy(false)
        .clang_arg("-Icef_interface")
        .clang_arg("-Icef_interface/cef_binary")
        .clang_arg("-xc++")
        // The input header we would like to generate
        // bindings for.
        .header_contents("bindgen.hpp", r#"
            #include "interface.hh"
        "#)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .whitelist_function("cef_interface_.*")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = Path::new(&out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
