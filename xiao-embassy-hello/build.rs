fn main() {
    // Ensure rebuilds if memory.x changes. cortex-m-rt's link.x will INCLUDE memory.x.
    println!("cargo:rerun-if-changed=memory.x");
}
