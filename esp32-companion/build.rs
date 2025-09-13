// Build script for ESP32 companion
// Only needed for ESP32-S2 target builds

fn main() {
    #[cfg(target_os = "espidf")]
    {
        // embuild::espidf::sysenv::output();
        println!("cargo:rerun-if-changed=build.rs");
    }
    
    #[cfg(not(target_os = "espidf"))]
    {
        // Nothing to do for standard targets
        println!("cargo:rerun-if-changed=build.rs");
    }
}