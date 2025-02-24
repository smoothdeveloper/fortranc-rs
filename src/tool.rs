use crate::command_helper::{run, run_output, CargoOutput, OutputKind};
use crate::tempfile::NamedTempfile;
use crate::{Error, ErrorKind};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::RwLock;

pub(crate) type CompilerFamilyLookupCache = HashMap<Box<[Box<OsStr>]>, ToolFamily>;

/// Configuration used to represent an invocation of a Fortran Compiler.
///
/// This can be used to figure out what compiler is in use, what the arguments
/// to it are, and what the environment variables look like for the compiler.
/// This can be used to further configure other build systems (e.g. forward
/// along FC and/or FFLAGS) or the `to_command` method can be used to run the
/// compiler itself.
#[derive(Clone, Debug)]
pub struct Tool {
    pub(crate) path: PathBuf,
    //pub(crate) fortranc_wrapper_path: Option<PathBuf>,
    //pub(crate) fortranc_wrapper_args: Vec<OsString>,
    pub(crate) args: Vec<OsString>,
    pub(crate) env: Vec<(OsString, OsString)>,
    pub(crate) family: ToolFamily,
    pub(crate) removed_args: Vec<OsString>,
    pub(crate) has_internal_target_arg: bool,
}

impl Tool {
    pub(crate) fn new(
        path: PathBuf,
        cached_compiler_family: &RwLock<CompilerFamilyLookupCache>,
        cargo_output: &CargoOutput,
        out_dir: Option<&Path>,
    ) -> Self {
        Self::with_features(path, vec![], cached_compiler_family, cargo_output, out_dir)
    }

    pub(crate) fn with_args(
        path: PathBuf,
        args: Vec<String>,
        cached_compiler_family: &RwLock<CompilerFamilyLookupCache>,
        cargo_output: &CargoOutput,
        out_dir: Option<&Path>,
    ) -> Self {
        Self::with_features(path, args, cached_compiler_family, cargo_output, out_dir)
    }

    /// Explicitly set the `ToolFamily`, skipping name-based detection.
    pub(crate) fn with_family(path: PathBuf, family: ToolFamily) -> Self {
        Self {
            path,
            //      fortranc_wrapper_path: None,
            //      fortranc_wrapper_args: Vec::new(),
            args: Vec::new(),
            env: Vec::new(),
            family,
            removed_args: Vec::new(),
            has_internal_target_arg: false,
        }
    }

