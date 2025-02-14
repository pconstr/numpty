#!/usr/bin/env python

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
