//! The exe's icon and version details, as a Win32 resource file the linker puts in the exe.
//!
//! The icon is 1.1's, read from the C# project until that leaves, as the fonts are. The version
//! details carry 1.1's names with the version from `Cargo.toml`. No tool is needed: a `.res` file
//! is a list of entries, each a header and some bytes, and both MSVC linkers take one as input.
use std::path::{Path, PathBuf};

type Failure = Box<dyn std::error::Error>;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;

/// Language neutral, and Unicode: the string table's name, as 1.1's is.
const TABLE: &str = "000004b0";
const LANGUAGE_NEUTRAL: u16 = 0x0000;
const CODE_PAGE_UNICODE: u16 = 0x04B0;

const COMPANY: &str = "Sobreiro";
const COPYRIGHT: &str = "Copyright 2026 Ricardo Silveira";
const ORIGINAL_FILENAME: &str = "Sobreiro.exe";

/// Writes the resource file into `out_dir` and tells the linker to take it, for the exe only.
pub fn link_exe_resources(out_dir: &Path) -> Result<(), Failure> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=build/resources.rs");
    // The version comes from the workspace's manifest.
    println!("cargo::rerun-if-changed=../../Cargo.toml");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return Ok(());
    }

    let icon = Path::new(&std::env::var("CARGO_MANIFEST_DIR")?)
        .join("../../src/Sobreiro/Assets/sobreiro.ico");
    println!("cargo::rerun-if-changed={}", icon.display());
    let icon =
        std::fs::read(&icon).map_err(|error| format!("the icon is not readable: {error}"))?;

    let mut res = Vec::new();
    entry(&mut res, 0, 0, 0, &[])?;
    add_icon(&mut res, &icon)?;
    entry(&mut res, RT_VERSION, 1, 0, &version_info()?)?;

    let path: PathBuf = out_dir.join("exe.res");
    std::fs::write(&path, res)?;
    println!("cargo::rustc-link-arg-bins={}", path.display());
    Ok(())
}

/// Splits the `.ico` into its images and writes each as an `RT_ICON`, and the group that lists
/// them as the one `RT_GROUP_ICON` Windows reads the exe's icon from.
fn add_icon(res: &mut Vec<u8>, ico: &[u8]) -> Result<(), Failure> {
    // ICONDIR: reserved, type (1 is an icon), count. Then 16 bytes for each image.
    if ico.len() < 6 || word(ico, 0)? != 0 || word(ico, 2)? != 1 {
        return Err("the icon is not an .ico file".into());
    }
    let count = usize::from(word(ico, 4)?);
    if count == 0 {
        return Err("the icon holds no images".into());
    }

    let mut group = Vec::new();
    push_u16(&mut group, 0);
    push_u16(&mut group, 1);
    push_u16(&mut group, u16::try_from(count)?);
    for index in 0..count {
        let at = 6 + 16 * index;
        let head = ico
            .get(at..at + 16)
            .ok_or("the icon's list of images is cut short")?;
        let size = usize::try_from(dword(head, 8)?)?;
        let start = usize::try_from(dword(head, 12)?)?;
        let image = ico
            .get(start..start.checked_add(size).ok_or("the icon is malformed")?)
            .ok_or("an image of the icon is cut short")?;
        let id = u16::try_from(index + 1)?;
        entry(res, RT_ICON, id, 0, image)?;

        // GRPICONDIRENTRY is the file's entry with the image's id in place of its offset.
        group.extend_from_slice(&head[..12]);
        push_u16(&mut group, id);
    }
    entry(res, RT_GROUP_ICON, 1, 0, &group)
}

/// One entry of a `.res` file: a header of 32 bytes (type and name as ordinals), the data, and
/// padding to the next 4 bytes.
fn entry(
    res: &mut Vec<u8>,
    kind: u16,
    name: u16,
    language: u16,
    data: &[u8],
) -> Result<(), Failure> {
    push_u32(res, u32::try_from(data.len())?);
    push_u32(res, 32);
    for ordinal in [kind, name] {
        push_u16(res, 0xFFFF);
        push_u16(res, ordinal);
    }
    push_u32(res, 0); // data version
    // MOVEABLE and PURE; icons are DISCARDABLE too, as `rc` writes them. The leading empty
    // entry has no flags, so its 32 bytes are the ones every `.res` file starts with.
    push_u16(
        res,
        match kind {
            0 => 0,
            RT_ICON | RT_GROUP_ICON => 0x1030,
            _ => 0x0030,
        },
    );
    push_u16(res, language);
    push_u32(res, 0); // version
    push_u32(res, 0); // characteristics
    res.extend_from_slice(data);
    align(res);
    Ok(())
}

