fn main() {
    // Rebuild if memory file changes.
    println!("cargo:rerun-if-changed=memory.x");
}
