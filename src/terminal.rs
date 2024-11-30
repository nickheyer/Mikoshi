use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use nix::sys::termios::{self, InputFlags, LocalFlags};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::fs::File;

pub struct Terminal {
    shell: Arc<Mutex<Child>>,
    buffer: Arc<Mutex<Vec<u8>>>,
    tx: mpsc::Sender<Vec<u8>>,
    rx_output: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    should_exit: Arc<Mutex<bool>>,
}

impl Terminal {
    pub fn start(&self) {
        let buffer_clone = Arc::clone(&self.buffer);
        let rx_output = Arc::clone(&self.rx_output);
        let should_exit_clone = Arc::clone(&self.should_exit);

        thread::spawn(move || {
            while !*should_exit_clone.lock().unwrap() {
                if let Ok(data) = rx_output.lock().unwrap().recv() {
                    if let Ok(mut buffer) = buffer_clone.lock() {
                        buffer.extend_from_slice(&data);
                    }
                } else {
                    break; // Exit loop if channel is closed
                }
            }
        });
    }

    pub fn new() -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let (tx_input, rx_input) = mpsc::channel::<Vec<u8>>();
        let (tx_output, rx_output) = mpsc::channel::<Vec<u8>>();
        let should_exit = Arc::new(Mutex::new(false));

        let mut command = Command::new("bash");
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("TERM", "xterm-256color");

        Terminal::set_raw_mode();
        let child = command.spawn().expect("Failed to spawn shell");
        let shell = Arc::new(Mutex::new(child));

        // Configure the terminal in raw mode
        Terminal::set_raw_mode();

        let terminal = Terminal {
            shell: Arc::clone(&shell),
            buffer: Arc::clone(&buffer),
            tx: tx_input.clone(),
            rx_output: Arc::new(Mutex::new(rx_output)),
            should_exit: Arc::clone(&should_exit),
        };

        // Input thread
        {
            let shell = Arc::clone(&shell);
            let should_exit = Arc::clone(&should_exit);
            thread::spawn(move || {
                Terminal::handle_input(shell, rx_input, should_exit);
            });
        }

        // Output thread
        {
            let shell = Arc::clone(&shell);
            let should_exit = Arc::clone(&should_exit);
            thread::spawn(move || {
                Terminal::handle_output(shell, tx_output, should_exit);
            });
        }

        terminal.start();
        terminal
    }

    fn set_raw_mode() {
        let stdin = std::io::stdin();
        let mut termios = termios::tcgetattr(&stdin).unwrap();
        termios.input_flags.remove(
            termios::InputFlags::BRKINT
            | termios::InputFlags::ICRNL
            | termios::InputFlags::INPCK
            | termios::InputFlags::ISTRIP
            | termios::InputFlags::IXON,
        );
        termios.local_flags.remove(
            termios::LocalFlags::ECHO
            | termios::LocalFlags::ICANON
            | termios::LocalFlags::IEXTEN
            | termios::LocalFlags::ISIG,
        );
        termios::tcsetattr(&stdin, termios::SetArg::TCSANOW, &termios).unwrap();
    }

    fn handle_input(
        shell: Arc<Mutex<Child>>,
        rx_input: mpsc::Receiver<Vec<u8>>,
        should_exit: Arc<Mutex<bool>>,
    ) {
        let mut stdin = shell.lock().unwrap().stdin.take().unwrap();
    
        for input in rx_input.iter() {
            if *should_exit.lock().unwrap() {
                break;
            }
    
            if input == [3] { // Ctrl+C
                let pid = shell.lock().unwrap().id();
                let _ = signal::kill(Pid::from_raw(pid as i32), Signal::SIGINT);
            } else if input == [4] { // Ctrl+D
                *should_exit.lock().unwrap() = true;
                break;
            } else if input == [8] || input == [127] { // Backspace
                // Handle backspace logic to sync terminal buffer
                let _ = stdin.write_all(b"\x08");
                let _ = stdin.flush();
            } else {
                let _ = stdin.write_all(&input);
                let _ = stdin.flush();
            }
        }
    }

    fn handle_output(
        shell: Arc<Mutex<Child>>,
        tx_output: mpsc::Sender<Vec<u8>>,
        should_exit: Arc<Mutex<bool>>,
    ) {
        let stdout = shell.lock().unwrap().stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut buffer = Vec::new();

        while !*should_exit.lock().unwrap() {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if !buffer.is_empty() {
                        let _ = tx_output.send(buffer.clone());
                    }
                }
            }
        }
    }

    pub fn write_input(&self, input: &[u8]) -> Result<(), String> {
        let filtered_input: Vec<u8> = input
            .iter()
            .copied()
            .filter(|&c| c.is_ascii_graphic() || c.is_ascii_whitespace() || c == b'\x08') // Allow backspace
            .collect();

        self.tx
            .send(filtered_input)
            .map_err(|e| format!("Failed to send input: {}", e))
    }

    pub fn read_output(&self) -> Option<Vec<u8>> {
        self.rx_output.lock().unwrap().recv().ok()
    }

    pub fn should_exit(&self) -> bool {
        *self.should_exit.lock().unwrap()
    }

    pub fn get_output(&self) -> Vec<u8> {
        self.buffer.lock()
            .map(|mut buffer| {
                let output = buffer.clone();
                buffer.clear();
                output
            })
            .unwrap_or_default()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Ok(mut shell) = self.shell.lock() {
            let _ = shell.kill();
            let _ = shell.wait();
        }
    }
}
