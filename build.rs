use wesl::Wesl;

fn main() {
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::fullscreen".parse().unwrap(), "fullscreen");
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::composite".parse().unwrap(), "composite");
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::fractal".parse().unwrap(), "fractal");
    Wesl::new("src/renderer/shaders").build_artifact(&"package::star".parse().unwrap(), "star");
    Wesl::new("src/renderer/shaders").build_artifact(&"package::bloom".parse().unwrap(), "bloom");
    // Wesl::new("src/renderer/shaders")
    //     .build_artifact(&"package::raymarching::naive".parse().unwrap(), "naive");
}
