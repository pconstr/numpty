#!/usr/bin/env python

from numpty import Terminal
import time

def main():
    cols, rows = 80, 40

    with Terminal(["ninvaders"], cols, rows) as term:

        wait_first = 1000
        wait_more = 5

        term.settle(wait_first, wait_more)
        print(term.text())

        term.keys([" "])
        term.settle(wait_first, wait_more)
        print(term.text())

        term.keys(["Space"])
        term.settle(wait_first, wait_more)
        print(term.text())

        for _ in range(20):
            term.keys(["Right"])
            term.settle(wait_first, wait_more)
            print(term.render())

 

if __name__ == "__main__":
    main()
