//! Extracting archives downloaded from forum posts.
//!
//! Only archives are extracted: `.zip`, `.rar`, `.7z`, `.tar`, `.gz`, `.tar.gz` and `.tgz`.
//! Installers and disc images (`.exe`, `.iso`, `.bin`) and anything else are refused without
//! touching the destination. Password-protected archives are opened with [`PASSWORD`], the
//! password releases on the forum use.

use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Component, Path, PathBuf};

/// The password protected archives from the forum are encrypted with.
pub const PASSWORD: &str = "cs.rin.ru";

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("set a folder to extract to in Settings first")]
    NoDestination,

    #[error("{0} is not an archive Rimuru extracts (zip, rar, 7z, tar or gz)")]
    Unsupported(String),

    #[error("the archive is password protected and the password {PASSWORD} didn't open it")]
    WrongPassword,

    #[error("an entry in the archive points outside the destination: {0}")]
    UnsafePath(String),

    #[error("couldn't extract the archive: {0}")]
    Archive(String),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl ExtractError {
    pub fn kind(&self) -> &'static str {
        match self {
            ExtractError::NoDestination => "extract_no_destination",
            ExtractError::Unsupported(_) => "extract_unsupported",
            ExtractError::WrongPassword => "extract_wrong_password",
            ExtractError::UnsafePath(_) => "extract_unsafe_path",
            ExtractError::Archive(_) => "extract_failed",
            ExtractError::Io(_) => "io",
        }
    }
}

pub type Result<T> = std::result::Result<T, ExtractError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    Rar,
    SevenZ,
    Tar,
    TarGz,
    Gz,
}

impl Format {
    /// Detects the format by the file name's extension, ignoring case, along with the name
    /// without that extension.
    pub fn detect(path: &Path) -> Option<(Format, String)> {
        let name = path.file_name()?.to_str()?;
        let lower = name.to_ascii_lowercase();
        let suffixes: &[(&str, Format)] = &[
            (".tar.gz", Format::TarGz),
            (".tgz", Format::TarGz),
            (".tar", Format::Tar),
            (".gz", Format::Gz),
            (".zip", Format::Zip),
            (".rar", Format::Rar),
            (".7z", Format::SevenZ),
        ];
        suffixes.iter().find_map(|(suffix, format)| {
            if !lower.ends_with(suffix) || name.len() == suffix.len() {
                return None;
            }
            Some((*format, name[..name.len() - suffix.len()].to_string()))
        })
    }
}

/// Extracts `archive` into `dest`: an archive's files go into a folder `dest/<name>` named
/// after the archive, so archives don't mix, and a plain `.gz` is decompressed to the file
/// `dest/<name>`. Returns that folder or file.
pub fn extract(archive: &Path, dest: &Path, password: &str) -> Result<PathBuf> {
    let Some((format, stem)) = Format::detect(archive) else {
        let name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| archive.display().to_string());
        return Err(ExtractError::Unsupported(name));
    };
    let stem = part_stem(format, &stem);
    let target = safe_join(dest, Path::new(&stem))?;
    if !archive.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} doesn't exist", archive.display()),
        )
        .into());
    }
    fs::create_dir_all(dest)?;
    match format {
        Format::Zip => extract_zip(archive, &target, password)?,
        Format::Rar => extract_rar(archive, &target, password)?,
        Format::SevenZ => extract_7z(archive, &target, password)?,
        Format::Tar => extract_tar(File::open(archive)?, &target)?,
        Format::TarGz => extract_tar(
            flate2::read::GzDecoder::new(BufReader::new(File::open(archive)?)),
            &target,
        )?,
        Format::Gz => {
            let mut input = flate2::read::MultiGzDecoder::new(BufReader::new(File::open(archive)?));
            let mut output = File::create(&target)?;
            io::copy(&mut input, &mut output).map_err(|e| ExtractError::Archive(e.to_string()))?;
        }
    }
    Ok(target)
}

/// A multi-volume RAR's first part, `game.part1.rar`, extracts into `game`.
fn part_stem(format: Format, stem: &str) -> String {
    if format == Format::Rar {
        let lower = stem.to_ascii_lowercase();
        if let Some(i) = lower.rfind(".part") {
            if i > 0 && lower[i + 5..].chars().all(|c| c.is_ascii_digit()) && lower.len() > i + 5 {
                return stem[..i].to_string();
            }
        }
    }
    stem.to_string()
}

