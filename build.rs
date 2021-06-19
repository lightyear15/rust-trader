
fn main() {
    //println!("cargo:rerun-if-changed=./ta_lib");
    let commons = std::fs::read_dir("./ta_lib/src/ta_common/").expect("error reading ta_common");
    //let srcs = std::fs::read_dir("./ta_lib/src/ta_func").expect("error in reading ta_func");
    cc::Build::new()
        .files(commons.map(|res_dir| res_dir.expect("error reading path in common").path()))
        //.files(srcs.map(|res_dir| res_dir.expect("error reading path in srcs").path()))
        .include("./ta_lib/include")
        .include("./ta_lib/src/ta_common")
        .compile("ta_lib")
}
