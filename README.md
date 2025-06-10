# clippy-clippy

📎 for your 📋.

Sends images on your clipboard to a vision AI model for transcription into
text.

## how to use

1. Compile it (an exercise for the reader, but having [devbox](https://www.jetify.com/devbox)
   and looking in the [justfile](justfile) would help)
2. Install it (in WSL I alias `clippy-clippy` to the windows executable)
3. Run it once to see if everything's working with your PATH.
   It'll output some information about how to configure,
   and the default config file is easy to understand if you've ever worked
   with an OpenAI compatible API
4. Copy an image onto your clipboard that has some text in it
5. Run it, it'll output the text

   If you want to copy that text back to your clipboard you can overwrite it
   with something like `clippy-clippy | pbcopy` on macos or
   `clippy-clippy | clip.exe` on windows.

   You might want to alias that to something,
   but I wanted `clippy-clippy` default functionality to not be "destructive"
   to what is on your clipboard. 
6. Try running it with the `-m` or `--markdown` flag to output github flavored
   markdown.  It's super nice for tables and things like that.

## vibecoded

Use at your own risk, this was almost entirely vibe-coded.
I had to tweak a bunch of things to get it to work on macos,
but after that was done it compiled for windows on WSL first try 🙀.
I made several adjustments to the system prompt and modified the flags a bit,
but it all seems to work well.

I don't know rust very well, and I like learning programming languages by
modifying existing codebases.
This is also a tool I'll likely use frequently,
which increases the probability that I'll be delving into the source and
making adjustments.

## improvements

The configuration system is ALMOST what I want, but works well enough.
It's currently totally usable as a tool,
so I'll use it for a while before I decide what other improvements it might need.
