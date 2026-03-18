fn main() {
    #[cfg(windows)]
    {
        // Use CARGO_MANIFEST_DIR for an absolute path — relative paths fail
        // in CI where the build script's working directory may differ
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let icon_path = format!("{}/assets/icon.ico", manifest_dir);

        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new(&icon_path).exists() {
            println!("cargo:warning=Embedding icon from {}", icon_path);
            res.set_icon(&icon_path);
        } else {
            panic!(
                "Icon file not found at {}. The exe will have no icon.",
                icon_path
            );
        }
        res.set("ProductName", "Write");
        res.set("FileDescription", "Write - Distraction-free terminal editor");
        res.compile().expect("Failed to compile Windows resources");
    }
}
