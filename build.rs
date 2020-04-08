use std::{env, fs, path::Path};

fn main() {
    let profile = if cfg!(debug_assertions) {
        "Debug"
    } else {
        "Release"
    };

    let out_dir = env::var("OUT_DIR").unwrap();

    println!(
        "cargo:rustc-link-search=native=cef_interface/cef_binary/{}",
        profile
    );
    println!("cargo:rustc-link-lib=comctl32");
    println!("cargo:rustc-link-lib=shlwapi");
    println!("cargo:rustc-link-lib=rpcrt4");
    println!("cargo:rustc-link-lib=libcef");
    println!("cargo:rustc-link-lib=cef_sandbox");
    if profile == "Debug" {
        println!("cargo:rustc-link-lib=ucrtd");
    }

    let cmake_path = cmake::Config::new("cef_interface")
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
    println!("cargo:rustc-link-lib=libcef_dll_wrapper");

    // link to cef_interface
    println!(
        "cargo:rustc-link-search=native={}",
        cmake_path.join("build/").join(profile).display()
    );
    println!("cargo:rustc-link-lib=cef_interface");

    fs::copy(
        cmake_path
            .join("build/cef_binary/tests/cefsimple")
            .join(profile)
            .join("cefsimple.exe"),
        Path::new(&out_dir).join("cef.exe"),
    )
    .unwrap();
}
