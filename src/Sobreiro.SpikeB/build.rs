fn main() {
    // The fonts and pictures go into the exe: nothing is read from the source folder when it runs.
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", config).expect("the .slint files compile");
}
