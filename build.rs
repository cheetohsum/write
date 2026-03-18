fn main() {
    #[cfg(windows)]
    {
        let icon_path = "assets/icon.ico";
        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new(icon_path).exists() {
            println!("cargo:warning=Embedding icon from {}", icon_path);
            res.set_icon(icon_path);
        } else {
            panic!("Icon file not found at {}. The exe will have no icon.", icon_path);
        }
        res.set("ProductName", "Write");
        res.set("FileDescription", "Write - Distraction-free terminal editor");
        res.compile().expect("Failed to compile Windows resources");
    }
}
