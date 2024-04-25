use std::{
    env,
    path::{Path, PathBuf},
};

// just use Release cef-binary because Debug makes crt problems for windows
// istringstream would crash on destructor
const PROFILE: &str = if false && cfg!(debug_assertions) {
    "Debug"
} else {
    "Release"
};

fn main() {
    let (libcef_lib_dir, libcef_include_dir) = build_libcef();
    let libcef_dll_wrapper_lib_dir = build_libcef_dll_wrapper();
    build_cef_interface(&libcef_include_dir);
    build_cef_exe(
        &libcef_lib_dir,
        &libcef_include_dir,
        &libcef_dll_wrapper_lib_dir,
    );

    // must link in reverse-order

    println!("cargo:rustc-link-lib=static=cef_interface");

    link(
        "cef_dll_wrapper",
        &libcef_dll_wrapper_lib_dir,
        LinkKind::Static,
    );

    // TODO why ignore mac??
    // #[cfg(not(target_os = "macos"))]
    link("cef", &libcef_lib_dir, LinkKind::Dynamic);

    #[cfg(target_os = "windows")]
    {
        // this must be linked first!
        // or else we get debug assertion popups about heap corruption/crt memory
        // also you can't build debug cef without linking this
        if PROFILE == "Debug" {
            // links to ucrtbased.dll
            println!("cargo:rustc-link-lib=static=ucrtd");
        }
    }

    #[cfg(target_os = "linux")]
    {
        // fixes undefined reference to c++ methods
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    build_bindings(&libcef_include_dir);
}

fn build_libcef() -> (PathBuf, PathBuf) {
    println!("cargo:rerun-if-env-changed=LIBCEF_LIB_DIR");
    let libcef_lib_dir = if let Ok(p) = env::var("LIBCEF_LIB_DIR") {
        PathBuf::from(p)
    } else {
        PathBuf::from("cef_interface/cef_binary").join(PROFILE)
    };
    assert!(
        libcef_lib_dir.is_dir(),
        "libcef_lib_dir {:?} is_dir",
        libcef_lib_dir
    );

    println!("cargo:rerun-if-env-changed=LIBCEF_INCLUDE_DIR");
    let mut libcef_include_dir = if let Ok(p) = env::var("LIBCEF_INCLUDE_DIR") {
        PathBuf::from(p)
    } else {
        PathBuf::from("cef_interface/cef_binary/include")
    };
    assert!(
        libcef_include_dir.is_dir(),
        "libcef_include_dir {:?} is_dir",
        libcef_include_dir
    );
    assert_eq!(
        libcef_include_dir.file_name().expect("file_name"),
        "include",
        "LIBCEF_INCLUDE_DIR directory needs to be named 'include' because of strange cef #include's"
    );
    assert!(libcef_include_dir.pop());

    (libcef_lib_dir, libcef_include_dir)
}

fn build_libcef_dll_wrapper() -> PathBuf {
    println!("cargo:rerun-if-env-changed=LIBCEF_DLL_WRAPPER_LIB_DIR");

    let libcef_dll_wrapper_lib_dir = if let Ok(p) = env::var("LIBCEF_DLL_WRAPPER_LIB_DIR") {
        PathBuf::from(p)
    } else {
        println!("cargo:rerun-if-changed=cef_interface/cef_binary/CMakeLists.txt");

        cmake::Config::new("cef_interface/cef_binary")
            .static_crt(true)
            .build_target("libcef_dll_wrapper")
            .profile(PROFILE)
            .define("USE_SANDBOX", "OFF")
            .define("PROJECT_ARCH", env::var("CARGO_CFG_TARGET_ARCH").unwrap())
            .build()
            .join("build")
            .join("libcef_dll_wrapper")
    };
    assert!(libcef_dll_wrapper_lib_dir.is_dir());

    libcef_dll_wrapper_lib_dir
}

fn build_cef_interface(libcef_include_dir: &Path) {
    println!("cargo:rerun-if-changed=cef_interface/app.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.hh");
    println!("cargo:rerun-if-changed=cef_interface/client.cc");
    println!("cargo:rerun-if-changed=cef_interface/client.hh");
    println!("cargo:rerun-if-changed=cef_interface/interface.cc");
    println!("cargo:rerun-if-changed=cef_interface/interface.hh");
    println!("cargo:rerun-if-changed=cef_interface/serialize.cc");
    println!("cargo:rerun-if-changed=cef_interface/serialize.hh");

    cc::Build::new()
        .cargo_warnings(!cfg!(debug_assertions))
        .include(libcef_include_dir)
        .file("cef_interface/app.cc")
        .file("cef_interface/client.cc")
        .file("cef_interface/interface.cc")
        .file("cef_interface/serialize.cc")
        .compile("cef_interface");
}

fn build_cef_exe(
    libcef_lib_dir: &Path,
    libcef_include_dir: &Path,
    libcef_dll_wrapper_lib_dir: &Path,
) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=cef_interface/cef_exe.cc");

    #[cfg(target_os = "windows")]
    const CEF_EXE_NAME: &str = "cef.exe";

    #[cfg(not(target_os = "windows"))]
    const CEF_EXE_NAME: &str = "cef";

    let mut cmd = cc::Build::new()
        .warnings(false)
        .cpp(true)
        .include(libcef_include_dir)
        .get_compiler()
        .to_command();
    cmd.arg("cef_interface/app.cc");
    cmd.arg("cef_interface/client.cc");
    cmd.arg("cef_interface/interface.cc");
    cmd.arg("cef_interface/serialize.cc");
    cmd.arg("cef_interface/cef_exe.cc");
    cmd.arg(format!("-L{}", libcef_dll_wrapper_lib_dir.display()));
    cmd.arg("-lcef_dll_wrapper");
    cmd.arg(format!("-L{}", libcef_lib_dir.display()));
    cmd.arg("-lcef");
    cmd.arg(format!("-o{}", out_dir.join(CEF_EXE_NAME).display()));
    assert!(cmd.status().unwrap().success());
}

