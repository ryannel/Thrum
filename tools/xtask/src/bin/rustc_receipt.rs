//! Bounded macOS rustc-invocation receipts for a local T2 candidate.
//!
//! This observes supplied invocation inputs; it does not attest compiler reads or
//! make an executable ready. The caller owns an exclusive, initially empty receipt
//! directory, retains it until every child exits, and verifies all receipts.

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stderr,
    reason = "a refused compiler invocation must report its error to the CLI caller"
)]
fn main() {
    match candidate::run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("rustc receipt candidate refused: {error}");
            std::process::exit(125);
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[expect(
    clippy::print_stderr,
    reason = "unsupported-platform CLI use must report its error"
)]
fn main() {
    eprintln!("rustc receipt candidate requires macOS");
    std::process::exit(125);
}

#[cfg(any(target_os = "macos", all(test, unix)))]
mod candidate {
    use std::{
        env,
        ffi::{OsStr, OsString},
        fs::{self, File, OpenOptions},
        io::{self, Write},
        os::unix::{ffi::OsStrExt, process::ExitStatusExt},
        path::{Path, PathBuf},
        process::{Command, ExitStatus},
    };

    const SELECTED_RUSTC: &str = "THRUM_SELECTED_RUSTC";
    const RECEIPT_DIR: &str = "THRUM_RECEIPT_DIR";
    const MAX_ARGUMENTS: usize = 1024;
    const MAX_ENVIRONMENT: usize = 512;
    const MAX_FIELD_BYTES: usize = 128 * 1024;
    const MAX_RECEIPT_BYTES: usize = 1024 * 1024;
    const COMPLETION_BYTES: usize = 9;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[derive(Debug, PartialEq)]
    enum Kind {
        Probe,
        Compile,
    }

    struct Invocation {
        rustc: PathBuf,
        cwd: PathBuf,
        arguments: Vec<OsString>,
        environment: Vec<(OsString, OsString)>,
    }

    impl Invocation {
        fn capture(
            selected: &Path,
            rustc: OsString,
            arguments: impl IntoIterator<Item = OsString>,
            environment: impl IntoIterator<Item = (OsString, OsString)>,
            cwd: PathBuf,
        ) -> Result<Self> {
            if !selected.is_absolute() || !Path::new(&rustc).is_absolute() {
                return Err("selected and invoked rustc must be absolute paths".into());
            }
            if rustc != selected.as_os_str() {
                return Err("invoked rustc differs from the selected executable".into());
            }
            if !cwd.is_absolute() {
                return Err("compiler working directory must be absolute".into());
            }
            // Header, vector counts, pending marker and completion frame. Check
            // aggregate growth during capture, before retaining more fields.
            let mut captured_bytes = 8 + 4 + 1 + 4 + 4 + 4 + COMPLETION_BYTES;
            account_field(&mut captured_bytes, selected.as_os_str())?;
            account_field(&mut captured_bytes, cwd.as_os_str())?;
            let mut captured_arguments = Vec::new();
            for argument in arguments {
                if captured_arguments.len() == MAX_ARGUMENTS {
                    return Err("too many rustc arguments".into());
                }
                if argument.as_bytes().starts_with(b"@") {
                    return Err("rustc response-file arguments are unsupported".into());
                }
                account_field(&mut captured_bytes, &argument)?;
                captured_arguments.push(argument);
            }
            let mut captured_environment = Vec::new();
            for (name, value) in environment {
                if name == SELECTED_RUSTC || name == RECEIPT_DIR {
                    continue;
                }
                if captured_environment.len() == MAX_ENVIRONMENT {
                    return Err("too many rustc environment variables".into());
                }
                account_field(&mut captured_bytes, &name)?;
                account_field(&mut captured_bytes, &value)?;
                captured_environment.push((name, value));
            }
            captured_environment.sort_unstable_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            if captured_environment
                .windows(2)
                .any(|pair| pair[0].0 == pair[1].0)
            {
                return Err("duplicate rustc environment variable".into());
            }
            Ok(Self {
                rustc: PathBuf::from(rustc),
                cwd,
                arguments: captured_arguments,
                environment: captured_environment,
            })
        }

        fn encode(&self) -> Result<Vec<u8>> {
            let mut bytes = b"THRMRCP1".to_vec();
            bytes.extend_from_slice(&1_u32.to_le_bytes());
            bytes.push(match kind(&self.arguments) {
                Kind::Probe => 0,
                Kind::Compile => 1,
            });
            field(&mut bytes, self.rustc.as_os_str())?;
            field(&mut bytes, self.cwd.as_os_str())?;
            count(&mut bytes, self.arguments.len());
            for argument in &self.arguments {
                field(&mut bytes, argument)?;
            }
            count(&mut bytes, self.environment.len());
            for (name, value) in &self.environment {
                field(&mut bytes, name)?;
                field(&mut bytes, value)?;
            }
            if bytes.len() + 4 + COMPLETION_BYTES > MAX_RECEIPT_BYTES {
                return Err("rustc receipt exceeds its byte limit".into());
            }
            bytes.extend_from_slice(b"PEND");
            Ok(bytes)
        }

        fn command(&self) -> Command {
            let mut command = Command::new(&self.rustc);
            command
                .args(&self.arguments)
                .current_dir(&self.cwd)
                .env_clear()
                .envs(self.environment.iter().map(|(name, value)| (name, value)));
            command
        }
    }

    fn check_field(value: &OsStr) -> Result<()> {
        if value.as_bytes().len() > MAX_FIELD_BYTES {
            return Err("rustc receipt field exceeds its byte limit".into());
        }
        Ok(())
    }

    fn account_field(bytes: &mut usize, value: &OsStr) -> Result<()> {
        check_field(value)?;
        *bytes += 4 + value.as_bytes().len();
        if *bytes > MAX_RECEIPT_BYTES {
            return Err("rustc receipt exceeds its byte limit".into());
        }
        Ok(())
    }

    fn count(bytes: &mut Vec<u8>, value: usize) {
        // All callers have already bounded counts/field lengths below u32::MAX.
        bytes.extend_from_slice(
            &u32::try_from(value)
                .expect("validated receipt count fits u32")
                .to_le_bytes(),
        );
    }

    fn field(bytes: &mut Vec<u8>, value: &OsStr) -> Result<()> {
        check_field(value)?;
        if bytes.len() + 4 + value.as_bytes().len() + COMPLETION_BYTES > MAX_RECEIPT_BYTES {
            return Err("rustc receipt exceeds its byte limit".into());
        }
        count(bytes, value.as_bytes().len());
        bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn kind(arguments: &[OsString]) -> Kind {
        if arguments.len() == 1 && matches!(arguments[0].as_bytes(), b"-vV" | b"-V" | b"--version")
        {
            return Kind::Probe;
        }
        // Cargo's stdin/file-name/config queries have no --emit. Only known
        // information-only prints count as probes: --print=link-args, for example,
        // can accompany a real compilation and must not receive that label.
        let mut query = false;
        let mut arguments = arguments.iter();
        while let Some(argument) = arguments.next() {
            let argument = argument.as_bytes();
            if argument == b"--emit" || argument.starts_with(b"--emit=") {
                return Kind::Compile;
            }
            let print = if argument == b"--print" {
                arguments.next().map(|value| value.as_bytes())
            } else {
                argument.strip_prefix(b"--print=")
            };
            if let Some(print) = print {
                if !matches!(
                    print,
                    b"file-names"
                        | b"sysroot"
                        | b"split-debuginfo"
                        | b"crate-name"
                        | b"cfg"
                        | b"target-libdir"
                        | b"target-list"
                ) {
                    return Kind::Compile;
                }
                query = true;
            }
        }
        if query { Kind::Probe } else { Kind::Compile }
    }

    fn start_receipt(directory: &Path, payload: &[u8]) -> Result<File> {
        if !directory.is_absolute() {
            return Err("receipt directory must be absolute".into());
        }
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("receipt directory must be an ordinary directory".into());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{}.receipt", std::process::id())))?;
        file.write_all(payload)?;
        Ok(file)
    }

    fn complete(file: &mut File, outcome: u8, status: i32) -> io::Result<()> {
        file.write_all(b"DONE")?;
        file.write_all(&[outcome])?;
        file.write_all(&status.to_le_bytes())
    }

    fn status(status: ExitStatus) -> (u8, i32, i32) {
        if let Some(code) = status.code() {
            (0, code, code)
        } else if let Some(signal) = status.signal() {
            (1, signal, 128 + signal)
        } else {
            (4, -1, 125)
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn run() -> Result<i32> {
        let selected = PathBuf::from(env::var_os(SELECTED_RUSTC).ok_or("missing selected rustc")?);
        let directory = PathBuf::from(env::var_os(RECEIPT_DIR).ok_or("missing receipt directory")?);
        let mut arguments = env::args_os().skip(1);
        let rustc = arguments.next().ok_or("missing Cargo rustc argument")?;
        let invocation = Invocation::capture(
            &selected,
            rustc,
            arguments,
            env::vars_os(),
            env::current_dir()?,
        )?;
        let metadata = fs::symlink_metadata(&selected)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("selected rustc must be an ordinary executable file".into());
        }
        let payload = invocation.encode()?;
        let mut receipt = start_receipt(&directory, &payload)?;
        let mut child = match invocation.command().spawn() {
            Ok(child) => child,
            Err(error) => {
                complete(&mut receipt, 2, error.raw_os_error().unwrap_or(-1))?;
                return Err(error.into());
            }
        };
        let child_status = match child.wait() {
            Ok(status) => status,
            Err(error) => {
                complete(&mut receipt, 3, error.raw_os_error().unwrap_or(-1))?;
                return Err(error.into());
            }
        };
        let (outcome, code, exit_code) = status(child_status);
        complete(&mut receipt, outcome, code)?;
        Ok(exit_code)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::ffi::OsStringExt;

        fn strings(values: &[&str]) -> Vec<OsString> {
            values.iter().map(OsString::from).collect()
        }

        fn invocation(
            arguments: Vec<OsString>,
            environment: Vec<(OsString, OsString)>,
        ) -> Result<Invocation> {
            Invocation::capture(
                Path::new("/selected/rustc"),
                OsString::from("/selected/rustc"),
                arguments,
                environment,
                PathBuf::from("/input"),
            )
        }

        #[test]
        fn refuses_wrong_rustc_response_files_and_limits_before_receipt() {
            assert!(
                Invocation::capture(
                    Path::new("/selected/rustc"),
                    OsString::from("/other/rustc"),
                    [],
                    [],
                    PathBuf::from("/input")
                )
                .is_err()
            );
            assert!(invocation(strings(&["@/outside/args"]), vec![]).is_err());
            assert!(invocation(vec![OsString::from("x"); MAX_ARGUMENTS + 1], vec![]).is_err());
            assert!(
                invocation(
                    vec![OsString::from("x".repeat(MAX_FIELD_BYTES + 1))],
                    vec![]
                )
                .is_err()
            );
            assert!(
                invocation(vec![OsString::from("x".repeat(MAX_FIELD_BYTES)); 9], vec![]).is_err()
            );
            let environment = (0..=MAX_ENVIRONMENT)
                .map(|index| (OsString::from(index.to_string()), OsString::new()))
                .collect();
            assert!(invocation(vec![], environment).is_err());
            assert!(
                invocation(
                    vec![],
                    vec![("A".into(), "1".into()), ("A".into(), "2".into())]
                )
                .is_err()
            );
        }

        #[test]
        fn recognizes_cargo_version_file_name_and_config_probes() {
            assert_eq!(kind(&strings(&["-vV"])), Kind::Probe);
            assert_eq!(
                kind(&strings(&[
                    "-",
                    "--crate-name",
                    "___",
                    "--print=file-names",
                    "--crate-type",
                    "bin",
                    "--crate-type",
                    "proc-macro",
                    "--print=sysroot",
                    "--print=split-debuginfo",
                    "--print=crate-name",
                    "--print=cfg",
                    "-Wwarnings"
                ])),
                Kind::Probe
            );
            assert_eq!(kind(&strings(&["--print", "cfg"])), Kind::Probe);
            assert_eq!(
                kind(&strings(&["src/lib.rs", "--emit=dep-info,link"])),
                Kind::Compile
            );
            assert_eq!(
                kind(&strings(&["--print=cfg", "--emit", "link"])),
                Kind::Compile
            );
            assert_eq!(
                kind(&strings(&["src/lib.rs", "--print=link-args"])),
                Kind::Compile
            );
        }

        #[test]
        fn binary_preserves_argument_and_environment_bytes_and_incomplete_marker() {
            let argument = OsString::from_vec(vec![b'x', 0xff, b' ']);
            let invocation = invocation(
                vec![argument.clone(), "second".into()],
                vec![
                    ("Z".into(), OsString::from_vec(vec![0xfe])),
                    (SELECTED_RUSTC.into(), "control".into()),
                    (RECEIPT_DIR.into(), "control".into()),
                    ("A".into(), "first".into()),
                ],
            )
            .unwrap();
            assert_eq!(invocation.environment.len(), 2);
            assert_eq!(invocation.environment[0].0, "A");
            let bytes = invocation.encode().unwrap();
            let mut expected = b"THRMRCP1".to_vec();
            expected.extend_from_slice(&1_u32.to_le_bytes());
            expected.push(1);
            field(&mut expected, OsStr::new("/selected/rustc")).unwrap();
            field(&mut expected, OsStr::new("/input")).unwrap();
            count(&mut expected, 2);
            field(&mut expected, &argument).unwrap();
            field(&mut expected, OsStr::new("second")).unwrap();
            count(&mut expected, 2);
            field(&mut expected, OsStr::new("A")).unwrap();
            field(&mut expected, OsStr::new("first")).unwrap();
            field(&mut expected, OsStr::new("Z")).unwrap();
            field(&mut expected, &OsString::from_vec(vec![0xfe])).unwrap();
            expected.extend_from_slice(b"PEND");
            assert_eq!(bytes, expected);
            let command = invocation.command();
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                invocation
                    .arguments
                    .iter()
                    .map(OsString::as_os_str)
                    .collect::<Vec<_>>()
            );
            assert_eq!(command.get_envs().count(), 2);
            assert!(command.get_envs().all(|(_, value)| value.is_some()));
        }

        #[test]
        fn receipt_is_exclusive_and_appends_completion() {
            let directory =
                env::temp_dir().join(format!("thrum-receipt-test-{}", std::process::id()));
            fs::create_dir(&directory).unwrap();
            let payload = invocation(strings(&["-vV"]), vec![])
                .unwrap()
                .encode()
                .unwrap();
            let mut file = start_receipt(&directory, &payload).unwrap();
            assert_eq!(
                fs::read(directory.join(format!("{}.receipt", std::process::id()))).unwrap(),
                payload
            );
            assert!(start_receipt(&directory, &payload).is_err());
            assert!(start_receipt(Path::new("relative"), &payload).is_err());
            complete(&mut file, 0, 7).unwrap();
            drop(file);
            let bytes =
                fs::read(directory.join(format!("{}.receipt", std::process::id()))).unwrap();
            assert_eq!(&bytes[..payload.len()], payload);
            assert_eq!(
                &bytes[payload.len()..],
                &[b'D', b'O', b'N', b'E', 0, 7, 0, 0, 0]
            );
            fs::remove_dir_all(directory).unwrap();
        }

        #[test]
        fn child_exit_and_signal_failures_remain_distinct() {
            assert_eq!(status(ExitStatus::from_raw(7 << 8)), (0, 7, 7));
            assert_eq!(status(ExitStatus::from_raw(9)), (1, 9, 137));
        }

        #[test]
        fn actual_child_receives_only_the_captured_environment_bytes() {
            let invocation = Invocation::capture(
                Path::new("/usr/bin/env"),
                OsString::from("/usr/bin/env"),
                [],
                [
                    ("Z".into(), OsString::from_vec(vec![0xfe])),
                    (SELECTED_RUSTC.into(), "control".into()),
                    (RECEIPT_DIR.into(), "control".into()),
                    ("A".into(), "first".into()),
                ],
                env::current_dir().unwrap(),
            )
            .unwrap();
            let child = invocation.command().output().unwrap();
            assert!(child.status.success());
            assert_eq!(child.stdout, b"A=first\nZ=\xfe\n");
        }
    }
}
