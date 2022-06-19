use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    // just use Release cef-binary because Debug makes crt problems for windows
    // istringstream would crash on destructor
    let profile = "Release";
    // if cfg!(debug_assertions) {
    //     "Debug"
    // } else {
    //     "Release"
    // };

    #[cfg(target_os = "windows")]
    {
        // this must be linked first!
        // or else we get debug assertion popups about heap corruption/crt memory
        // also you can't build debug cef without linking this
        if profile == "Debug" {
            // links to ucrtbased.dll
            println!("cargo:rustc-link-lib=static=ucrtd");
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=cef_interface/cef_binary/CMakeLists.txt");
    println!("cargo:rerun-if-changed=cef_interface/CMakeLists.txt");
    println!("cargo:rerun-if-changed=cef_interface/interface.hh");
    println!("cargo:rerun-if-changed=cef_interface/interface.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.hh");
    println!("cargo:rerun-if-changed=cef_interface/client.cc");
    println!("cargo:rerun-if-changed=cef_interface/client.hh");
    println!("cargo:rerun-if-changed=cef_interface/serialize.cc");
    println!("cargo:rerun-if-changed=cef_interface/serialize.hh");
    println!("cargo:rerun-if-changed=cef_interface/cef_exe.cc");

    let cmake_path = cmake::Config::new("cef_interface")
        .static_crt(true)
        .build_target("cef_interface")
        .profile(profile)
        .define("USE_SANDBOX", "OFF")
        .define("PROJECT_ARCH", env::var("CARGO_CFG_TARGET_ARCH").unwrap())
        .build();

    // link to cef_interface
    link(
        "cef_interface",
        &cmake_path.join("build/"),
        LinkKind::Static,
        profile,
    );

    // link to libcef_dll_wrapper
    link(
        "cef_dll_wrapper",
        &cmake_path.join("build/libcef_dll_wrapper"),
        LinkKind::Static,
        profile,
    );

    // link to libcef
    link(
        "cef",
        format!("cef_interface/cef_binary/{}", profile),
        LinkKind::Dynamic,
        profile,
    );

    #[cfg(target_os = "linux")]
    {
        // fixes undefined reference to c++ methods
        println!("cargo:rustc-link-lib=static=stdc++");
    }

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
        .header_contents(
            "bindgen.hpp",
            "#include \"interface.hh\"",
        )
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_function("cef_interface_.*")
        .allowlist_type("RustSchemeReturn")
        .rustified_enum("FFIRustV8Value_Tag")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = Path::new(&out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // build cef_exe
    let cmake_path = cmake::Config::new("cef_interface")
        .static_crt(true)
        .build_target("cef_exe")
        .profile(profile)
        .define("USE_SANDBOX", "OFF")
        .define("PROJECT_ARCH", env::var("CARGO_CFG_TARGET_ARCH").unwrap())
        .build();

    #[cfg(target_os = "windows")]
    const CEF_EXE_NAME: &str = "cef.exe";

    #[cfg(target_os = "windows")]
    const CEF_EXE_OLD_NAME: &str = "cef_exe.exe";

    #[cfg(not(target_os = "windows"))]
    const CEF_EXE_NAME: &str = "cef";

    #[cfg(not(target_os = "windows"))]
    const CEF_EXE_OLD_NAME: &str = "cef_exe";

    let _ignore = fs::remove_dir_all(Path::new(&out_dir).join(CEF_EXE_NAME));
    fs::copy(
        cmake_path
            .join("build")
            .join(profile)
            .join(CEF_EXE_OLD_NAME),
        Path::new(&out_dir).join(CEF_EXE_NAME),
    )
    .unwrap();
}

enum LinkKind {
    Static,
    Dynamic,
}
fn link<P: Into<PathBuf>>(name: &str, search_path: P, kind: LinkKind, profile: &str) {
    let search_path = search_path.into();
    let kind = match kind {
        LinkKind::Static => "static",
        LinkKind::Dynamic => "dylib",
    };

    #[cfg(target_os = "windows")]
    let search_path = if search_path.join(profile).is_dir() {
        search_path.join(profile)
    } else {
        search_path
    };

    #[cfg(target_os = "windows")]
    let name = if search_path.join(format!("lib{}.lib", name)).is_file() {
        format!("lib{}", name)
    } else {
        name.to_string()
    };

    println!("cargo:rustc-link-search=native={}", search_path.display());
    println!("cargo:rustc-link-lib={}={}", kind, name);
}
