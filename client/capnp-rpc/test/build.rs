fn main() {
    ::capnpc::CompilerCommand::new()
        .file("test.capnp")
        .run()
        .unwrap();
}
