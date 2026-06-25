Output demos/9_chords.gif
Set FontSize 18
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Macchiato"

Hide
Type "cargo run -- demos/9_chords.json"
Enter
Sleep 1s
Show

# Start at e string, col 0
Type "kkkk"
Sleep 200ms

# Hit 'i' to insert
Type "i"
Sleep 300ms

# Top note
Type "0"
Sleep 100ms
Enter
Sleep 200ms

# Middle string hammer-ons and pull-offs
Type "2"
Sleep 100ms
Type "h"
Sleep 100ms
Type "3"
Sleep 100ms
Type "p"
Sleep 100ms
Type "2"
Sleep 100ms
Type "h"
Sleep 100ms
Type "3"
Sleep 100ms
Enter
Sleep 200ms

# Bottom string hammer-ons and pull-offs
Type "2"
Sleep 100ms
Type "h"
Sleep 100ms
Type "4"
Sleep 100ms
Type "p"
Sleep 100ms
Type "2"
Sleep 100ms
Type "h"
Sleep 100ms
Type "4"
Sleep 200ms
Escape
Sleep 1s

# Undo to show it all gets rolled back
Type "u"
Sleep 1s

# Redo to show it all comes back
Type "U"
Sleep 2s
