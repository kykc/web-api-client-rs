extern crate windres;

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use windres::Build;

fn main() {
    let target = env::var_os("TARGET")
        .expect("TARGET")
        .into_string()
        .expect("TARGET");
    if target.contains("-windows-gnu") {
        mingw_check_47048();

        Build::new().compile("src/main.rc").unwrap();
    }
}

fn mingw_check_47048() {
    let rustc = env::var_os("RUSTC").expect("RUSTC");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let try_dir = out_dir.join("try_47048");
    let mut cmd;

    fs::create_dir_all(&try_dir).expect("create directory");
    create_file(&try_dir.join("say_hi.c"), SAY_HI_C);
    create_file(&try_dir.join("c_main.c"), C_MAIN_C);
    create_file(&try_dir.join("r_main.rs"), R_MAIN_RS);
    create_file(&try_dir.join("workaround.c"), WORKAROUND_C);

    cmd = Command::new("gcc");
    cmd.current_dir(&try_dir).args(&["-fPIC", "-c", "say_hi.c"]);
    execute(cmd);

    cmd = Command::new("ar");
    cmd.current_dir(&try_dir)
        .args(&["cr", "libsay_hi.a", "say_hi.o"]);
    execute(cmd);

    cmd = Command::new("gcc");
    cmd.current_dir(&try_dir)
        .args(&["c_main.c", "-L.", "-lsay_hi", "-o", "c_main.exe"]);
    execute(cmd);

    // try simple rustc command that should work, so that failure
    // really is the bug being checked for
    cmd = Command::new(&rustc);
    cmd.arg("--version");
    execute(cmd);

    cmd = Command::new(&rustc);
    cmd.current_dir(&try_dir)
        .args(&["r_main.rs", "-L.", "-lsay_hi", "-o", "r_main.exe"]);
    let status = cmd
        .status()
        .unwrap_or_else(|_| panic!("Unable to execute: {:?}", cmd));
    let need_workaround = !status.success();

    // build and test libworkaround_47048.a
    if need_workaround {
        cmd = Command::new("gcc");
        cmd.current_dir(&try_dir).args(&["-fPIC", "-O2", "-c", "workaround.c"]);
        execute(cmd);

        cmd = Command::new("ar");
        cmd.current_dir(&try_dir)
            .args(&["cr", "libworkaround_47048.a", "workaround.o"]);
        execute(cmd);

        cmd = Command::new(&rustc);
        cmd.current_dir(&try_dir).args(&[
            "r_main.rs",
            "-L.",
            "-lsay_hi",
            "-lworkaround_47048",
            "-o",
            "r_main.exe",
        ]);
        execute(cmd);

        let src = try_dir.join("libworkaround_47048.a");
        let lib_dir = out_dir.join("lib");
        fs::create_dir_all(&lib_dir).expect("create directory");
        let dst = lib_dir.join("libworkaround_47048.a");
        fs::rename(src, dst).expect("move file");

        let lib_dir_str = lib_dir.to_str().expect("unsupported characters");
        println!("cargo:rustc-link-search=native={}", lib_dir_str);
        println!("cargo:rustc-link-lib=static=workaround_47048");
    }

    fs::remove_dir_all(try_dir).expect("remove directory");
}

fn create_file(filename: &Path, contents: &str) {
    let mut file = File::create(filename)
        .unwrap_or_else(|_| panic!("Unable to create file: {:?}", filename));
    file.write_all(contents.as_bytes())
        .unwrap_or_else(|_| panic!("Unable to write to file: {:?}", filename));
}

fn execute(mut command: Command) {
    let status = command
        .status()
        .unwrap_or_else(|_| panic!("Unable to execute: {:?}", command));
    if !status.success() {
        if let Some(code) = status.code() {
            panic!("Program failed with code {}: {:?}", code, command);
        } else {
            panic!("Program failed: {:?}", command);
        }
    }
}

const SAY_HI_C: &'static str = r#"/* say_hi.c */
#include <stdio.h>
void say_hi(void) {
    fprintf(stdout, "hi!\n");
}
"#;

const C_MAIN_C: &'static str = r#"/* c_main.c */
void say_hi(void);
int main(void) {
    say_hi();
    return 0;
}
"#;

const R_MAIN_RS: &'static str = r#"// r_main.rs
extern "C" {
    fn say_hi();
}
fn main() {
    unsafe {
        say_hi();
    }
}
"#;

const WORKAROUND_C: &'static str = r#"/* workaround.c */
#define _CRTBLD
#include <stdio.h>
int xmlIndentTreeOutput = 0; // Fix for borken msys2 libxml2, should be removed when fix arrives from their side
FILE *__cdecl __acrt_iob_func(unsigned index)
{
    return &(__iob_func()[index]);
}

typedef FILE *__cdecl (*_f__acrt_iob_func)(unsigned index);
_f__acrt_iob_func __MINGW_IMP_SYMBOL(__acrt_iob_func) = __acrt_iob_func;
"#;