/// Joins an entry's path onto `dest`, refusing absolute paths and `..`. Backslashes count as
/// separators, since archives made on Windows use them.
fn safe_join(dest: &Path, entry: &Path) -> Result<PathBuf> {
    let unsafe_path = || ExtractError::UnsafePath(entry.display().to_string());
    let normalised = entry.to_string_lossy().replace('\\', "/");
    let mut out = dest.to_path_buf();
    let mut pushed = false;
    for component in Path::new(&normalised).components() {
        match component {
            Component::Normal(part) => {
                out.push(part);
                pushed = true;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(unsafe_path());
            }
        }
    }
    if pushed { Ok(out) } else { Err(unsafe_path()) }
}

fn extract_zip(archive: &Path, target: &Path, password: &str) -> Result<()> {
    use zip::result::ZipError;

    let zip_error = |e: ZipError| match e {
        ZipError::InvalidPassword => ExtractError::WrongPassword,
        ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED) => ExtractError::WrongPassword,
        ZipError::Io(e) => ExtractError::Io(e),
        e => ExtractError::Archive(e.to_string()),
    };
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?)).map_err(zip_error)?;
    fs::create_dir_all(target)?;
    for i in 0..zip.len() {
        let encrypted = zip.by_index_raw(i).map_err(zip_error)?.encrypted();
        let mut entry = if encrypted {
            zip.by_index_decrypt(i, password.as_bytes())
        } else {
            zip.by_index(i)
        }
        .map_err(zip_error)?;
        if entry.is_symlink() {
            continue;
        }
        let name = entry.name().map_err(zip_error)?.into_owned();
        let path = safe_join(target, Path::new(&name))?;
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&path)?;
        // A wrong ZipCrypto password can pass the header check and only fail on the data.
        io::copy(&mut entry, &mut out).map_err(|e| {
            if encrypted {
                ExtractError::WrongPassword
            } else {
                ExtractError::Archive(e.to_string())
            }
        })?;
    }
    Ok(())
}

fn extract_7z(archive: &Path, target: &Path, password: &str) -> Result<()> {
    use sevenz_rust2::Error as SevenZError;

    fs::create_dir_all(target)?;
    sevenz_rust2::decompress_file_with_password(archive, target, password.into()).map_err(|e| {
        match e {
            SevenZError::PasswordRequired | SevenZError::MaybeBadPassword(_) => {
                ExtractError::WrongPassword
            }
            e => ExtractError::Archive(e.to_string()),
        }
    })
}

fn extract_rar(archive: &Path, target: &Path, password: &str) -> Result<()> {
    use unrar::error::{Code, UnrarError};

    let rar_error = |e: UnrarError| match e.code {
        Code::MissingPassword | Code::BadPassword => ExtractError::WrongPassword,
        _ => ExtractError::Archive(e.to_string()),
    };
    fs::create_dir_all(target)?;
    let mut open = unrar::Archive::with_password(archive, password)
        .open_for_processing()
        .map_err(rar_error)?;
    while let Some(header) = open.read_header().map_err(rar_error)? {
        let entry = header.entry();
        let path = safe_join(target, &entry.filename)?;
        open = if entry.is_directory() {
            fs::create_dir_all(&path)?;
            header.skip()
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            header.extract_to(&path)
        }
        .map_err(rar_error)?;
    }
    Ok(())
}

