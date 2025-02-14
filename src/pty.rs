use crate::nbio;
use anyhow::Result;
use futures::channel::oneshot;
use nix::libc;
use nix::pty;
use nix::pty::Winsize;
use std::convert::Infallible;
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::wait;
use nix::unistd::{self, ForkResult, Pid};
use std::io::pipe;
use std::io::Write;
use nix::unistd::close;
use std::env;
use std::ffi::{CString, NulError};
use std::fs::File;
use std::future::Future;
use std::io::{PipeReader, PipeWriter};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::fd::{AsRawFd, OwnedFd};
use std::{error::Error, fmt};
use tokio::io::unix::AsyncFd;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct ExecError {
    message: String
}

impl Error for ExecError {}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExecError: {}", self.message)
    }
}


fn spawn(
    command: Vec<String>,
    winsize: &pty::Winsize,
    input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    token: CancellationToken
) -> Result<impl Future<Output = Result<()>>> {

    let (pipe_in, pipe_out) = pipe()?;

    let result = unsafe { pty::forkpty(Some(winsize), None) }?;

    match result.fork_result {
        ForkResult::Parent { child } => {
            let mut reader = PipeReader::from(pipe_in);
            let mut s: String = "".to_string();
            close(pipe_out.as_raw_fd()).unwrap();
            let res = reader.read_to_string(&mut s);
            match res {
                Ok(_) => {
                    if s.is_empty() {
                        Ok(drive_child(child, result.master, input_rx, output_tx, token))
                    } else {
                        Err(ExecError{message: s}.into())
                    }
                },
                Err(e) => {
                    Err(ExecError{message: e.to_string()}.into())
                }
            }
        },

        ForkResult::Child => {
            close(pipe_in.as_raw_fd()).unwrap();
            match exec(command) {
                Err(e) => {
                    let mut writer = PipeWriter::from(pipe_out);
                    writer.write(e.to_string().as_bytes()).unwrap();
                    unsafe { libc::_exit(1) }
                }
                Ok(_) => {
                    unreachable!();
                }
            }
        }
    }
}

async fn drive_child(
    child: Pid,
    master: OwnedFd,
    input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    token: CancellationToken
) -> Result<()> {
    let result = do_drive_child(master, input_rx, output_tx, token).await;
    unsafe { libc::kill(child.as_raw(), libc::SIGHUP) };

    tokio::task::spawn_blocking(move || {
        let _ = wait::waitpid(child, None);
    }).await.unwrap();
    result
}

const READ_BUF_SIZE: usize = 128 * 1024;

async fn do_drive_child(
    master: OwnedFd,
    mut input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    token: CancellationToken
) -> Result<()> {
    let mut buf = [0u8; READ_BUF_SIZE];
    let mut input: Vec<u8> = Vec::with_capacity(READ_BUF_SIZE);
    nbio::set_non_blocking(&master.as_raw_fd())?;
    let mut master_file = unsafe { File::from_raw_fd(master.as_raw_fd()) };
    let master_fd = AsyncFd::new(master)?;

    loop {
        tokio::select! {
            result = input_rx.recv() => {
                match result {
                    Some(data) => {
                        input.extend_from_slice(&data);
                    }

                    None => {
                        return Ok(());
                    }
                }
            }

            result = master_fd.readable() => {
                let mut guard = result?;

                loop {
                    match nbio::read(&mut master_file, &mut buf)? {
                        Some(0) => {
                            return Ok(());
                        }

                        Some(n) => {
                            output_tx.send(buf[0..n].to_vec()).await?;
                        }

                        None => {
                            guard.clear_ready();
                            break;
                        }
                    }
                }
            }

            result = master_fd.writable(), if !input.is_empty() => {
                let mut guard = result?;
                let mut buf: &[u8] = input.as_ref();

                loop {
                    match nbio::write(&mut master_file, buf)? {
                        Some(0) => {
                            return Ok(());
                        }

                        Some(n) => {
                            buf = &buf[n..];

                            if buf.is_empty() {
                                break;
                            }
                        }

                        None => {
                            guard.clear_ready();
                            break;
                        }
                    }
                }

                let left = buf.len();

                if left == 0 {
                    input.clear();
                } else {
                    input.drain(..input.len() - left);
                }
            }

            _ = token.cancelled() => {
                break;
            }
        }
    }

    Ok(())
}

fn exec(command: Vec<String>) -> Result<Infallible> {
    let command = command.iter()
    .map(|s| CString::new(s.as_bytes()))
    .collect::<Result<Vec<CString>, NulError>>()?;
    env::set_var("TERM", "xterm-256color");
    unsafe { signal::signal(Signal::SIGPIPE, SigHandler::SigDfl) }?;
    Ok(unistd::execvp(&command[0], &command)?)
}


pub async fn run_pty(
    command: Vec<String>,
    cols: usize,
    rows: usize,
    input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    start_tx: oneshot::Sender<Result<()>>,
    token: CancellationToken,
) -> Result<()> {
    let winsize = Winsize {
        ws_col: u16::try_from(cols).unwrap(),
        ws_row: u16::try_from(rows).unwrap(),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let outcome = spawn(command, &winsize, input_rx, output_tx, token);
    match outcome {
        Ok(f) => {
            start_tx.send(Ok(())).unwrap();
            tokio::spawn(f).await?
        }
        Err(e) => {
            start_tx.send(Err(e)).unwrap();
            Ok(())
        }
    }
}