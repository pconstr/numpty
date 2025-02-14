#![feature(anonymous_pipe)]
//!
//! NumPy interface to a child process running in a headless pseudoterminal (pty)
//!
//! `NumPty` runs a process and connects it to a headless pseudoterminal through which the output
//! can be examined and the input controlled. Snapshots of the terminal contents can be captured
//! and represented as [NumPy](https://numpy.org/) character code point and color matrices for convenient processing.
//!

mod color;
mod keys;
mod lines;
mod nbio;
mod protocol;
mod pty;
mod term;

use lines::chars_from_lines;
use lines::indexedcolor_from_lines;
use lines::render_lines;
use lines::truecolor_from_lines;
use protocol::Req;
use pty::run_pty;
use term::run_term;

use anyhow::{anyhow, Result};
use keys::InputSeq;
use numpy::{PyArray2, PyArray3};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use futures::channel::oneshot;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::PyAny;
use tokio::time::Duration;



/// A child process running in a headless pseudo-terminal
#[pyclass]
pub struct Terminal {
    command: Vec<String>,
    rows: usize,
    cols: usize,
    rt: Runtime,
    input_tx: Option<mpsc::Sender<Vec<u8>>>,
    req_tx: Option<mpsc::Sender<Req>>,
    token: Option<CancellationToken>,
    lines: Option<Vec<avt::Line>>,
}

impl Terminal {
    fn do_stop(&mut self) {
        if let Some(token) = &self.token {
            token.cancel();
        }
    }

    fn do_start(slf: &mut Self) -> Result<()> {
        let (input_tx, input_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel(1024);
        let (output_tx, output_rx) = mpsc::channel(1024);
        let (req_tx, req_rx) = mpsc::channel(1);
        let (start_tx, start_rx) = oneshot::channel();

        let token = CancellationToken::new();

        slf.rt.spawn(run_pty(
            slf.command.clone(),
            slf.cols,
            slf.rows,
            input_rx,
            output_tx,
            start_tx,
            token.clone(),
        ));

        slf.rt.spawn(run_term(
            slf.cols,
            slf.rows,
            output_rx,
            req_rx,
            token.clone(),
        ));

        slf.input_tx = Some(input_tx);
        slf.req_tx = Some(req_tx);

        slf.rt.block_on(async {
            let outcome = start_rx.await;
            match outcome {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow!("could not communicate")),
            }
        })
    }
}

#[pymethods]
impl Terminal {
    /// Create a Terminal with `cols` and `rows` to run `command`
    /// The subprocess is not started until either `start` is called
    /// or the runtime context is enter - if Terminal is used as a context manager.
    #[new]
    pub fn py_new(command: Vec<String>, cols: usize, rows: usize) -> PyResult<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(3)
            .enable_all()
            .build()?;

