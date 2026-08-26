fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
    embed_resource::compile("icon.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