/// The `VS_VERSIONINFO` block: the fixed numbers, the strings Explorer shows under Details, and
/// the language they are in.
fn version_info() -> Result<Vec<u8>, Failure> {
    let number = |name: &str| -> Result<u16, Failure> { Ok(std::env::var(name)?.parse::<u16>()?) };
    let major = number("CARGO_PKG_VERSION_MAJOR")?;
    let minor = number("CARGO_PKG_VERSION_MINOR")?;
    let patch = number("CARGO_PKG_VERSION_PATCH")?;
    let prerelease = !std::env::var("CARGO_PKG_VERSION_PRE")?.is_empty();
    let product = std::env::var("CARGO_PKG_VERSION")?;

    // VS_FIXEDFILEINFO: the file's and the product's version as two DWORDs each, the fourth
    // part 0.
    let mut fixed = Vec::new();
    push_u32(&mut fixed, 0xFEEF_04BD); // signature
    push_u32(&mut fixed, 0x0001_0000); // structure version
    for _ in 0..2 {
        push_u32(&mut fixed, (u32::from(major) << 16) | u32::from(minor));
        push_u32(&mut fixed, u32::from(patch) << 16);
    }
    push_u32(&mut fixed, 0x3F); // flags that mean something
    push_u32(&mut fixed, if prerelease { 0x2 } else { 0 }); // VS_FF_PRERELEASE
    push_u32(&mut fixed, 0x0004_0004); // VOS_NT_WINDOWS32
    push_u32(&mut fixed, 1); // VFT_APP
    push_u32(&mut fixed, 0); // subtype
    push_u32(&mut fixed, 0); // date
    push_u32(&mut fixed, 0);

    let file_version = format!("{major}.{minor}.{patch}.0");
    let strings = [
        ("CompanyName", COMPANY),
        ("FileDescription", "Sobreiro"),
        ("FileVersion", file_version.as_str()),
        ("InternalName", "Sobreiro"),
        ("LegalCopyright", COPYRIGHT),
        ("OriginalFilename", ORIGINAL_FILENAME),
        ("ProductName", "Sobreiro"),
        ("ProductVersion", product.as_str()),
    ];
    let mut table = Vec::new();
    for (name, value) in strings {
        let text = utf16(value);
        table.push(block(name, text.len() / 2, true, &text, &[])?);
    }
    let table = block(TABLE, 0, true, &[], &table)?;
    let string_info = block("StringFileInfo", 0, true, &[], &[table])?;

    let mut translation = Vec::new();
    push_u16(&mut translation, LANGUAGE_NEUTRAL);
    push_u16(&mut translation, CODE_PAGE_UNICODE);
    let var = block("Translation", translation.len(), false, &translation, &[])?;
    let var_info = block("VarFileInfo", 0, true, &[], &[var])?;

    block(
        "VS_VERSION_INFO",
        fixed.len(),
        false,
        &fixed,
        &[string_info, var_info],
    )
}

/// One block of the version resource: length, value length (in bytes for binary values, in
/// characters for text), type (1 text, 0 binary), the key, the value, then the children. The
/// key and the value each start on a 4-byte boundary, and so does each child.
fn block(
    key: &str,
    value_length: usize,
    text: bool,
    value: &[u8],
    children: &[Vec<u8>],
) -> Result<Vec<u8>, Failure> {
    let mut body = Vec::new();
    push_u16(&mut body, 0); // the length, filled in below
    push_u16(&mut body, u16::try_from(value_length)?);
    push_u16(&mut body, u16::from(text));
    body.extend_from_slice(&utf16(key));
    align(&mut body);
    body.extend_from_slice(value);
    for child in children {
        align(&mut body);
        body.extend_from_slice(child);
    }
    let length = u16::try_from(body.len())?;
    body[..2].copy_from_slice(&length.to_le_bytes());
    Ok(body)
}

/// UTF-16, little endian, with the closing NUL.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn align(bytes: &mut Vec<u8>) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn word(bytes: &[u8], at: usize) -> Result<u16, Failure> {
    let part = bytes.get(at..at + 2).ok_or("the icon is cut short")?;
    Ok(u16::from_le_bytes([part[0], part[1]]))
}

fn dword(bytes: &[u8], at: usize) -> Result<u32, Failure> {
    let part = bytes.get(at..at + 4).ok_or("the icon is cut short")?;
    Ok(u32::from_le_bytes([part[0], part[1], part[2], part[3]]))
}
