# Project Structure

## Docs

- book

The mdbook describing the language

- docs

The compiled html book

## Crates

- graphix-compiler

The compiler for the graphix language

- graphix-stdlib

The graphix standard library implementation, both the rust and graphix parts

- graphix-rt

The graphix runtime

- graphix-shell

The graphix REPL, along with the terminal (and future graphical) gui library
implementations

# Code Review

When asked to do code review please use the following process. When you wish to
say something about a particular part of the code add a comment in the form

// CR <your-name> for <cr-adressee>: text of your comment

For example if you are claude, and you wish to tell eestokes that particular use
of unsafe is ok you might write

// CR claude for estokes: This use of unsafe does not seem safe because ...

I will then read your comment, and if I think I have addressed the problem I
will change it to an XCR. For example,

// XCR claude for estokes: This use of unsafe does not seem safe because ...

I might also add additional explanation to the comment prefixed by my name, for example

// XCR claude for estokes: This use of unsafe does not seem safe because ...
// estokes: I think it's actually safe because ...

When you write a CR. The first thing you should do is read the CR again and make
sure you really agree with what you said. Sometimes our first pass at something
turns out to be wrong on further reflection.

When I ask you to review your XCRs please read the XCR my comments, and the
code, and decide if my code change or my explanation really addresses the issue
you had. If it does, then delete the XCR. If it doesn't then turn the XCR back
into a CR and add additional comments explaining what you think is still
incorrect.

In general we keep the code quality in this library very high, even if it means
hard work. We don't take shortcuts, and we think through all the implications of
our changes very carefully. Please apply this philosophy to code review.
