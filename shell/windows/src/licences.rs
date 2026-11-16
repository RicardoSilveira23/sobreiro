//! The licence and notice texts the exe carries: those of the libraries it is built from, Sobreiro's
//! own, and the two fonts'. A later licence window reads them from here.

/// The notices of the libraries the exe is built from, made by the licence test.
pub const NOTICES: &str = include_str!("../notices.txt");
/// Sobreiro's own licence.
pub const LICENCE: &str = include_str!("../../../LICENSE");
/// The licence of the Inter font.
pub const INTER: &str = include_str!("../../../src/Sobreiro/Fonts/OFL-Inter.txt");
/// The licence of the Unbounded font.
pub const UNBOUNDED: &str = include_str!("../../../src/Sobreiro/Fonts/OFL-Unbounded.txt");

/// Each text with its name.
pub fn all() -> [(&'static str, &'static str); 4] {
    [
        ("Third-party notices", NOTICES),
        ("Sobreiro licence", LICENCE),
        ("Inter licence", INTER),
        ("Unbounded licence", UNBOUNDED),
    ]
}

/// Nothing reads the texts yet, and the linker drops what nothing reads. The licence window will
/// read them; until then this keeps them in the exe.
pub fn keep() {
    std::hint::black_box(all());
}
