use std::{
    fs::File,
    mem,
    process::{Child, Command},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use mmap_wrapper::MmapWrapper;

#[repr(C)]
struct LatestStreamInfo {
    msgs_per_15s: u64,
    msgs_per_30s: u64,
    msgs_per_60s: u64,
    raid: u64,
    follow: u64,
    redeem: u64,
}

fn murder(c: &mut Child) {
    std::process::Command::new("kill")
        .args(["--timeout", "1000", "TERM"])
        .args(["--timeout", "1000", "KILL"])
        .args(["--signal", "INT"])
        .arg(c.id().to_string())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    c.wait().unwrap();
}

fn get_latest_stream_info<P: AsRef<str>>(path: P) -> MmapWrapper<LatestStreamInfo> {
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.as_ref())
        .unwrap();

    let _ = f.set_len(mem::size_of::<LatestStreamInfo>() as u64);

    let m = unsafe { memmap2::Mmap::map(&f).unwrap() };
    MmapWrapper::new(m)
}

struct RedeemHandler {
    current_running_process: Child,
    current_redeem: Command,
    latest_info_wrapper: MmapWrapper<LatestStreamInfo>,
}

impl RedeemHandler {
    pub fn new() -> RedeemHandler {
        let latest_info_wrapper = get_latest_stream_info("/tmp/strim-mmap-test.bin");

        let default_redeem: Command = Command::new("cava");
        let current_redeem = default_redeem;

        let current_running_process = std::process::Command::new("cava").spawn().unwrap();

        RedeemHandler {
            current_running_process,
            current_redeem,
            latest_info_wrapper,
        }
    }

    fn handle(&mut self) {
        let latest_info = unsafe { self.latest_info_wrapper.get_inner() };
        let mut redeem = match latest_info.redeem {
            1 => Command::new("sl-loop"),
            2 => {
                let mut c = Command::new("hyfetch");
                c.arg("--june");
                c
            }
            3 => Command::new("cava"),
            4 => {
                let mut c = Command::new("btm");
                c.arg("-e");
                c.args(["--default_widget_type", "net"]);
                c
            }
            _ => Command::new("cava"),
        };

        if redeem.get_program() == self.current_redeem.get_program() {
            if let Err(e) = self.current_running_process.try_wait() {
                log::warn!("progrem crweshdx: {e}");
            };
        } else {
            murder(&mut self.current_running_process);
            self.current_running_process = redeem.spawn().unwrap();
            self.current_redeem = redeem;
        }
    }
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let mut redeem_handler = RedeemHandler::new();

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(300));
        redeem_handler.handle();
    }

    murder(&mut redeem_handler.current_running_process);
}
