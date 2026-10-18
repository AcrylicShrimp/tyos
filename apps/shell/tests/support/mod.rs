use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const PROMPT: &[u8] = b"(shell)> ";
const OUTPUT_TIMEOUT: Duration = Duration::from_secs(10);
const INPUT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(2);

struct Input {
    bytes: Vec<u8>,
    done: Sender<io::Result<()>>,
}

pub struct Qemu {
    child: Child,
    input: Option<Sender<Input>>,
    writer: Option<JoinHandle<()>>,
    serial: File,
    output: Vec<u8>,
    log_path: PathBuf,
}

impl Qemu {
    pub fn start() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let kernel = root.join("target/riscv64gc-unknown-none-elf/debug/tyos");
        Self::start_with_kernel(&kernel, PROMPT)
    }

    pub fn start_with_kernel(kernel: &Path, ready: &[u8]) -> Self {
        assert!(
            kernel.is_file(),
            "build the apps and kernel first; see qemu_smoke.rs"
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let log_dir =
            std::env::temp_dir().join(format!("tyos-shell-smoke-{}-{stamp}", std::process::id()));
        fs::create_dir(&log_dir).expect("create UART log directory");
        let log_path = log_dir.join("serial.log");
        let log = File::create(&log_path).expect("create UART log");
        let serial = File::open(&log_path).expect("open UART log for reading");
        eprintln!("UART log: {}", log_path.display());

        let mut command = Command::new("qemu-system-riscv64");
        if std::env::var_os("TYOS_QEMU_TRACE").is_some() {
            command
                .args(["-d", "int", "-D"])
                .arg(log_dir.join("traps.log"));
        }
        let child = command
            .args([
                "-machine",
                "virt",
                "-nographic",
                "-monitor",
                "none",
                "-bios",
                "default",
                "-kernel",
            ])
            .arg(kernel)
            .stdin(Stdio::piped())
            .stdout(log.try_clone().expect("clone UART log"))
            .stderr(log)
            .spawn()
            .expect("start qemu-system-riscv64 (install QEMU and put it on PATH)");

        let mut qemu = Self {
            child,
            input: None,
            writer: None,
            serial,
            output: Vec::new(),
            log_path,
        };
        let mut stdin = qemu.child.stdin.take().expect("QEMU stdin");
        let (tx, rx) = mpsc::channel::<Input>();
        // Pipe writes may block if the guest stops reading. Keep them off the
        // test thread so every send can time out; Drop kills QEMU to unblock it.
        qemu.writer = Some(thread::spawn(move || {
            for input in rx {
                let result = stdin.write_all(&input.bytes).and_then(|()| stdin.flush());
                let failed = result.is_err();
                let _ = input.done.send(result);
                if failed {
                    break;
                }
            }
        }));
        qemu.input = Some(tx);
        qemu.expect_output("guest ready", ready);
        qemu
    }

    pub fn expect_output(&mut self, label: &str, marker: &[u8]) {
        self.wait_for(label, |output| contains(output, marker));
        self.require_no_fault();
    }

    pub fn release_guest(&mut self, byte: u8) {
        self.send(&[byte], 1);
    }

    pub fn observe_quiet(&mut self, duration: Duration, forbidden: &[u8]) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.refresh();
            self.require_running();
            self.require_no_fault();
            if contains(&self.output, forbidden) {
                self.fail("guest continued before its UART gate was released");
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn require_no_fault(&self) {
        if contains(&self.output, b"PANIC") || contains(&self.output, b"page fault") {
            self.fail("unexpected panic or page fault");
        }
    }

    pub fn check(
        &mut self,
        label: &str,
        payload: &[u8],
        expected: &[&[u8]],
        prompts: usize,
        chunk: usize,
    ) {
        self.refresh();
        let start = self.output.len();
        self.send(payload, chunk);
        self.wait_for(label, |output| {
            occurrences(&output[start..], PROMPT) >= prompts
        });
        let actual = &self.output[start..];
        for marker in expected {
            if !contains(actual, marker) {
                self.fail(&format!("{label}: missing {}-byte output", marker.len()));
            }
        }
        if occurrences(actual, PROMPT) != prompts {
            self.fail(&format!("{label}: unexpected extra prompt"));
        }
        println!("PASS {label}");
    }

    pub fn exit_shell(&mut self) {
        self.refresh();
        let start = self.output.len();
        self.send(b"exit 7\n", 64);
        self.wait_for("shell exit", |output| {
            contains(&output[start..], b"bye\r\n")
        });
        // Observe a short idle period after exit, rather than just the 'bye'.
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            self.refresh();
            self.require_running();
            thread::sleep(POLL_INTERVAL);
        }
        if contains(&self.output[start..], PROMPT) {
            self.fail("shell continued after exit");
        }
        if contains(&self.output, b"PANIC") || contains(&self.output, b"page fault") {
            self.fail("unexpected panic or page fault");
        }
        println!("PASS exit without kernel panic");
    }

    pub fn poweroff_shell(&mut self) {
        self.refresh();
        let start = self.output.len();
        self.send_input(b"poweroff\n", 64, true);
        let deadline = Instant::now() + OUTPUT_TIMEOUT;
        loop {
            self.refresh();
            self.require_no_fault();
            if let Some(status) = self.child.try_wait().expect("check poweroff status") {
                if !status.success() {
                    self.fail(&format!("QEMU poweroff failed: {status}"));
                }
                if !contains(&self.output[start..], b"powering off\r\n") {
                    self.fail("QEMU exited before the shell requested poweroff");
                }
                if contains(&self.output[start..], PROMPT) {
                    self.fail("shell continued after poweroff");
                }
                println!("PASS poweroff terminates QEMU successfully");
                return;
            }
            if Instant::now() >= deadline {
                self.fail("timed out waiting for poweroff to terminate QEMU");
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn send(&mut self, payload: &[u8], chunk: usize) {
        self.send_input(payload, chunk, false);
    }

    fn send_input(&mut self, payload: &[u8], chunk: usize, expect_exit: bool) {
        assert!(chunk > 0);
        let deadline = Instant::now() + INPUT_TIMEOUT;
        for bytes in payload.chunks(chunk) {
            let (done, result) = mpsc::channel();
            self.input
                .as_ref()
                .unwrap()
                .send(Input {
                    bytes: bytes.to_vec(),
                    done,
                })
                .unwrap_or_else(|err| self.fail(&format!("stdin writer stopped: {err}")));
            let remaining = deadline.saturating_duration_since(Instant::now());
            match result.recv_timeout(remaining) {
                Ok(Ok(())) => {}
                Ok(Err(err)) => self.fail(&format!("write QEMU stdin: {err}")),
                Err(err) => self.fail(&format!(
                    "timed out or disconnected while sending input: {err}"
                )),
            }
            self.refresh();
            if !expect_exit {
                self.require_running();
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn wait_for(&mut self, label: &str, ready: impl Fn(&[u8]) -> bool) {
        let deadline = Instant::now() + OUTPUT_TIMEOUT;
        loop {
            self.refresh();
            self.require_no_fault();
            if ready(&self.output) {
                return;
            }
            self.require_running();
            if Instant::now() >= deadline {
                self.fail(&format!("timed out waiting for {label}"));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn refresh(&mut self) {
        // A regular file avoids filling QEMU's stdout pipe while stdin is sent.
        // Bound each read so a noisy guest cannot monopolize the timeout loop.
        (&mut self.serial)
            .take(65536)
            .read_to_end(&mut self.output)
            .expect("read UART log");
    }

    fn require_running(&mut self) {
        if let Some(status) = self.child.try_wait().expect("check QEMU status") {
            self.fail(&format!("QEMU exited unexpectedly: {status}"));
        }
    }

    fn fail(&self, message: &str) -> ! {
        let tail = &self.output[self.output.len().saturating_sub(1000)..];
        panic!(
            "{message}\nUART log: {}\n{}",
            self.log_path.display(),
            String::from_utf8_lossy(tail)
        );
    }
}

impl Drop for Qemu {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        eprintln!("UART log saved: {}", self.log_path.display());
    }
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn occurrences(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .filter(|&window| window == needle)
        .count()
}
