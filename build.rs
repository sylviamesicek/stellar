use wesl::Wesl;

fn main() {
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::composite".parse().unwrap(), "composite");
    Wesl::new("src/renderer/shaders")
        .build_artifact(&"package::sierpinski".parse().unwrap(), "sierpinski");
}
