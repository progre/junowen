use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        static_vcruntime::metabuild();
        WindowsResource::new().compile().unwrap();
    }
}