fn extract_tar(reader: impl io::Read, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    let mut tar = tar::Archive::new(reader);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        safe_join(target, &path)?;
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            continue;
        }
        // `unpack_in` also refuses anything that would land outside `target`.
        entry.unpack_in(target)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn write_zip(path: &Path, files: &[(&str, &[u8])], password: Option<&str>) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, data) in files {
            let options = SimpleFileOptions::default();
            let options = match password {
                Some(pw) => options.with_aes_encryption(zip::AesMode::Aes256, pw),
                None => options,
            };
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    fn write_tar_gz(path: &Path, files: &[(&str, &[u8])]) {
        let gz = flate2::write::GzEncoder::new(File::create(path).unwrap(), Default::default());
        let mut tar = tar::Builder::new(gz);
        for (name, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, *data).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
    }

    fn is_empty(dir: &Path) -> bool {
        !dir.exists() || fs::read_dir(dir).unwrap().next().is_none()
    }

    #[test]
    fn extracts_a_zip_into_the_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("Game v1.0.zip");
        write_zip(
            &archive,
            &[("readme.txt", b"hello"), ("bin/game.dat", b"data")],
            None,
        );
        let dest = tmp.path().join("extracted");

        let out = extract(&archive, &dest, PASSWORD).unwrap();

        assert_eq!(out, dest.join("Game v1.0"));
        assert_eq!(fs::read(out.join("readme.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(out.join("bin/game.dat")).unwrap(), b"data");
    }

    #[test]
    fn extracts_a_password_protected_zip_with_the_forum_password() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("locked.zip");
        write_zip(&archive, &[("secret.txt", b"opened")], Some(PASSWORD));
        let dest = tmp.path().join("out");

        let out = extract(&archive, &dest, PASSWORD).unwrap();

        assert_eq!(fs::read(out.join("secret.txt")).unwrap(), b"opened");
    }

    #[test]
    fn reports_a_wrong_password() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("locked.zip");
        write_zip(
            &archive,
            &[("secret.txt", b"opened")],
            Some("something else"),
        );

        let err = extract(&archive, &tmp.path().join("out"), PASSWORD).unwrap_err();

        assert!(matches!(err, ExtractError::WrongPassword), "{err:?}");
    }

    #[test]
    fn extracts_a_tar_gz_into_the_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("Game.TAR.GZ");
        write_tar_gz(&archive, &[("dir/file.txt", b"tarred")]);
        let dest = tmp.path().join("out");

        let out = extract(&archive, &dest, PASSWORD).unwrap();

        assert_eq!(out, dest.join("Game"));
        assert_eq!(fs::read(out.join("dir/file.txt")).unwrap(), b"tarred");
    }

    #[test]
    fn extracts_a_7z_with_the_forum_password() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("a.txt"), b"seven").unwrap();
        let archive = tmp.path().join("game.7z");
        sevenz_rust2::compress_to_path_encrypted(&src, &archive, PASSWORD.into()).unwrap();
        let dest = tmp.path().join("out");

        let out = extract(&archive, &dest, PASSWORD).unwrap();

        assert_eq!(fs::read(out.join("a.txt")).unwrap(), b"seven");
    }

    #[test]
    fn decompresses_a_plain_gz_to_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("notes.txt.gz");
        let mut gz =
            flate2::write::GzEncoder::new(File::create(&archive).unwrap(), Default::default());
        gz.write_all(b"gzipped").unwrap();
        gz.finish().unwrap();
        let dest = tmp.path().join("out");

        let out = extract(&archive, &dest, PASSWORD).unwrap();

        assert_eq!(out, dest.join("notes.txt"));
        assert_eq!(fs::read(out).unwrap(), b"gzipped");
    }

    #[test]
    fn skips_installers_and_disc_images() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out");
        for name in [
            "setup.exe",
            "game.iso",
            "game.bin",
            "SETUP.EXE",
            "readme.txt",
        ] {
            let file = tmp.path().join(name);
            fs::write(&file, b"not an archive").unwrap();

            let err = extract(&file, &dest, PASSWORD).unwrap_err();

            assert!(
                matches!(err, ExtractError::Unsupported(_)),
                "{name}: {err:?}"
            );
            assert!(is_empty(&dest), "{name} wrote into the destination");
        }
    }

    #[test]
    fn refuses_zip_entries_outside_the_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("evil.zip");
        write_zip(&archive, &[("../../escaped.txt", b"x")], None);
        let dest = tmp.path().join("a/b");

        let err = extract(&archive, &dest, PASSWORD).unwrap_err();

        assert!(matches!(err, ExtractError::UnsafePath(_)), "{err:?}");
        assert!(!tmp.path().join("escaped.txt").exists());
        assert!(!tmp.path().join("a/escaped.txt").exists());
    }

    #[test]
    fn detects_formats_by_extension() {
        let detect = |n: &str| Format::detect(Path::new(n));
        assert_eq!(detect("a.zip"), Some((Format::Zip, "a".into())));
        assert_eq!(detect("a.RAR"), Some((Format::Rar, "a".into())));
        assert_eq!(detect("a.7z"), Some((Format::SevenZ, "a".into())));
        assert_eq!(detect("a.tar"), Some((Format::Tar, "a".into())));
        assert_eq!(detect("a.tar.gz"), Some((Format::TarGz, "a".into())));
        assert_eq!(detect("a.tgz"), Some((Format::TarGz, "a".into())));
        assert_eq!(detect("a.gz"), Some((Format::Gz, "a".into())));
        assert_eq!(detect("a.iso"), None);
        assert_eq!(detect(".zip"), None);
    }

    #[test]
    fn names_multi_part_rars_after_the_game() {
        assert_eq!(part_stem(Format::Rar, "Game.part1"), "Game");
        assert_eq!(part_stem(Format::Rar, "Game.part"), "Game.part");
        assert_eq!(part_stem(Format::Zip, "Game.part1"), "Game.part1");
    }

    #[test]
    fn safe_join_rejects_escapes() {
        let dest = Path::new("/d");
        assert!(safe_join(dest, Path::new("..\\x")).is_err());
        assert!(safe_join(dest, Path::new("/etc/passwd")).is_err());
        assert_eq!(
            safe_join(dest, Path::new("a\\b")).unwrap(),
            Path::new("/d/a/b")
        );
    }
}
