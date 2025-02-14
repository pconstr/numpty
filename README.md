# NumPty - NumPy interface to a child process running in a headless pseudoterminal (pty)

`NumPty` runs a process and connects it to a headless pseudoterminal through which the output can be examined and the input controlled. Snapshots of the terminal contents can be captured and represented as [NumPy](https://numpy.org/) character code point and color matrices for convenient processing.

`NumPty` is written in Rust and is based on code (mostly `nbio.rs`, `pty.rs` and `keys.rs`) from [andyk/ht](https://github.com/andyk/ht), with different ergonomics.
It is meant to be used in Python programs through its [pyo3](https://pyo3.rs) bindings. Like `ht`, `NumPty` uses [asciinema/avt](https://github.com/asciinema/avt) to emulate the terminal in RAM.


`NumPty` is work-in-progress, unstable and subject to change.


# Use Cases

Use `NumPty` to control programs with terminal user interfaces, that is programs that interact with the user by drawing with text characters on the terminal.


# Installation

`pip install numpty`

No wheels yet, so it needs the Rust compiler and Cargo to build from source.

Or clone the repo and `pip install .`


# Usage


Here's an example of controlling `nudoku` (`apt install nudoku` on Ubuntu):


```python
from numpty import Terminal

def main():
    cols, rows = 60, 22
    with Terminal(["nudoku", "-d", "hard"], cols, rows) as term:

        wait_first = 1000
        wait_more = 100

        term.settle(wait_first, wait_more)
        print(term.render())
        print(term.chars())

        term.keys(["S"])
        term.settle(wait_first, wait_more)
        print(term.render())

        chars = term.chars()
        print(chars)
        foreground, foreground_mask = term.foreground_indexedcolor()
        assert foreground.shape == (rows, cols)
        assert foreground_mask.shape == (rows, cols)

        print(foreground)
        print(foreground_mask)

        foreground2, foreground_mask2 = term.foreground_truecolor()
        assert foreground2.shape == (3, rows, cols)
        assert foreground_mask2.shape == (rows, cols)
        assert (foreground_mask2 == foreground_mask).all()

        term.keys(["Q"])


if __name__ == "__main__":
    main()
```

`Terminal()` starts the specified program as a child and terminates it on exit.


## Settling

`settle(wait_first, wait_more)` is used to wait for a good moment to capture a snapshot of the terminal.
It will first wait for up to `wait_first` milliseconds to detect a first update to the terminal.
If more than that time passes it will give up without capturing a new snapshot.
After the first update it will repeatedly wait for up to `wait_more` milliseconds to detect subsequent updates,
restarting the timer every time more output is detected.
Execution is blocked until no output is detected for `wait_more` milliseconds.

At that point the terminal is considered "settled" and a snapshot is made replacing the previous one.


## Accessing the snapshot

The most recent snapshot can then be accessed as NumPy matrices using any of these methods:

* `chars()` retrieves a _rows_ x _cols_ `u32` matrix of UCS-4 (unicode) code points.
* `foreground_indexedcolor()` retrieves a tuple with a _rows_ x _cols_ `u8` matrix of foreground colors (0 if default) and a corresponding mask (bool) matrix where an element is True if the color is not the default.
* `background_indexedcolor()` is analogous to `foreground_indexedcolor` but for the background. In both cases no attempt is made to convert truecolor codes to indexed colors.
* `foreground_truecolor()` retrieves a tuple with a 3 x rows_ x _cols_ `u8` matrix of foreground colors ((0,0,0) if default) and a corresponding mask.
* `background_truecolor()` is analogous to `foreground_truecolor` but for the background. In both cases indexed colors are converted to truecolor using an inbuilt palette.

There are also a couple of methods to get the snapshot as strings:

* `text()` retrieves a text string with the text context of the snapshot, lines terminated by `\n`
* `render()` is like `text()` but with foreground and background coloring.


## Sending input

`input(str)` is used to send an input string to the controlled process.

`keys([str,...])` is used to send input to the controlled process.

Each element of the array can be either a key name or an arbitrary text.
If a key is not matched by any supported key name then the text is sent to the
process as is, i.e. like when using `input()`.

The key and modifier specifications were inspired by
[tmux](https://github.com/tmux/tmux/wiki/Modifier-Keys).

The following key specifications are currently supported:

- `Enter`
- `Space`
- `Escape` or `^[` or `C-[`
- `Tab`
- `Left` - left arrow key
- `Right` - right arrow key
- `Up` - up arrow key
- `Down` - down arrow key
- `Home`
- `End`
- `PageUp`
- `PageDown`
- `F1` to `F12`

Modifier keys are supported by prepending a key with one of the prefixes:

- `^` - control - e.g. `^c` means <kbd>Ctrl</kbd> + <kbd>C</kbd>
- `C-` - control - e.g. `C-c` means <kbd>Ctrl</kbd> + <kbd>C</kbd>
- `S-` - shift - e.g. `S-F6` means <kbd>Shift</kbd> + <kbd>F6</kbd>
- `A-` - alt/option - e.g. `A-Home` means <kbd>Alt</kbd> + <kbd>Home</kbd>

Modifiers can be combined (for arrow keys only at the moment), so combinations
such as `S-A-Up` or `C-S-Left` are possible.

`C-` control modifier notation can be used with ASCII letters (both lower and
upper case are supported) and most special key names. The caret control notation
(`^`) may only be used with ASCII letters, not with special keys.

Shift modifier can be used with special key names only, such as `Left`, `PageUp`
etc. For text characters, instead of specifying e.g. `S-a` just use upper case
`A`.

Alt modifiers can be used with any Unicode character and most special key names.


# License

All code is licensed under the Apache License, Version 2.0. See LICENSE file for
details.
