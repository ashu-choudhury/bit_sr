fn main() {
    println!("cargo:rerun-if-changed=c_src/nvdaController_s.c");
    println!("cargo:rerun-if-changed=c_src/nvdaController.h");

    let mut build = cc::Build::new();
    build.file("c_src/nvdaController_s.c");
    build.include("c_src");
    build.define("WIN32_LEAN_AND_MEAN", None);
    build.define("_M_AMD64", None);
    build.flag_if_supported("-fms-extensions");
    build.flag_if_supported("-fpermissive");
    build.flag_if_supported("-Wno-incompatible-pointer-types");
    build.warnings(false);
    build.compile("nvda_controller_stub");

    println!("cargo:rustc-link-lib=rpcrt4");
}
