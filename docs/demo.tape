# Terminal recording for the README. Regenerate after a release with:
#   cargo build --release && vhs docs/demo.tape
# Needs vhs (brew install vhs). Output: docs/demo.gif

Output docs/demo.gif

Set Shell bash
Set FontFamily "Fira Code"
Set FontSize 17
Set Width 1200
Set Height 860
Set Padding 24
Set Theme { "background": "#0b0c0e", "foreground": "#f2efe8", "cursor": "#ffb454", "black": "#15181c", "red": "#ff6b6b", "green": "#8fd18f", "yellow": "#ffb454", "blue": "#7fd3f7", "magenta": "#d7a6ff", "cyan": "#7fd3f7", "white": "#f2efe8", "brightBlack": "#7d7a73", "brightRed": "#ff8a8a", "brightGreen": "#a8e0a8", "brightYellow": "#ffcb7f", "brightBlue": "#a5e2ff", "brightMagenta": "#e6c3ff", "brightCyan": "#a5e2ff", "brightWhite": "#ffffff" }
Set TypingSpeed 60ms
Set PlaybackSpeed 1.0

Hide
Type "source docs/demo/setup.sh"
Enter
Sleep 500ms
Show

Sleep 800ms
Type "rhow"
Sleep 400ms
Enter
Sleep 4s

Type "clear"
Enter
Type "rhow --group"
Sleep 400ms
Enter
Sleep 4s

Type "clear"
Enter
Type "rhow --ci"
Sleep 400ms
Enter
Sleep 4s
