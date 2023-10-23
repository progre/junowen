use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        WindowsResource::new().compile().unwrap();
    }
}
