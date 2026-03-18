fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }
        res.set("ProductName", "Write");
        res.set("FileDescription", "Write - Distraction-free terminal editor");
        res.compile().expect("Failed to compile Windows resources — icon may not be embedded");
    }
}
