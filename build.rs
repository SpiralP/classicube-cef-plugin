use std::{
    env,
    path::{Path, PathBuf},
};

const PROFILE: &str = if cfg!(debug_assertions) {
    "Debug"
} else {
    "Release"
};

fn main() {
    let mut links = Vec::new();

    // fixes undefined reference to c++ methods;
    // required here because libcef/libcef_dll_wrapper depend on it,
    // also note we do cargo_metadata(false) on cc::Build
    #[cfg(target_os = "linux")]
    links.push(Link::new(LinkKind::Dynamic, "stdc++".to_string(), None));

    #[cfg(target_os = "macos")]
    links.push(Link::new(LinkKind::Dynamic, "c++".to_string(), None));

    let libcef_include_dir = build_libcef(&mut links);
    build_libcef_dll_wrapper(&mut links);
    build_cef_interface(&libcef_include_dir, &mut links);

    links.reverse();
    for link in links {
        link.print();
    }

    build_bindings(&libcef_include_dir);
}

#[allow(unused_variables)]
fn build_libcef(links: &mut Vec<Link>) -> PathBuf {
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

    // Fixes linux/windows not being able to `cargo test` because STATUS_DLL_NOT_FOUND;
    // Putting the lib files in OUT_DIR will let `cargo test` link them at runtime.
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#dynamic-library-paths
    // Only doing this for dev test builds so that nothing strange happens for release builds.
    #[cfg(all(debug_assertions, not(target_os = "macos")))]
    let libcef_lib_dir = {
        use std::fs;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let new_libcef_lib_dir = out_dir.join("libcef");

        fs::create_dir_all(&new_libcef_lib_dir).unwrap();
        for entry in fs::read_dir(libcef_lib_dir.canonicalize().unwrap()).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            let file_name = entry.file_name();
            let file_name = file_name.to_str().unwrap();
            if file_type.is_file()
                && (file_name.ends_with(".so")
                    || file_name.ends_with(".dll")
                    || file_name.ends_with(".lib"))
            {
                let from = entry.path();
                let to = new_libcef_lib_dir.join(file_name);
                let _ = fs::remove_file(&to).ok();

                #[cfg(target_os = "windows")]
                fs::copy(&from, &to).unwrap();

                #[cfg(target_os = "linux")]
                std::os::unix::fs::symlink(&from, &to).unwrap();
            }
        }

        new_libcef_lib_dir
    };

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

    // mac calls cef_load_library to load libcef at runtime, so never link libcef
    #[cfg(not(target_os = "macos"))]
    links.push(Link::new(
        LinkKind::Dynamic,
        "cef".to_string(),
        Some(libcef_lib_dir.to_path_buf()),
    ));

    // help ./cef/cef and ./plugins/plugin.so find libcef.so
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN/cef_binary");
        println!("cargo:rustc-cdylib-link-arg=-Wl,-rpath,$ORIGIN/../cef/cef_binary");
    }

    libcef_include_dir
}

fn build_libcef_dll_wrapper(links: &mut Vec<Link>) {
    println!("cargo:rerun-if-env-changed=LIBCEF_DLL_WRAPPER_LIB_DIR");

    let libcef_dll_wrapper_lib_dir = if let Ok(p) = env::var("LIBCEF_DLL_WRAPPER_LIB_DIR") {
        PathBuf::from(p)
    } else {
        println!("cargo:rerun-if-changed=cef_interface/cef_binary/CMakeLists.txt");

        let mut build = cmake::Config::new("cef_interface/cef_binary");

        // rust builds with /MT (static C-RunTime), but libcef_dll_wrapper uses /MTd,
        // and windows does NOT like this! So remove the "d" in a big hack
        #[cfg(target_os = "windows")]
        let build = if PROFILE == "Debug" {
            build
                .configure_arg("-DCEF_RUNTIME_LIBRARY_FLAG=/DSUPER_HACK_IGNORE_DEBUG_D_HERE_")
                .configure_arg("-DCEF_COMPILER_FLAGS=/MT")
        } else {
            &mut build
        };

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let build = build.build_arg("-j16");

        build
            .static_crt(true) // /MT (uses /MTd when profile is Debug)
            .build_target("libcef_dll_wrapper")
            .profile(PROFILE)
            .define("USE_SANDBOX", "OFF")
            .define(
                // see cef_binary/cmake/cef_variables.cmake
                "PROJECT_ARCH",
                match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
                    "x86" => "x86",
                    "x86_64" => "x86_64",
                    "arm" => "arm",
                    "aarch64" => "arm64",
                    other => panic!("unsupported target arch {:?}", other),
                },
            )
            .build()
            .join("build")
            .join("libcef_dll_wrapper")
    };
    assert!(
        libcef_dll_wrapper_lib_dir.is_dir(),
        "libcef_dll_wrapper_lib_dir {:?} is_dir",
        libcef_dll_wrapper_lib_dir
    );

    links.push(Link::new(
        LinkKind::Static,
        "cef_dll_wrapper".to_string(),
        Some(libcef_dll_wrapper_lib_dir.to_path_buf()),
    ));
}