        Ok(Terminal {
            command: command,
            rows: rows,
            cols: cols,
            rt: rt,
            input_tx: None,
            req_tx: None,
            token: None,
            lines: None,
        })
    }

    /// Start the subprocess by running the command specified creating the Terminal
    pub fn start(&mut self) -> PyResult<()> {
        if !self.req_tx.is_none() {
            return Err(PyValueError::new_err("already started"));
        };
        let outcome = Terminal::do_start(self);
        outcome.map_err(|e| PyOSError::new_err(e.to_string()))
    }

    #[pyo3(name = "__enter__")]
    pub fn enter<'a>(mut slf: PyRefMut<'a, Self>, _py: Python) -> PyResult<PyRefMut<'a, Self>> {
        if !slf.req_tx.is_none() {
            return Err(PyValueError::new_err("already started"));
        };
        match Terminal::do_start(&mut slf) {
            Ok(_) => Ok(slf),
            Err(e) => Err(PyOSError::new_err(e.to_string())),
        }
    }

    #[pyo3(name = "__exit__")]
    pub fn exit(
        &mut self,
        _py: Python,
        _exception_type: Py<PyAny>,
        _exception_value: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> bool {
        self.do_stop();
        false
    }


    /// Wait for output to the Terminal to settle and then capture the terminal's context in a snapshot.
    /// This happens using a simple heuristic.
    /// First wait for at most `wait_first` ms for some output to arrive. If none arrives give up, not taking any snapshot.
    /// If some output arrives then wait repeatedly until `wait_more` ms have passed without any additional output.
    /// At that point the terminal is considered "settled" and a snapshot is taken replacing the previous one.
    pub fn settle(&mut self, wait_first: u64, wait_more: u64) -> PyResult<()> {
        let Some(ref req_tx) = self.req_tx else {
            return Err(PyValueError::new_err("not started"));
        };
        let wait_first = Duration::from_millis(wait_first);
        let wait_more = Duration::from_millis(wait_more);
        self.rt.block_on(async {
            let (reply_tx, reply_rx) = oneshot::channel();
            let req = Req {
                reply: reply_tx,
                wait_first: wait_first,
                wait_more: wait_more,
            };
            req_tx
                .send(req)
                .await
                .map_err(|e| PyOSError::new_err(e.to_string()))?;
            let reply = reply_rx
                .await
                .map_err(|e| PyOSError::new_err(e.to_string()))?;
            // don't really care about terminal if there was a launch
            if let Some(e) = reply.error {
                return Err(PyOSError::new_err(e));
            }
            self.lines = Some(reply.lines);
            Ok(())
        })
    }

    /// Retrieves a _rows_ x _cols_ `u32` matrix of UCS-4 (unicode) code points.
    pub fn chars<'py>(&self, _py: Python<'py>) -> Option<Bound<'py, PyArray2<u32>>> {
        self.lines.as_ref()
            .map(|l| chars_from_lines(&l))
            .map(|a|PyArray2::from_owned_array(_py, a))
    }

    /// Retrieves a tuple with a _rows_ x _cols_ `u8` matrix of background colors (0 if default)
    /// and a corresponding mask (bool) matrix where an element is True if the color is not the default.
    /// No attempt is made to convert truecolor codes to indexed colors.
    pub fn foreground_indexedcolor<'py>(
        &self,
        _py: Python<'py>,
    ) -> Option<(Bound<'py, PyArray2<u8>>, Bound<'py, PyArray2<bool>>)> {
        self.lines.as_ref()
            .map(|l| indexedcolor_from_lines(l, |pen| pen.foreground()))
            .map(|(fga, fgma)| (
                PyArray2::from_owned_array(_py, fga),
                PyArray2::from_owned_array(_py, fgma)
            ))
    }

    /// Retrieves a tuple with a 3 x rows_ x _cols_ `u8` matrix of foreground colors ((0,0,0) if default)
    /// and a corresponding mask.
    /// Indexed colors are converted to truecolor using an inbuilt palette.
    pub fn foreground_truecolor<'py>(
        &self,
        _py: Python<'py>,
    ) -> Option<(Bound<'py, PyArray3<u8>>, Bound<'py, PyArray2<bool>>)> {
        self.lines.as_ref()
            .map(|l| truecolor_from_lines(l, |pen| pen.foreground()))
            .map(|(fga, fgma)| (
                PyArray3::from_owned_array(_py, fga),
                PyArray2::from_owned_array(_py, fgma)
            ))
    }

    /// Retrieves a tuple with a _rows_ x _cols_ `u8` matrix of foreground colors (0 if default)
    /// and a corresponding mask (bool) matrix where an element is True if the color is not the default.
    /// No attempt is made to convert truecolor codes to indexed colors.
    pub fn background_indexedcolor<'py>(
        &self,
        _py: Python<'py>,
    ) -> Option<(Bound<'py, PyArray2<u8>>, Bound<'py, PyArray2<bool>>)> {
        self.lines.as_ref()
            .map(|l| indexedcolor_from_lines(l, |pen| pen.background()))
            .map(|(fga, fgma)| (
                PyArray2::from_owned_array(_py, fga),
                PyArray2::from_owned_array(_py, fgma)
            ))
    }

    /// Retrieves a tuple with a 3 x rows_ x _cols_ `u8` matrix of background colors ((0,0,0) if default)
    /// and a corresponding mask.
    /// Indexed colors are converted to truecolor using an inbuilt palette.
    pub fn background_truecolor<'py>(
        &self,
        _py: Python<'py>,
    ) -> Option<(Bound<'py, PyArray3<u8>>, Bound<'py, PyArray2<bool>>)> {
        self.lines.as_ref()
            .map(|l| truecolor_from_lines(l, |pen| pen.background()))
            .map(|(fga, fgma)| (
                PyArray3::from_owned_array(_py, fga),
                PyArray2::from_owned_array(_py, fgma)
            ))
    }

    /// Retrieves a text string with the text context of the snapshot, lines terminated by `\n`
    pub fn text(&self) -> PyResult<String> {
        match &self.lines {
            Some(lines) => {
                let rendered = lines
                    .iter()
                    .map(|l| l.text())
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(rendered)
            }
            None => Ok("".to_string()),
        }
    }

    /// Like `text()` but with foreground and background coloring.
    pub fn render(&self) -> Option<String> {
        self.lines.as_ref().map(render_lines)
    }

    /// Send an input string to the controlled process.
    pub fn input(&mut self, input: String) -> PyResult<()> {
        let Some(ref input_tx) = self.input_tx else {
            return Err(PyValueError::new_err("not started"));
        };

        let sent = self.rt.block_on(async {
            let seq = keys::InputSeq::Standard(input);
            // is the cursor always in this mode as the Vt is created?
            let cursor_key_app_mode = true;
            let seqs = vec![seq]; 
            let data = keys::seqs_to_bytes(&seqs, cursor_key_app_mode);
            input_tx.send(data).await
        });
        sent.map_err(|e| PyOSError::new_err(e.to_string()))
    }


    /// Send input to the controlled process, through the terminal.
    /// Each element of the array can be either a key name or an arbitrary text.
    /// If a key is not matched by any supported key name then the text is sent to the
    /// process as is, i.e. like when using `input()`.
    /// 
    /// The key and modifier specifications were inspired by
    /// [tmux](https://github.com/tmux/tmux/wiki/Modifier-Keys).
    /// 
    /// The following key specifications are currently supported:
    /// 
    /// - `Enter`
    /// - `Space`
    /// - `Escape` or `^[` or `C-[`
    /// - `Tab`
    /// - `Left` - left arrow key
    /// - `Right` - right arrow key
    /// - `Up` - up arrow key
    /// - `Down` - down arrow key
    /// - `Home`
    /// - `End`
    /// - `PageUp`
    /// - `PageDown`
    /// - `F1` to `F12`
    /// 
    /// Modifier keys are supported by prepending a key with one of the prefixes:
    /// 
    /// - `^` - control - e.g. `^c` means <kbd>Ctrl</kbd> + <kbd>C</kbd>
    /// - `C-` - control - e.g. `C-c` means <kbd>Ctrl</kbd> + <kbd>C</kbd>
    /// - `S-` - shift - e.g. `S-F6` means <kbd>Shift</kbd> + <kbd>F6</kbd>
    /// - `A-` - alt/option - e.g. `A-Home` means <kbd>Alt</kbd> + <kbd>Home</kbd>
    /// 
    /// Modifiers can be combined (for arrow keys only at the moment), so combinations
    /// such as `S-A-Up` or `C-S-Left` are possible.
    /// 
    /// `C-` control modifier notation can be used with ASCII letters (both lower and
    /// upper case are supported) and most special key names. The caret control notation
    /// (`^`) may only be used with ASCII letters, not with special keys.
    /// 
    /// Shift modifier can be used with special key names only, such as `Left`, `PageUp`
    /// etc. For text characters, instead of specifying e.g. `S-a` just use upper case
    /// `A`.
    /// 
    /// Alt modifiers can be used with any Unicode character and most special key names.
    pub fn keys(&mut self, keys: Vec<String>) -> PyResult<()> {
        let Some(ref input_tx) = self.input_tx else {
            return Err(PyValueError::new_err("not started"));
        };

        let sent = self.rt.block_on(async {
            let seqs: Vec<InputSeq> = keys.into_iter().map(keys::parse_key).collect();
            // is the cursor always in this mode as the Vt is created?
            let cursor_key_app_mode = true;
            let data = keys::seqs_to_bytes(&seqs, cursor_key_app_mode);
            input_tx.send(data).await
        });
        sent.map_err(|e| PyOSError::new_err(e.to_string()))
    }

    pub fn stop(&mut self) -> PyResult<()> {
        if self.input_tx.is_none() {
            return Err(PyValueError::new_err("not started"));
        };
        self.do_stop();
        Ok(())
    }
}

#[pymodule]
fn numpty(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Terminal>()?;
    Ok(())
}
