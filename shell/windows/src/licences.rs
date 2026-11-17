//! The licence and notice texts the exe carries: those of the libraries it is built from, Sobreiro's
//! own, and the two fonts'. The licence window (`about.rs`) reads them from here.

/// The notices of the libraries the exe is built from, made by the licence test.
pub static NOTICES: &str = include_str!("../notices.txt");
/// Sobreiro's own licence.
pub static LICENCE: &str = include_str!("../../../LICENSE");
/// The licence of the Inter font.
pub static INTER: &str = include_str!("../fonts/OFL-Inter.txt");
/// The licence of the Unbounded font.
pub static UNBOUNDED: &str = include_str!("../fonts/OFL-Unbounded.txt");

/// Each text with its name.
pub fn all() -> [(&'static str, &'static str); 4] {
    [
        ("Third-party notices", NOTICES),
        ("Sobreiro licence", LICENCE),
        ("Inter licence", INTER),
        ("Unbounded licence", UNBOUNDED),
    ]
}

/// Keeps the texts in the exe, which the linker would drop if nothing read them. The licence
/// window reads them now, so this and its call in `main` can go.
pub fn keep() {
    std::hint::black_box(all());
}
