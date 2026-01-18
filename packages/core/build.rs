fn main() {
    // Only run napi_build when building the NAPI feature (for Node bindings)
    #[cfg(feature = "napi")]
    napi_build::setup();
}
