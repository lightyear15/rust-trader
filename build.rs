fn main() {
    println!("cargo:rerun-if-changed=./ta_lib");
    let commons = std::fs::read_dir("./ta_lib/src/ta_common/").expect("error reading ta_common");
    let srcs = std::fs::read_dir("./ta_lib/src/ta_func").expect("error in reading ta_func");
    let src_files_iter = commons.chain(srcs).filter_map(|res_dir| {
        let dir_entry = res_dir.expect("error reading src_files");
        if dir_entry.file_type().expect("in reading file type").is_dir() {
            return None;
        }
        let path = dir_entry.path();
        let extension = path.extension().expect("missing extension");
        if extension != "c" {
            return None;
        }
        Some(path)
    });
    cc::Build::new()
        .files(src_files_iter)
        .include("./ta_lib/include")
        .include("./ta_lib/src/ta_common")
        .compile("ta_lib")
}
