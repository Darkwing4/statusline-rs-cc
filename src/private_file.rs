use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let mut file =
        owner_only(OpenOptions::new().write(true).create(true).truncate(true)).open(path)?;
    restrict(&file)?;
    file.write_all(contents.as_ref())
}

#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;

#[cfg(unix)]
fn owner_only(options: &mut OpenOptions) -> &mut OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(OWNER_ONLY)
}

#[cfg(unix)]
fn restrict(file: &File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(std::fs::Permissions::from_mode(OWNER_ONLY))
}

#[cfg(not(unix))]
fn owner_only(options: &mut OpenOptions) -> &mut OpenOptions {
    options
}

#[cfg(not(unix))]
fn restrict(_file: &File) -> io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::write;

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn loosen(path: &Path) {
        fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn keeps_new_and_existing_files_readable_by_the_owner_only() {
        let dir = std::env::temp_dir().join(format!("statusline-private-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secret.txt");

        write(&path, "one").unwrap();
        assert_eq!(mode(&path), 0o600);

        loosen(&path);
        write(&path, "two").unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");

        fs::remove_dir_all(&dir).unwrap();
    }
}
