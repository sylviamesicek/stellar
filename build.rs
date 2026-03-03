use wesl::Wesl;

fn main() {
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::fullscreen".parse().unwrap(), "fullscreen");
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::composite".parse().unwrap(), "composite");
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::fractal".parse().unwrap(), "fractal");
}
