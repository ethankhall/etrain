extern crate assert_cli;
extern crate tempdir;

#[cfg(test)]
mod integration {
    use assert_cli;
    use std::path::PathBuf;
    use tempdir::TempDir;
    use std::env::var;
    use std::io::prelude::*;
    use std::fs;
    use std::os::unix::fs::OpenOptionsExt;

    #[test]
    fn checkout_github_repo() {
        with_test_dir(|tmp_dir| {
            assert_cli::Assert::command(&[build_exec().as_str(), "checkout", "ethankhall/inc"])
            .current_dir(tmp_dir.clone())
            .succeeds()
            .unwrap();

            let inc_dir = tmp_dir.join("inc");
            assert!(inc_dir.exists());

            let inc_file = inc_dir.join("inc.toml");
            assert!(inc_file.exists());
        });
    }

    #[test]
    fn checkout_no_args() {
        assert_cli::Assert::main_binary()
            .with_args(&["checkout"])
            .fails()
            .and()
            .stderr().contains("No repository specified. Review `inc checkout --help` for options.")
            .unwrap();
    }

    #[test]
    fn checkout_help() {
        assert_cli::Assert::main_binary()
            .with_args(&["checkout", "--help"])
            .succeeds()
            .and()
            .stdout().contains("Usage:
  inc-checkout [options] <repository> [<directory>]
  inc-checkout <repository> [<directory>]
  inc-checkout [options]
  inc-checkout (-h | --help)

Options:
  -s <service>, --service=<service>       Where to checkout from. A lot of cases will be github.
  -v, --verbose ...                       Increasing verbosity.
  -w, --warn                              Only display warning messages.
  -q, --quiet                             No output printed to stdout.
  -h, --help                              Prints this message.
  -l, --list                              Lists all options for service.

Args:
  <repository>    The (possibly remote) repository to clone from.
  <directory>     Clones a repository into a newly created directory.")
            .unwrap();
    }

    #[test]
    fn checkout_list_internal() {
        assert_cli::Assert::main_binary()
            .with_args(&["checkout", "--list"])
            .succeeds()
            .and()
            .stdout().contains("Services:\n - github\t[default]")
            .unwrap();
    }

    #[test]
    fn checkout_list_with_external() {
        with_test_dir(|tmp_dir| {

            let file_path = tmp_dir.clone().join("inc-checkout-service-foobar");
            let file_path = file_path.to_str().unwrap();
            let mut tmp_file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o770)
                .open(file_path)
                .unwrap();
            writeln!(tmp_file, "echo \"github.com/ethankhall/inc\"").expect("write temp file");

            let new_path = format!("{}:{}", var("PATH").unwrap(), tmp_dir.to_str().unwrap());

            assert_cli::Assert::main_binary()
            .with_args(&["-vvv", "checkout", "--list"])
            .with_env(&[("PATH", new_path)])
            .succeeds()
            .and()
            .stdout().contains("Services:\n - foobar\n - github\t[default]")
            .unwrap();
        });
    }

    #[test]
    fn checkout_from_service() {
        with_test_dir(|tmp_dir| {
            let checkout_dir = tmp_dir.clone().join("inc-checkout");

            let file_path = tmp_dir.clone().join("inc-checkout-service-foobar");
            let file_path = file_path.to_str().unwrap();
            let mut tmp_file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o770)
                .open(file_path)
                .unwrap();
            writeln!(tmp_file, "echo \"git@github.com:ethankhall/inc.git\"").expect("write temp file");

            let new_path = format!("{}:{}", var("PATH").unwrap(), tmp_dir.to_str().unwrap());

            assert_cli::Assert::main_binary()
                .with_args(&["-vvv", "checkout", "--service=foobar", "something-random", checkout_dir.to_str().unwrap()])
                .with_env(&[("PATH", new_path)])
                .succeeds()
                .unwrap();
        });
    }

    fn with_test_dir<F: Fn(PathBuf) -> ()>(exec: F) {
        let tmp_dir = TempDir::new("checkout-dir-tmp").expect("temp dir should be created");

        exec(tmp_dir.path().to_owned());

        tmp_dir.close().expect("temp dir should be closed");
    }

    fn build_exec() -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.push("target");
        path.push("debug");
        path.push("inc");

        let path = String::from(path.to_str().unwrap());

        println!("path: {}", path);
        path
    }
}