fn build_cef_interface(libcef_include_dir: &Path, links: &mut Vec<Link>) {
    println!("cargo:rerun-if-changed=cef_interface/app.cc");
    println!("cargo:rerun-if-changed=cef_interface/app.hh");
    println!("cargo:rerun-if-changed=cef_interface/client.cc");
    println!("cargo:rerun-if-changed=cef_interface/client.hh");
    println!("cargo:rerun-if-changed=cef_interface/interface.cc");
    println!("cargo:rerun-if-changed=cef_interface/interface.hh");
    println!("cargo:rerun-if-changed=cef_interface/serialize.cc");
    println!("cargo:rerun-if-changed=cef_interface/serialize.hh");

    let mut build = cc::Build::new();
    let build = build
        .cpp(true)
        .std("c++17")
        .warnings(true)
        .warnings_into_errors(true)
        .static_crt(true) // only ever uses /MT, never /MTd
        .cargo_metadata(false)
        .include(libcef_include_dir)
        .file("cef_interface/app.cc")
        .file("cef_interface/client.cc")
        .file("cef_interface/interface.cc")
        .file("cef_interface/serialize.cc");

    #[cfg(not(target_os = "windows"))]
    let build = build.flag("-Wno-unused-parameter");

    build.compile("cef_interface");

    links.push(Link::new(
        LinkKind::Static,
        "cef_interface".to_string(),
        Some(PathBuf::from(env::var("OUT_DIR").unwrap())),
    ));
}

fn build_bindings(libcef_include_dir: &Path) {
    bindgen::Builder::default()
        .derive_copy(false)
        .clang_arg("-Icef_interface")
        .clang_arg(format!("-I{}", libcef_include_dir.display()))
        .clang_arg("-xc++")
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
        .generate()
        .expect("Unable to generate bindings") 
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[allow(dead_code)]
pub enum LinkKind {
    Static,
    Dynamic,
    Framework,
}

pub struct Link {
    kind: LinkKind,
    name: String,
    search_path: Option<PathBuf>,
}

impl Link {
    pub fn new(kind: LinkKind, name: String, search_path: Option<PathBuf>) -> Self {
        let (name, search_path) = if let Some(search_path) = search_path {
            // cmake puts things in Release/Debug folders (on windows only)
            #[cfg(target_os = "windows")]
            let search_path = if search_path.join(PROFILE).is_dir() {
                search_path.join(PROFILE)
            } else {
                search_path.to_path_buf()
            };

            assert!(
                search_path.is_dir(),
                "search_path {:?} is_dir",
                search_path.display()
            );

            #[cfg(target_os = "windows")]
            let name = {
                // they are sometimes named with lib in front: libcef.lib, libcef_dll_wrapper.lib
                let name = if search_path.join(format!("lib{}.lib", name)).is_file() {
                    format!("lib{}", name)
                } else {
                    name
                };

                // only .lib needs to exist on windows for both kinds
                let lib_path = search_path.join(format!("{}.lib", name));
                assert!(
                    lib_path.is_file(),
                    "lib_path {:?} is_file",
                    lib_path.display()
                );

                name
            };

            #[cfg(target_os = "linux")]
            {
                let lib_path = search_path.join(format!(
                    "lib{}.{}",
                    name,
                    match kind {
                        LinkKind::Static => "a",
                        LinkKind::Dynamic => "so",
                        _ => panic!("can't use framework on linux"),
                    }
                ));
                assert!(
                    lib_path.is_file(),
                    "lib_path {:?} is_file",
                    lib_path.display()
                );
            }

            (name, Some(search_path))
        } else {
            (name, None)
        };

        Self {
            kind,
            name,
            search_path,
        }
    }

    pub fn print(&self) {
        if let Some(search_path) = &self.search_path {
            let kind = match self.kind {
                LinkKind::Static => "native",
                LinkKind::Dynamic => "native",
                LinkKind::Framework => "framework",
            };
            println!("cargo:rustc-link-search={}={}", kind, search_path.display());
        }

        let kind = match self.kind {
            LinkKind::Static => "static",
            LinkKind::Dynamic => "dylib",
            LinkKind::Framework => "framework",
        };
        println!("cargo:rustc-link-lib={}={}", kind, self.name);
    }
}
