fn main() {

  let mut build = fortranc::Build::new();
  build
    .file("src/foo.f90")
    .flag_if_supported("-Wall")
    .flag_if_supported("-cpp")
    .flag_if_supported("-fpp")
    .flag_if_supported("-Wfoo-bar-this-flag-does-not-exist")
    .define("FOO", None)
    .define("BAR", "1")
    .compile("foo");

  fortranc::Build::new()
    .file("src/bar1.f90")
    .file("src/bar2.f90")
    .include("src")
    .compile("bar");

}