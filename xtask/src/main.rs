#[macro_use]
extern crate clap;

use clap::Parser;
use once_cell::sync::Lazy;
use os_xtask_utils::{BinUtil, Cargo, CommandExt, Qemu};
use std::{
    fs,
    path::{Path, PathBuf},
};

static PROJECT: Lazy<&'static Path> =
    Lazy::new(|| Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap());

#[derive(Parser)]
#[clap(name = "perf-playground")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Make(BuildArgs),
    Asm(BuildArgs),
    Guest(BuildArgs),
    Qemu(BuildArgs),
}

fn main() {
    use Commands::*;
    match Cli::parse().command {
        Make(args) => {
            args.make(false);
        }
        Asm(args) => args.asm(),
        Guest(args) => args.guest(),
        Qemu(args) => args.qemu(),
    }
}

#[derive(Args, Default)]
struct BuildArgs {
    /// app
    #[clap(long)]
    app: String,
    /// platform
    #[clap(long, short)]
    plat: String,
    /// log level
    #[clap(long)]
    log: Option<String>,
    #[clap(long)]
    gdb: Option<u16>,
}

impl BuildArgs {
    fn path_exist(path: &str) -> bool {
        fs::metadata(path).is_ok()
    }

    fn make(&self, is_std: bool) -> PathBuf {
        assert!(
            Self::path_exist(PROJECT.join("platforms").join(&self.plat).to_str().unwrap()),
            "{}",
            format!("platform {0} not exist", self.plat)
        );
        assert!(
            Self::path_exist(PROJECT.join("apps").join(&self.app).to_str().unwrap()),
            "{}",
            format!("app {0} not exist", self.app)
        );

        fs::write(
            PROJECT.join("obj").join("Cargo.toml"),
            format!(
                "\
[package]
name = \"obj\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
app = {{ path = \"../apps/{0}\", package = \"{0}\" }}
platform = {{ path = \"../platforms/{1}\", package = \"{1}\" }}
stdio = {{ path = \"../common/stdio\" }}
executor = {{ path = \"../common/executor\" }}
thread = {{ path = \"../libs/thread\" }}
net = {{ path = \"../libs/net\" }}
mem = {{ path = \"../common/mem\", features = [{2}] }}
timer = {{ path = \"../common/timer\", features = [] }}

[build-dependencies]
linker = {{ path = \"../platforms/{1}-ld\", package = \"{1}-ld\" }}

[features]
default = [{2}]
std = []
",
                self.app,
                self.plat,
                if is_std { "\"std\"" } else { "" },
            ),
        )
        .unwrap();
        let build_tool: &str = match is_std {
            true => "x86_64-apple-darwin",
            false => "riscv64gc-unknown-none-elf",
        };
        Cargo::build()
            .package("obj")
            .optional(&self.log, |cargo, level| {
                cargo.env("LOG", level);
            })
            .release()
            .target(build_tool)
            .invoke();
        PROJECT.join("target").join(build_tool)
    }

    fn asm(&self) {
        let target = self.make(false);
        let elf = target.join("release").join("obj");
        let out = PROJECT.join("kernel.asm");
        fs::write(out, BinUtil::objdump().arg(elf).arg("-d").output().stdout).unwrap();
    }

    fn guest(&self) {
        use std::process::Command;
        let target = self.make(true);
        let elf = target.join("release").join("obj");
        let mut command = Command::new(elf);
        let status = command.status().expect("guest failed");
        assert!(status.success());
    }

    fn qemu(&self) {
        let target = self.make(false);
        let elf = target.join("release").join("obj");
        Qemu::system("riscv64")
            .args(["-machine", self.plat.strip_prefix("qemu-").unwrap()])
            .arg("-kernel")
            .arg(objcopy(elf, true))
            .args(["-m", "3G"])
            .args(["-smp", "1"])
            .arg("-nographic")
            .args([
                "-netdev",
                "user,id=net0,hostfwd=tcp::6000-:6000,hostfwd=tcp::6001-:6001",
            ])
            .args([
                "-object",
                "filter-dump,id=net0,netdev=net0,file=/Users/jackzhang/packets.pcap",
            ])
            .args(["-device", "e1000,netdev=net0,bus=pcie.0"])
            .optional(&self.gdb, |qemu, gdb| {
                qemu.args(["-S", "-gdb", &format!("tcp::{gdb}")]);
            })
            .invoke();
    }
}

fn objcopy(elf: impl AsRef<Path>, binary: bool) -> PathBuf {
    let elf = elf.as_ref();
    let bin = elf.with_extension("bin");
    BinUtil::objcopy()
        .arg(elf)
        .arg("--strip-all")
        .conditional(binary, |binutil| {
            binutil.args(["-O", "binary"]);
        })
        .arg(&bin)
        .invoke();
    bin
}