fn build_bindings(libcef_include_dir: &Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .derive_copy(false)
        .clang_arg("-Icef_interface")
        .clang_arg(format!("-I{}", libcef_include_dir.display()))
        .clang_arg("-xc++")
        // The input header we would like to generate
        // bindings for.
        .header_contents(
            "bindgen.hpp",
            "#include \"interface.hh\"",
        )
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("cef_interface_.*")
        .allowlist_type("RustSchemeReturn")
        .rustified_enum("FFIRustV8ValueTag")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

enum LinkKind {
    Static,
    Dynamic,
}

// TODO still needed? check windows
fn link(name: &str, search_path: &Path, kind: LinkKind) {
    let kind = match kind {
        LinkKind::Static => "static",
        LinkKind::Dynamic => "dylib",
    };

    #[cfg(target_os = "windows")]
    let search_path = if search_path.join(PROFILE).is_dir() {
        search_path.join(PROFILE)
    } else {
        search_path
    };

    #[cfg(target_os = "windows")]
    let name = if search_path.join(format!("lib{}.lib", name)).is_file() {
        format!("lib{}", name)
    } else {
        name.to_string()
    };

    assert!(
        search_path.is_dir(),
        "search_path {:?} is_dir",
        search_path.display()
    );
    let lib_path = search_path.join(format!(
        "lib{}.{}",
        name,
        match kind {
            "static" => "a",
            "dylib" => "so",
            _ => unreachable!(),
        }
    ));
    assert!(
        lib_path.is_file(),
        "lib_path {:?} is_file",
        lib_path.display()
    );
    println!("cargo:rustc-link-search=native={}", search_path.display());
    println!("cargo:rustc-link-lib={}={}", kind, name);
}