    pub(crate) fn with_features(
        path: PathBuf,
        args: Vec<String>,
        cached_compiler_family: &RwLock<CompilerFamilyLookupCache>,
        cargo_output: &CargoOutput,
        out_dir: Option<&Path>,
    ) -> Self {
        /*
        fn is_zig_cc(path: &Path, cargo_output: &CargoOutput) -> bool {
            run_output(
                Command::new(path).arg("--version"),
                // tool detection issues should always be shown as warnings
                cargo_output,
            )
              .map(|o| String::from_utf8_lossy(&o).contains("ziglang"))
              .unwrap_or_default()
              || {
                match path.file_name().map(OsStr::to_string_lossy) {
                    Some(fname) => fname.contains("zig"),
                    _ => false,
                }
            }
        }*/

        fn guess_family_from_stdout(
            stdout: &str,
            path: &Path,
            args: &[String],
            cargo_output: &CargoOutput,
        ) -> Result<ToolFamily, Error> {
            cargo_output.print_debug(&stdout);

            // https://gitlab.kitware.com/cmake/cmake/-/blob/69a2eeb9dff5b60f2f1e5b425002a0fd45b7cadb/Modules/CMakeDetermineCompilerId.cmake#L267-271
            // stdin is set to null to ensure that the help output is never paginated.
            let accepts_cl_style_flags = run(
                Command::new(path).args(args).arg("-?").stdin(Stdio::null()),
                &{
                    // the errors are not errors!
                    let mut cargo_output = cargo_output.clone();
                    cargo_output.warnings = cargo_output.debug;
                    cargo_output.output = OutputKind::Discard;
                    cargo_output
                },
            )
            .is_ok();

            let flang = stdout.contains(r#""Flang""#);
            let gfortran = stdout.contains(r#""GNU Fortran""#);
            /*
            match (clang, accepts_cl_style_flags, gcc, emscripten, vxworks) {
                (clang_cl, true, _, false, false) => Ok(ToolFamily::Msvc { clang_cl }),
                (true, _, _, _, false) | (_, _, _, true, false) => Ok(ToolFamily::Clang {
                    zig_cc: is_zig_cc(path, cargo_output),
                }),
                (false, false, true, _, false) | (_, _, _, _, true) => Ok(ToolFamily::Gnu),
                (false, false, false, false, false) => {
                    cargo_output.print_warning(&"Compiler family detection failed since it does not define `__clang__`, `__GNUC__`, `__EMSCRIPTEN__` or `__VXWORKS__`, also does not accept cl style flag `-?`, fallback to treating it as GNU");
                    Err(Error::new(
                        ErrorKind::ToolFamilyMacroNotFound,
                        "Expects macro `__clang__`, `__GNUC__` or `__EMSCRIPTEN__`, `__VXWORKS__` or accepts cl style flag `-?`, but found none",
                    ))
                }
            }*/
            Ok(ToolFamily::GFortran)
        }

        fn detect_family_inner(
            path: &Path,
            args: &[String],
            cargo_output: &CargoOutput,
            out_dir: Option<&Path>,
        ) -> Result<ToolFamily, Error> {
            let out_dir = out_dir
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(env::temp_dir()));

            // Ensure all the parent directories exist otherwise temp file creation
            // will fail
            std::fs::create_dir_all(&out_dir).map_err(|err| Error {
                kind: ErrorKind::IOError,
                message: format!("failed to create OUT_DIR '{}': {}", out_dir.display(), err)
                    .into(),
            })?;

            let mut tmp =
                NamedTempfile::new(&out_dir, "detect_compiler_family.f90").map_err(|err| {
                    Error {
                        kind: ErrorKind::IOError,
                        message: format!(
                            "failed to create detect_compiler_family.c temp file in '{}': {}",
                            out_dir.display(),
                            err
                        )
                        .into(),
                    }
                })?;
            let mut tmp_file = tmp.take_file().unwrap();
            tmp_file.write_all(include_bytes!("detect_compiler_family.f90"))?;
            // Close the file handle *now*, otherwise the compiler may fail to open it on Windows
            // (#1082). The file stays on disk and its path remains valid until `tmp` is dropped.
            tmp_file.flush()?;
            tmp_file.sync_data()?;
            drop(tmp_file);

            // When expanding the file, the compiler prints a lot of information to stderr
            // that it is not an error, but related to expanding itself.
            //
            // cc would have to disable warning here to prevent generation of too many warnings.
            let mut compiler_detect_output = cargo_output.clone();
            compiler_detect_output.warnings = compiler_detect_output.debug;

            let stdout = run_output(
                Command::new(path).arg("-cpp").arg("-E").arg(tmp.path()),
                &compiler_detect_output,
            )?;
            let stdout = String::from_utf8_lossy(&stdout);

            if stdout.contains("-Wslash-u-filename") {
                let stdout = run_output(
                    Command::new(path)
                        .arg("-cpp")
                        .arg("-E")
                        .arg("--")
                        .arg(tmp.path()),
                    &compiler_detect_output,
                )?;
                let stdout = String::from_utf8_lossy(&stdout);
                guess_family_from_stdout(&stdout, path, args, cargo_output)
            } else {
                guess_family_from_stdout(&stdout, path, args, cargo_output)
            }
        }
        let detect_family = |path: &Path, args: &[String]| -> Result<ToolFamily, Error> {
            let cache_key = [path.as_os_str()]
                .iter()
                .cloned()
                .chain(args.iter().map(OsStr::new))
                .map(Into::into)
                .collect();
            if let Some(family) = cached_compiler_family.read().unwrap().get(&cache_key) {
                return Ok(*family);
            }

            let family = detect_family_inner(path, args, cargo_output, out_dir)?;
            cached_compiler_family
                .write()
                .unwrap()
                .insert(cache_key, family);
            Ok(family)
        };

        let family = detect_family(&path, &args).unwrap_or_else(|e| {
            cargo_output.print_warning(&format_args!(
                "Compiler family detection failed due to error: {}",
                e
            ));
            match path.file_name().map(OsStr::to_string_lossy) {
                /*
                Some(fname) if fname.contains("clang-cl") => ToolFamily::Msvc { clang_cl: true },
                Some(fname) if fname.ends_with("cl") || fname == "cl.exe" => {
                    ToolFamily::Msvc { clang_cl: false }
                }
                Some(fname) if fname.contains("clang") => {
                    let is_clang_cl = args
                      .iter()
                      .any(|a| a.strip_prefix("--driver-mode=") == Some("cl"));
                    if is_clang_cl {
                        ToolFamily::Msvc { clang_cl: true }
                    } else {
                        ToolFamily::Clang {
                            zig_cc: is_zig_cc(&path, cargo_output),
                        }
                    }
                }
                Some(fname) if fname.contains("zig") => ToolFamily::Clang { zig_cc: true },
                _ => ToolFamily::Gnu,*/
                _ => ToolFamily::GFortran,
            }
        });

        Tool {
            path,
            //fortranc_wrapper_path: None,
            //fortranc_wrapper_args: Vec::new(),
            args: Vec::new(),
            env: Vec::new(),
            family,
            removed_args: Vec::new(),
            has_internal_target_arg: false,
        }
    }

    /// Add an argument to be stripped from the final command arguments.
    pub(crate) fn remove_arg(&mut self, flag: OsString) {
        self.removed_args.push(flag);
    }

    /// Push an "exotic" flag to the end of the compiler's arguments list.
    pub(crate) fn push_cc_arg(&mut self, flag: OsString) {
        self.args.push(flag);
    }

    /// Checks if an argument or flag has already been specified or conflicts.
    ///
    /// Currently only checks optimization flags.
    pub(crate) fn is_duplicate_opt_arg(&self, flag: &OsString) -> bool {
        let flag = flag.to_str().unwrap();
        let mut chars = flag.chars();

        // todo: something similar to is_like_msvc may be needed
        // intel ifx/ifort on windows require / rather than - for compiler flags
        /*
        // Only duplicate check compiler flags
        if self.is_like_msvc() {
            if chars.next() != Some('/') {
                return false;
            }
        } else*/
        if (self.is_like_gnu_fortran() || self.is_like_flang()) && chars.next() != Some('-') {
            return false;
        }

        // Check for existing optimization flags (-O, /O)
        if chars.next() == Some('O') {
            return self
                .args()
                .iter()
                .any(|a| a.to_str().unwrap_or("").chars().nth(1) == Some('O'));
        }

        // TODO Check for existing -m..., -m...=..., /arch:... flags
        false
    }

    /// Don't push optimization arg if it conflicts with existing args.
    pub(crate) fn push_opt_unless_duplicate(&mut self, flag: OsString) {
        if self.is_duplicate_opt_arg(&flag) {
            eprintln!("Info: Ignoring duplicate arg {:?}", &flag);
        } else {
            self.push_cc_arg(flag);
        }
    }

    /// Converts this compiler into a `Command` that's ready to be run.
    ///
    /// This is useful for when the compiler needs to be executed and the
    /// command returned will already have the initial arguments and environment
    /// variables configured.
    pub fn to_command(&self) -> Command {
        /*
                let mut cmd = match self.fortranc_wrapper_path {
                    Some(ref cc_wrapper_path) => {
                        let mut cmd = Command::new(cc_wrapper_path);
                        cmd.arg(&self.path);
                        cmd
                    }
                    None => Command::new(&self.path),
                };
                cmd.args(&self.fortranc_wrapper_args);
        */
        let mut cmd = Command::new(&self.path);
        let value = self
            .args
            .iter()
            .filter(|a| !self.removed_args.contains(a))
            .collect::<Vec<_>>();
        cmd.args(&value);

        for (k, v) in self.env.iter() {
            cmd.env(k, v);
        }
        cmd
    }

    /// Returns the path for this compiler.
    ///
    /// Note that this may not be a path to a file on the filesystem, e.g. "cc",
    /// but rather something which will be resolved when a process is spawned.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the default set of arguments to the compiler needed to produce
    /// executables for the target this compiler generates.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// Returns the set of environment variables needed for this compiler to
    /// operate.
    ///
    /// This is typically only used for MSVC compilers currently.
    pub fn env(&self) -> &[(OsString, OsString)] {
        &self.env
    }

    /// Returns the compiler command in format of FC environment variable.
    /// Or empty string if FC env was not present
    ///
    /// This is typically used by configure script
    pub fn fortranc_env(&self) -> OsString {
        /*
        match self.fortranc_wrapper_path {
            Some(ref fortranc_wrapper_path) => {
                let mut cc_env = fortranc_wrapper_path.as_os_str().to_owned();
                cc_env.push(" ");
                cc_env.push(self.path.to_path_buf().into_os_string());
                for arg in self.fortranc_wrapper_args.iter() {
                    cc_env.push(" ");
                    cc_env.push(arg);
                }
                cc_env
            }
            None => OsString::from(""),
        }*/
        OsString::from("")
    }

    /// Returns the compiler flags in format of FFLAGS environment variable.
    /// Important here - this will not be FFLAGS from env, its internal gfortran's flags to use as FFLAGS
    /// This is typically used by configure script
    pub fn fflags_env(&self) -> OsString {
        let mut flags = OsString::new();
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                flags.push(" ");
            }
            flags.push(arg);
        }
        flags
    }

    /// Whether the tool is GNU Compiler Collection-like.
    pub fn is_like_gnu_fortran(&self) -> bool {
        self.family == ToolFamily::GFortran
    }

    /// Whether the tool is Flang-like.
    pub fn is_like_flang(&self) -> bool {
        matches!(self.family, ToolFamily::Flang { .. })
    }
    /*
    /// Whether the tool is AppleClang under .xctoolchain
    #[cfg(target_vendor = "apple")]
    pub(crate) fn is_xctoolchain_clang(&self) -> bool {
        let path = self.path.to_string_lossy();
        path.contains(".xctoolchain/")
    }
    #[cfg(not(target_vendor = "apple"))]
    pub(crate) fn is_xctoolchain_clang(&self) -> bool {
        false
    }*/
}
/// Represents the family of tools this tool belongs to.
///
/// Each family of tools differs in how and what arguments they accept.
///
/// Detection of a family is done on best-effort basis and may not accurately reflect the tool.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ToolFamily {
    /// Tool is GNU Fortran Compiler-like
    GFortran,
    /// Tool is Intel IFX Fortran Compiler-like
    IntelIFX,
    Flang,
    LFortran,
}

impl ToolFamily {
    pub(crate) fn verbose_stderr(&self) -> bool {
        matches!(*self, ToolFamily::Flang { .. })
    }
}
