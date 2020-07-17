#![no_std]

fn main() {
    capnpc::CompilerCommand::new()
        .file("src/hello.capnp")
        .run()
        .expect("compiling schema");
}
